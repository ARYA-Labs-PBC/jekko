use super::types::Flags;

pub(super) fn parse_flags(args: &[&str]) -> Flags {
    Flags {
        single: args.contains(&"--single"),
        baseline: args.contains(&"--baseline"),
        skip_install: args.contains(&"--skip-install"),
        sourcemaps: args.contains(&"--sourcemaps"),
    }
}
