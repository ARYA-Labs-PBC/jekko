use std::fs;

use tempfile::TempDir;

use super::*;

#[test]
fn catalog_new_contains_full_builtin_set() {
    let catalog = SlashCatalog::new();
    let ids: Vec<&str> = catalog.all().map(|c| c.id()).collect();
    for must_have in [
        "help",
        "clear",
        "compact",
        "model",
        "config",
        "mcp",
        "resume",
        "permissions",
        "status",
        "cost",
        "init",
        "review",
        "login",
        "logout",
        "agents",
        "memory",
        "copy",
        "export",
        "ps",
        "stop",
        "cd",
        "theme",
        "doctor",
        "quit",
        "new",
        "bug",
        "rename",
        "add-dir",
        "vim",
        "echo",
        "panels",
    ] {
        assert!(
            ids.contains(&must_have),
            "missing built-in /{must_have} in catalog"
        );
    }
    assert_eq!(catalog.len(), BUILTIN_SLASH.len());
}

#[test]
fn filter_by_prefix_returns_only_matches() {
    let catalog = SlashCatalog::new();
    let hits: Vec<&str> = catalog.filter("co").iter().map(|c| c.id()).collect();
    assert!(hits.contains(&"config"));
    assert!(hits.contains(&"cost"));
    assert!(hits.contains(&"copy"));
    assert!(hits.iter().all(|id| id.starts_with("co")));
}

#[test]
fn filter_empty_query_returns_everything() {
    let catalog = SlashCatalog::new();
    assert_eq!(catalog.filter("").len(), catalog.len());
}

#[test]
fn find_returns_matching_command() {
    let catalog = SlashCatalog::new();
    let cmd = catalog.find("help").expect("help exists");
    assert_eq!(cmd.id(), "help");
    assert!(!cmd.is_user_defined());
}

#[test]
fn action_for_maps_builtin_ids() {
    let catalog = SlashCatalog::new();
    assert_eq!(catalog.action_for("help"), SlashAction::Help);
    assert_eq!(catalog.action_for("quit"), SlashAction::Quit);
    assert_eq!(catalog.action_for("compact"), SlashAction::Compact);
    assert_eq!(catalog.action_for("model"), SlashAction::Model);
    assert_eq!(catalog.action_for("add-dir"), SlashAction::AddDir);
    assert_eq!(catalog.action_for("zzz"), SlashAction::Unknown);
}

#[test]
fn user_commands_load_from_workspace_dir() {
    let tmp = TempDir::new().unwrap();
    let cmd_dir = tmp.path().join(".jankurai").join("commands");
    fs::create_dir_all(&cmd_dir).unwrap();
    fs::write(
        cmd_dir.join("deploy.md"),
        "---\ndescription: Ship the current branch\n---\nrun deploy.sh\n",
    )
    .unwrap();

    let catalog = SlashCatalog::new().with_user_commands(tmp.path());
    let deploy = catalog.find("deploy").expect("deploy loaded");
    assert!(deploy.is_user_defined());
    assert_eq!(deploy.description(), "Ship the current branch");
    assert_eq!(deploy.body(), Some("run deploy.sh\n"));
}

#[test]
fn user_commands_cannot_shadow_builtins() {
    let tmp = TempDir::new().unwrap();
    let cmd_dir = tmp.path().join(".jankurai").join("commands");
    fs::create_dir_all(&cmd_dir).unwrap();
    fs::write(
        cmd_dir.join("quit.md"),
        "---\ndescription: hijacked\n---\nbody\n",
    )
    .unwrap();

    let catalog = SlashCatalog::new().with_user_commands(tmp.path());
    let quit = catalog.find("quit").expect("quit still present");
    assert!(!quit.is_user_defined(), "builtin must win on collision");
    assert_eq!(quit.description(), "exit the inline chat surface");
}

