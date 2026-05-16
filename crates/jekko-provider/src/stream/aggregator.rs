use super::events::ProviderEventKind;

/// Streaming aggregator that reassembles a multi-frame tool call from
/// [`ProviderEventKind::ToolCallStart`] + [`ProviderEventKind::ToolCallInputDelta`]+
/// [`ProviderEventKind::ToolCallEnd`] frames.
///
/// Used by tests and by adapters that emit tool calls in pieces.
#[derive(Debug, Default)]
pub struct ToolCallAggregator {
    in_flight: std::collections::HashMap<String, ToolCallBuilder>,
}

#[derive(Debug, Clone, Default)]
struct ToolCallBuilder {
    name: String,
    json: String,
}

/// Completed tool call.
#[derive(Debug, Clone, PartialEq)]
pub struct AggregatedToolCall {
    /// Tool call id.
    pub id: String,
    /// Tool name.
    pub name: String,
    /// Parsed input JSON.
    pub input: serde_json::Value,
}

impl ToolCallAggregator {
    /// Create an empty aggregator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply an event; returns the completed tool call when an end event
    /// completes the build.
    pub fn apply(&mut self, ev: &ProviderEventKind) -> Option<AggregatedToolCall> {
        match ev {
            ProviderEventKind::ToolCallStart { id, name } => {
                self.in_flight.insert(
                    id.clone(),
                    ToolCallBuilder {
                        name: name.clone(),
                        json: String::new(),
                    },
                );
                None
            }
            ProviderEventKind::ToolCallInputDelta { id, delta } => {
                if let Some(b) = self.in_flight.get_mut(id) {
                    b.json.push_str(delta);
                }
                None
            }
            ProviderEventKind::ToolCallEnd { id, name, input } => {
                self.in_flight.remove(id);
                Some(AggregatedToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    input: input.clone(),
                })
            }
            _ => None,
        }
    }

    /// Finalize a tool call that did not get an explicit end event by parsing
    /// the accumulated delta JSON. Used by adapters that report only deltas.
    pub fn finalize(&mut self, id: &str) -> Option<AggregatedToolCall> {
        let builder = self.in_flight.remove(id)?;
        let input: serde_json::Value =
            serde_json::from_str(&builder.json).unwrap_or(serde_json::Value::Null);
        Some(AggregatedToolCall {
            id: id.to_string(),
            name: builder.name,
            input,
        })
    }
}
