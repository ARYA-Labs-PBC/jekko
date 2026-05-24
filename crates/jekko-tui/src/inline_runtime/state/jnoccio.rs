#[derive(Debug)]
struct JnoccioBootRuntime {
    status: JnoccioBootStatus,
}

impl JnoccioBootRuntime {
    fn new(status: JnoccioBootStatus) -> Self {
        Self { status }
    }

    fn drain_updates(&mut self, rx: &mut Option<Receiver<JnoccioBootStatus>>) -> bool {
        let Some(rx) = rx.as_ref() else {
            return false;
        };
        let mut dirty = false;
        while let Ok(next) = rx.try_recv() {
            if self.status != next {
                self.status = next;
                dirty = true;
            }
        }
        dirty
    }

    fn footer_label(&self) -> Option<String> {
        match self.status {
            JnoccioBootStatus::Idle => None,
            _ => Some(format!("jnoccio {}", self.status.label())),
        }
    }

    fn status_lines(&self) -> Vec<String> {
        match self.detail() {
            Some(detail) => detail.lines().map(|line| line.to_string()).collect(),
            None => vec![format!("jnoccio: {}", self.status.label())],
        }
    }

    fn detail(&self) -> Option<String> {
        self.status.detail()
    }
}
