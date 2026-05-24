/// Trait so `apply_to_command` works with both `std::process::Command` and
/// `tokio::process::Command` without dragging in conditional generics.
/// Methods are prefixed with `policy_` to avoid clashing with the underlying
/// inherent methods (which `apply_to_command` *also* needs to call via these
/// trait impls).
pub trait CommandLike {
    fn policy_current_dir(&mut self, dir: &std::path::Path);
    fn policy_env_clear(&mut self);
    fn policy_env(&mut self, key: &str, value: &str);
}

impl CommandLike for std::process::Command {
    fn policy_current_dir(&mut self, dir: &std::path::Path) {
        self.current_dir(dir);
    }
    fn policy_env_clear(&mut self) {
        self.env_clear();
    }
    fn policy_env(&mut self, key: &str, value: &str) {
        self.env(key, value);
    }
}

impl CommandLike for tokio::process::Command {
    fn policy_current_dir(&mut self, dir: &std::path::Path) {
        self.current_dir(dir);
    }
    fn policy_env_clear(&mut self) {
        self.env_clear();
    }
    fn policy_env(&mut self, key: &str, value: &str) {
        self.env(key, value);
    }
}
