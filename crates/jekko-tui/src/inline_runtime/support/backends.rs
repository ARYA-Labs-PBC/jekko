// ── Built-in backends ────────────────────────────────────────────────────────

/// Local echo backend — useful for the inline `jekko chat --echo` demo and
/// for tests. Streams a short reply in 8-char chunks and emits a fake
/// `Bash(echo)` tool flow so the spinner + active-tool chip + tool-card
/// rendering can be smoke-tested without a live gateway.
pub struct EchoBackend;

impl ChatBackend for EchoBackend {
    fn start_turn(&mut self, prompt: String, cancel: CancellationToken) -> Receiver<ChatEvent> {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let check_cancel = |tx: &Sender<ChatEvent>, cancel: &CancellationToken| -> bool {
                if cancel.is_cancelled() {
                    let _ = tx.send(ChatEvent::TurnFailed("cancelled".to_string()));
                    true
                } else {
                    false
                }
            };
            let tool_id = "echo-bash".to_string();
            let preamble = "echo from the offline backend — running a fake tool first.";
            for chunk in chunk_string(preamble, 8) {
                if check_cancel(&tx, &cancel) {
                    return;
                }
                if tx
                    .send(ChatEvent::AssistantDelta(chunk.to_string()))
                    .is_err()
                {
                    return;
                }
                std::thread::sleep(Duration::from_millis(50));
            }

            let tool_events = [
                ChatEvent::Tool(ToolEvent::Start {
                    id: tool_id.clone(),
                    name: "Bash".into(),
                    input: Some(format!("echo {prompt:?}")),
                }),
                ChatEvent::Tool(ToolEvent::StdoutChunk {
                    id: tool_id.clone(),
                    chunk: prompt.clone(),
                }),
                ChatEvent::Tool(ToolEvent::Complete { id: tool_id }),
            ];
            for evt in tool_events {
                if check_cancel(&tx, &cancel) {
                    return;
                }
                if tx.send(evt).is_err() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(140));
            }

            let reply = format!("\n\necho: {prompt}");
            for chunk in chunk_string(&reply, 8) {
                if check_cancel(&tx, &cancel) {
                    return;
                }
                if tx
                    .send(ChatEvent::AssistantDelta(chunk.to_string()))
                    .is_err()
                {
                    return;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            let _ = tx.send(ChatEvent::TurnComplete);
        });
        rx
    }
}
