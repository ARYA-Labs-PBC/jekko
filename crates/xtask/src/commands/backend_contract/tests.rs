use std::fs;

use super::inventory::{discover_cli_commands, discover_route_groups, pascal_to_kebab};

#[test]
fn pascal_to_kebab_handles_mixed_acronyms() {
    assert_eq!(pascal_to_kebab("McpServer"), "mcp-server");
    assert_eq!(pascal_to_kebab("Db"), "db");
    assert_eq!(pascal_to_kebab("Providers"), "providers");
}

#[test]
fn discover_route_groups_ignores_mod_rs() {
    let tmp = tempfile::tempdir().unwrap();
    let routes = tmp.path().join("routes");
    fs::create_dir_all(routes.join("v2")).unwrap();
    fs::write(routes.join("mod.rs"), "// mod").unwrap();
    fs::write(routes.join("session.rs"), "// session").unwrap();
    fs::write(routes.join("v2").join("session.rs"), "// v2").unwrap();
    let groups = discover_route_groups(&routes).unwrap();
    assert_eq!(groups, vec!["session".to_string(), "v2".to_string()]);
}

#[test]
fn discover_cli_commands_parses_enum_variants() {
    let tmp = tempfile::tempdir().unwrap();
    let cli = tmp.path().join("cli.rs");
    fs::write(
        &cli,
        r#"
pub enum Command {
    Run(cmd::run::RunArgs),
    #[command(name = "mcp-server")]
    McpServer(cmd::mcp_server::McpServerArgs),
    Db(cmd::db::DbArgs),
}
"#,
    )
    .unwrap();
    let commands = discover_cli_commands(&cli).unwrap();
    assert_eq!(
        commands,
        vec![
            "db".to_string(),
            "mcp-server".to_string(),
            "run".to_string(),
        ]
    );
}
