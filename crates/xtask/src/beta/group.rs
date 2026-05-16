use std::env;

pub(super) struct Group {
    enabled: bool,
}

impl Group {
    pub(super) fn new(title: String) -> Self {
        if env::var("GITHUB_ACTIONS").ok().as_deref() != Some("true") {
            println!("{title}");
            return Self { enabled: false };
        }

        println!("::group::{title}");
        Self { enabled: true }
    }
}

impl Drop for Group {
    fn drop(&mut self) {
        if self.enabled {
            println!("::endgroup::");
        }
    }
}