#[test]
fn action_for_user_defined_carries_body() {
    let tmp = TempDir::new().unwrap();
    let cmd_dir = tmp.path().join(".jankurai").join("commands");
    fs::create_dir_all(&cmd_dir).unwrap();
    fs::write(
        cmd_dir.join("ship.md"),
        "---\ndescription: ship it\n---\nplease deploy now\n",
    )
    .unwrap();

    let catalog = SlashCatalog::new().with_user_commands(tmp.path());
    match catalog.action_for("ship") {
        SlashAction::UserDefined { id, body } => {
            assert_eq!(id, "ship");
            assert_eq!(body, "please deploy now\n");
        }
        other => panic!("expected UserDefined, got {other:?}"),
    }
}

#[test]
fn with_user_commands_no_dir_is_noop() {
    let tmp = TempDir::new().unwrap();
    let catalog = SlashCatalog::new().with_user_commands(tmp.path());
    assert_eq!(catalog.len(), BUILTIN_SLASH.len());
}

#[test]
fn catalog_includes_jankurai() {
    assert!(
        BUILTIN_SLASH.iter().any(|c| c.id() == "jankurai"),
        "jekko-native /jankurai missing from BUILTIN_SLASH"
    );
}

#[test]
fn catalog_includes_phase_m_natives() {
    for must_have in [
        "jankurai",
        "audit",
        "audit-check",
        "jankurai-status",
        "score",
        "sandbox",
        "keys",
        "daemon",
        "plugin",
        "features",
        "session",
        "fork",
        "attach",
        "serve",
        "run",
        "completion",
        "providers",
        "models",
        "acp",
        "mcp-server",
        "debug",
        "import",
        "stats",
        "pr",
        "github",
        "db",
        "upgrade",
        "uninstall",
    ] {
        assert!(
            BUILTIN_SLASH.iter().any(|c| c.id() == must_have),
            "missing jekko-native /{must_have} in BUILTIN_SLASH"
        );
    }
}

#[test]
fn slash_action_for_jankurai_id() {
    assert_eq!(SlashAction::for_id("jankurai"), SlashAction::Jankurai);
    assert_eq!(SlashAction::for_id("audit"), SlashAction::Audit);
    assert_eq!(SlashAction::for_id("audit-check"), SlashAction::AuditCheck);
    assert_eq!(
        SlashAction::for_id("jankurai-status"),
        SlashAction::JankuraiStatus
    );
    assert_eq!(SlashAction::for_id("score"), SlashAction::Score);
    assert_eq!(SlashAction::for_id("mcp-server"), SlashAction::McpServer);
    assert_eq!(SlashAction::for_id("uninstall"), SlashAction::Uninstall);
}

#[test]
fn catalog_exposes_tier_2_submenus() {
    let catalog = SlashCatalog::new();
    for parent in [
        "keys",
        "daemon",
        "plugin",
        "features",
        "session",
        "providers",
        "mcp",
        "agents",
    ] {
        let submenu = catalog
            .submenu_for(parent)
            .unwrap_or_else(|| panic!("missing submenu for /{parent}"));
        assert_eq!(submenu.parent_id, parent);
        assert!(
            !submenu.items.is_empty(),
            "submenu for /{parent} should have child rows"
        );
        assert!(
            submenu.shell_base.starts_with("jekko "),
            "submenu for /{parent} should expose shell fallback base"
        );
    }
}

#[test]
fn catalog_submenu_rows_match_expected_cli_shapes() {
    let catalog = SlashCatalog::new();
    let keys = catalog.submenu_for("keys").expect("keys submenu");
    assert_eq!(keys.item(0).map(|item| item.id), Some("set <PROVIDER>"));
    assert!(keys.items.iter().any(|item| item.id == "status"));

    let providers = catalog.submenu_for("providers").expect("providers submenu");
    assert!(providers.items.iter().any(|item| item.id == "show <NAME>"));
    assert!(providers
        .items
        .iter()
        .any(|item| item.id == "logout <NAME>"));

    let mcp = catalog.submenu_for("mcp").expect("mcp submenu");
    assert!(mcp
        .items
        .iter()
        .any(|item| item.id == "attach <NAME> <TARGET>"));
}
