/// Default model used when no override is supplied via flag or env.
///
/// This MUST be the model the local jnoccio-fusion gateway exposes on
/// `/v1/chat/completions`. The gateway publishes a single visible model
/// (`jnoccio/jnoccio-fusion`) and routes internally across the keyed provider
/// pool; requesting any other id (e.g. a raw `anthropic/...` model) makes the
/// gateway answer `404 model_not_found`, which the TUI surfaces as
/// `jnoccio gateway returned non-200 status: 404`. Override with
/// [`MODEL_ENV`] only with another id the gateway actually exposes.
pub const DEFAULT_MODEL: &str = "jnoccio/jnoccio-fusion";

/// Environment variable consulted for the default model when the caller does
/// not pass one explicitly. Mirrors the rest of the jekko stack.
pub const MODEL_ENV: &str = "JEKKO_CHAT_MODEL";

/// Wrapper backend that posts to the local jnoccio-fusion gateway over the
/// SSE worker defined in [`crate::chat_bridge`].
pub struct ChatBridgeBackend {
    config: ChatBridgeConfig,
    /// T-INLINE-CLUSTER #11 / T-SANDBOX-ENF sidecar — see [`ChatBridgeRuntimePolicy`].
    policy: ChatBridgeRuntimePolicy,
}

/// Configuration knobs surfaced to callers. Wire format / gateway URL are
/// resolved inside `chat_bridge` from process env (`JEKKO_CHAT_GATEWAY_*`) so
/// this struct only carries the bits the runtime itself needs.
#[derive(Clone, Debug)]
pub struct ChatBridgeConfig {
    pub model: String,
}

impl Default for ChatBridgeConfig {
    fn default() -> Self {
        let model = match std::env::var(MODEL_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
        {
            Some(value) => value,
            None => DEFAULT_MODEL.to_string(),
        };
        Self { model }
    }
}

/// T-INLINE-CLUSTER #11 / T-SANDBOX-ENF coordination knobs. Kept as a
/// separate, non-breaking sidecar so the existing `ChatBridgeConfig` initialiser
/// (`ChatBridgeConfig { model }`) at every caller continues to compile while
/// T3-A4b owns CLI plumbing. The T-SANDBOX-ENF agent reads from here when
/// constructing the runner-level `SandboxPolicy`.
#[derive(Clone, Debug, Default)]
pub struct ChatBridgeRuntimePolicy {
    /// Raw CLI value of `--sandbox`. `None` ⇒ runner picks the platform default.
    pub sandbox_profile: Option<String>,
    /// Raw CLI value of `--ask-for-approval`. `None` ⇒ runner picks default.
    pub approval_mode: Option<String>,
    /// Raw CLI value of `--permission-mode` (Claude-compatible).
    pub permission_mode: Option<String>,
}

impl ChatBridgeBackend {
    pub fn new(config: ChatBridgeConfig) -> Self {
        Self {
            config,
            policy: ChatBridgeRuntimePolicy::default(),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(ChatBridgeConfig::default())
    }

    /// T-INLINE-CLUSTER #11: attach the sandbox/approval/permission policy
    /// derived from `--sandbox` / `--ask-for-approval` / `--permission-mode`.
    /// T-SANDBOX-ENF consumes this when it builds the runner.
    pub fn with_runtime_policy(mut self, policy: ChatBridgeRuntimePolicy) -> Self {
        self.policy = policy;
        self
    }

    /// T-INLINE-CLUSTER #11 / T-SANDBOX-ENF: expose the resolved policy.
    pub fn policy(&self) -> &ChatBridgeRuntimePolicy {
        &self.policy
    }
}

impl ChatBackend for ChatBridgeBackend {
    fn start_turn(&mut self, prompt: String, cancel: CancellationToken) -> Receiver<ChatEvent> {
        let (action_tx, action_rx) = mpsc::channel::<Action>();
        let (event_tx, event_rx) = mpsc::channel::<ChatEvent>();

        spawn_chat_request(prompt, self.config.model.clone(), action_tx, cancel);

        std::thread::Builder::new()
            .name("jekko-chat-bridge-translator".into())
            .spawn(move || {
                let mut tool_stdout: HashMap<String, String> = HashMap::new();
                for action in action_rx {
                    let events = translate_action_stateful(action, &mut tool_stdout);
                    let mut hit_terminal = false;
                    for evt in events {
                        let terminal =
                            matches!(evt, ChatEvent::TurnComplete | ChatEvent::TurnFailed(_));
                        if event_tx.send(evt).is_err() {
                            hit_terminal = true;
                            break;
                        }
                        if terminal {
                            hit_terminal = true;
                            break;
                        }
                    }
                    if hit_terminal {
                        break;
                    }
                }
            })
            .ok();

        event_rx
    }
}
