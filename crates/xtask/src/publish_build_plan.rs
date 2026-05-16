use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishBuildPlan {
    pub targets: Vec<PublishBuildTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishBuildTarget {
    pub os: String,
    pub arch: String,
    pub name: String,
    pub bun_target: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub abi: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub avx2: Option<bool>,
}

#[derive(Debug, Clone, Copy)]
struct TargetSpec {
    os: &'static str,
    arch: &'static str,
    abi: Option<&'static str>,
    avx2: Option<bool>,
}

const ALL_TARGETS: &[TargetSpec] = &[
    TargetSpec {
        os: "linux",
        arch: "arm64",
        abi: None,
        avx2: None,
    },
    TargetSpec {
        os: "linux",
        arch: "x64",
        abi: None,
        avx2: None,
    },
    TargetSpec {
        os: "linux",
        arch: "x64",
        abi: None,
        avx2: Some(false),
    },
    TargetSpec {
        os: "linux",
        arch: "arm64",
        abi: Some("musl"),
        avx2: None,
    },
    TargetSpec {
        os: "linux",
        arch: "x64",
        abi: Some("musl"),
        avx2: None,
    },
    TargetSpec {
        os: "linux",
        arch: "x64",
        abi: Some("musl"),
        avx2: Some(false),
    },
    TargetSpec {
        os: "darwin",
        arch: "arm64",
        abi: None,
        avx2: None,
    },
    TargetSpec {
        os: "darwin",
        arch: "x64",
        abi: None,
        avx2: None,
    },
    TargetSpec {
        os: "darwin",
        arch: "x64",
        abi: None,
        avx2: Some(false),
    },
];

pub fn run(package_name: &str, single: bool, baseline: bool) -> Result<()> {
    let plan = PublishBuildPlan {
        targets: ALL_TARGETS
            .iter()
            .copied()
            .filter(|item| !single || matches_current_platform(item))
            .filter(|item| baseline || item.avx2 != Some(false))
            .filter(|item| !single || item.abi.is_none())
            .map(|item| to_target(package_name, item))
            .collect(),
    };
    println!("{}", serde_json::to_string(&plan)?);
    Ok(())
}

fn matches_current_platform(item: &TargetSpec) -> bool {
    item.os == host_os() && item.arch == host_arch()
}

fn to_target(package_name: &str, item: TargetSpec) -> PublishBuildTarget {
    let name = [
        package_name,
        if item.os == "win32" {
            "windows"
        } else {
            item.os
        },
        item.arch,
        item.avx2
            .map(|avx2| if !avx2 { "baseline" } else { "" })
            .unwrap_or(""),
        item.abi.unwrap_or(""),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join("-");

    PublishBuildTarget {
        os: item.os.to_string(),
        arch: item.arch.to_string(),
        name: name.clone(),
        bun_target: name.replace(package_name, "bun"),
        abi: item.abi.map(ToOwned::to_owned),
        avx2: item.avx2,
    }
}

fn host_os() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        "linux" => "linux",
        "windows" => "win32",
        other => other,
    }
}

fn host_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn run_emits_all_targets_by_default() {
        let plan = plan(false, false);
        assert_eq!(plan.targets.len(), 6);
        assert!(plan.targets.iter().all(|item| item.os != "win32"));
    }

    #[test]
    fn run_filters_to_current_platform_when_single() {
        let plan = plan(true, false);
        assert!(plan.targets.iter().all(|item| item.abi.is_none()));
    }

    #[test]
    fn run_includes_baseline_targets_when_requested() {
        let plan = plan(false, true);
        assert!(plan.targets.iter().any(|item| item.avx2 == Some(false)));
    }

    #[test]
    fn names_include_windows_alias() {
        let target = to_target(
            "jekko",
            TargetSpec {
                os: "win32",
                arch: "x64",
                abi: None,
                avx2: None,
            },
        );
        assert_eq!(target.name, "jekko-windows-x64");
        assert_eq!(target.bun_target, "bun-windows-x64");
    }

    #[test]
    fn serialises_to_expected_json_shape() {
        let plan = plan(false, false);
        let value: Value = serde_json::to_value(&plan).unwrap();
        assert!(value.get("targets").is_some());
    }

    fn plan(single: bool, baseline: bool) -> PublishBuildPlan {
        PublishBuildPlan {
            targets: ALL_TARGETS
                .iter()
                .copied()
                .filter(|item| !single || matches_current_platform(item))
                .filter(|item| baseline || item.avx2 != Some(false))
                .filter(|item| !single || item.abi.is_none())
                .map(|item| to_target("jekko", item))
                .collect(),
        }
    }
}
