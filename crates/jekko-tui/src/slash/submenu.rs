/// One row inside a tier-2 slash submenu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlashSubcommand {
    pub id: &'static str,
    pub description: &'static str,
}

impl SlashSubcommand {
    pub const fn new(id: &'static str, description: &'static str) -> Self {
        Self { id, description }
    }
}

/// Static submenu definition for slash commands that map to CLI namespaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlashSubmenu {
    pub parent_id: &'static str,
    pub shell_base: &'static str,
    pub items: &'static [SlashSubcommand],
}

impl SlashSubmenu {
    pub fn item(&self, index: usize) -> Option<&'static SlashSubcommand> {
        self.items.get(index)
    }
}

const KEYS_SUBMENU: &[SlashSubcommand] = &[
    SlashSubcommand::new("set <PROVIDER>", "set an API key"),
    SlashSubcommand::new("list", "list configured keys"),
    SlashSubcommand::new("delete <PROVIDER>", "remove a key"),
    SlashSubcommand::new("path", "show keystore path"),
    SlashSubcommand::new("init", "initialize the keystore"),
    SlashSubcommand::new("status", "show auth status"),
    SlashSubcommand::new("users", "list known users"),
];

const DAEMON_SUBMENU: &[SlashSubcommand] = &[
    SlashSubcommand::new("start", "start the daemon"),
    SlashSubcommand::new("stop", "stop the daemon"),
    SlashSubcommand::new("status", "show daemon status"),
    SlashSubcommand::new("logs", "tail daemon logs"),
];

const PLUGIN_SUBMENU: &[SlashSubcommand] = &[
    SlashSubcommand::new("list", "list installed plugins"),
    SlashSubcommand::new("install <NAME>", "install a plugin"),
    SlashSubcommand::new("enable <NAME>", "enable a plugin"),
    SlashSubcommand::new("disable <NAME>", "disable a plugin"),
];

const FEATURES_SUBMENU: &[SlashSubcommand] = &[
    SlashSubcommand::new("list", "list feature flags"),
    SlashSubcommand::new("enable <NAME>", "enable a feature flag"),
    SlashSubcommand::new("disable <NAME>", "disable a feature flag"),
];

const SESSION_SUBMENU: &[SlashSubcommand] = &[
    SlashSubcommand::new("list", "list sessions"),
    SlashSubcommand::new("show <ID>", "show a session"),
    SlashSubcommand::new("delete <ID>", "delete a session"),
    SlashSubcommand::new("export <ID>", "export a session"),
    SlashSubcommand::new("import <PATH>", "import a session"),
];

const PROVIDERS_SUBMENU: &[SlashSubcommand] = &[
    SlashSubcommand::new("list", "list providers"),
    SlashSubcommand::new("show <NAME>", "show provider details"),
    SlashSubcommand::new("enable <NAME>", "enable a provider"),
    SlashSubcommand::new("disable <NAME>", "disable a provider"),
    SlashSubcommand::new("login <NAME>", "login to a provider"),
    SlashSubcommand::new("logout <NAME>", "logout of a provider"),
];

const MCP_SUBMENU: &[SlashSubcommand] = &[
    SlashSubcommand::new("list", "list MCP servers"),
    SlashSubcommand::new("attach <NAME> <TARGET>", "attach an MCP server"),
    SlashSubcommand::new("detach <NAME>", "detach an MCP server"),
    SlashSubcommand::new("status <NAME>", "show MCP status"),
];

const AGENTS_SUBMENU: &[SlashSubcommand] = &[
    SlashSubcommand::new("create", "create an agent definition"),
    SlashSubcommand::new("list", "list agent definitions"),
    SlashSubcommand::new("show <NAME>", "show one agent definition"),
    SlashSubcommand::new("remove <NAME>", "remove an agent definition"),
];

pub const SLASH_SUBMENUS: &[SlashSubmenu] = &[
    SlashSubmenu {
        parent_id: "keys",
        shell_base: "jekko keys",
        items: KEYS_SUBMENU,
    },
    SlashSubmenu {
        parent_id: "daemon",
        shell_base: "jekko daemon",
        items: DAEMON_SUBMENU,
    },
    SlashSubmenu {
        parent_id: "plugin",
        shell_base: "jekko plugin",
        items: PLUGIN_SUBMENU,
    },
    SlashSubmenu {
        parent_id: "features",
        shell_base: "jekko features",
        items: FEATURES_SUBMENU,
    },
    SlashSubmenu {
        parent_id: "session",
        shell_base: "jekko session",
        items: SESSION_SUBMENU,
    },
    SlashSubmenu {
        parent_id: "providers",
        shell_base: "jekko providers",
        items: PROVIDERS_SUBMENU,
    },
    SlashSubmenu {
        parent_id: "mcp",
        shell_base: "jekko mcp",
        items: MCP_SUBMENU,
    },
    SlashSubmenu {
        parent_id: "agents",
        shell_base: "jekko agent",
        items: AGENTS_SUBMENU,
    },
];
