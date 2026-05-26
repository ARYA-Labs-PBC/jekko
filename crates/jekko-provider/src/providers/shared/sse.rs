//! SSE streaming pipeline shared by every provider adapter.
//!
//! This module owns the JSON-over-SSE plumbing: the hand-rolled receiver
//! stream that decouples adapters from `tokio-stream`, the per-frame map
//! pipeline used to convert provider events into canonical
//! [`ProviderEvent`]s, and the helpers each adapter calls at the top of its
//! per-frame mapper (`preparse_sse_frame`, `parse_data_as_json`,
//! `sse_decode_all`).
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_util::stream::{Stream, StreamExt};
use reqwest::header::HeaderMap;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::error::{ProviderError, ProviderResult};
use crate::stream::{ProviderEvent, SseDecoder, SseFrame};

/// Hand-rolled receiver stream that does not depend on `tokio-stream`.
pub struct McpReceiverStream<T> {
    rx: mpsc::Receiver<T>,
}

impl<T> McpReceiverStream<T> {
    /// Construct from a tokio mpsc receiver.
    pub fn new(rx: mpsc::Receiver<T>) -> Self {
        Self { rx }
    }
}

impl<T> Stream for McpReceiverStream<T> {
    type Item = T;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

/// Maps a [`reqwest::Response`] into an async stream of [`ProviderEvent`]s by
/// running a per-frame `map_event` closure over each decoded SSE block.
///
/// The closure receives the raw frame and returns zero or more canonical
/// events. Errors short-circuit the stream.
pub fn sse_into_provider_stream<F>(
    response: reqwest::Response,
    abort: CancellationToken,
    mut map_event: F,
) -> McpReceiverStream<ProviderResult<ProviderEvent>>
where
    F: FnMut(&SseFrame) -> ProviderResult<Vec<ProviderEvent>> + Send + 'static,
{
    let (tx, rx) = mpsc::channel(128);
    let mut decoder = SseDecoder::new();
    tokio::spawn(async move {
        let mut body = response.bytes_stream();
        loop {
            tokio::select! {
                _ = abort.cancelled() => {
                    let _ = tx.send(Err(ProviderError::Aborted)).await;
                    break;
                }
                next = body.next() => {
                    match next {
                        Some(Ok(chunk)) => {
                            let frames = decoder.feed(&chunk);
                            for frame in frames {
                                match map_event(&frame) {
                                    Ok(events) => {
                                        for ev in events {
                                            if tx.send(Ok(ev)).await.is_err() {
                                                return;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let _ = tx.send(Err(e)).await;
                                        return;
                                    }
                                }
                            }
                        }
                        Some(Err(e)) => {
                            let _ = tx.send(Err(ProviderError::Transport(e.to_string()))).await;
                            return;
                        }
                        None => break,
                    }
                }
            }
        }
        let frames = decoder.flush();
        for frame in frames {
            if let Ok(events) = map_event(&frame) {
                for ev in events {
                    if tx.send(Ok(ev)).await.is_err() {
                        return;
                    }
                }
            }
        }
    });
    McpReceiverStream::new(rx)
}

/// Send a JSON POST and convert the SSE response into a provider stream.
pub async fn post_json_sse_stream<F>(
    client: &reqwest::Client,
    url: &str,
    headers: HeaderMap,
    body: &Value,
    abort: CancellationToken,
    map_event: F,
) -> ProviderResult<McpReceiverStream<ProviderResult<ProviderEvent>>>
where
    F: FnMut(&SseFrame) -> ProviderResult<Vec<ProviderEvent>> + Send + 'static,
{
    let response = client.post(url).headers(headers).json(body).send().await?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        // Explicit propagation: name the body-read failure instead of
        // silently coercing it to an empty string, so callers can tell
        // a body-less response apart from a transport read error.
        let body = match response.text().await {
            Ok(text) => text,
            Err(err) => format!("<failed to read error body: {err}>"),
        };
        return Err(ProviderError::Http { status, body });
    }

    Ok(sse_into_provider_stream(response, abort, map_event))
}

/// Buffer-mode variant of [`sse_into_provider_stream`] used by tests: takes a
/// fully-buffered byte slice and synchronously produces the event sequence.
///
/// Each chunk is fed to the SSE decoder in one shot, then the resulting frames
/// are run through `map_event`.
pub fn sse_decode_all<F>(bytes: &[u8], mut map_event: F) -> ProviderResult<Vec<ProviderEvent>>
where
    F: FnMut(&SseFrame) -> ProviderResult<Vec<ProviderEvent>>,
{
    let mut decoder = SseDecoder::new();
    let mut out = Vec::new();
    let frames = decoder.feed(&Bytes::copy_from_slice(bytes));
    for frame in frames {
        out.extend(map_event(&frame)?);
    }
    let final_frames = decoder.flush();
    for frame in final_frames {
        out.extend(map_event(&frame)?);
    }
    Ok(out)
}

/// Try to parse the SSE data payload as JSON.
pub fn parse_data_as_json(data: &str) -> ProviderResult<Value> {
    serde_json::from_str(data).map_err(|e| ProviderError::SseDecode(e.to_string()))
}

/// Outcome of pre-parsing a raw SSE frame. The decoder either short-circuits
/// with a pre-baked event vector (end-of-stream sentinel, ignored event,
/// blank data) or hands back the JSON payload ready for protocol-specific
/// mapping.
pub enum SsePreparse<'a> {
    /// Short-circuit: the frame already maps to this set of events; the
    /// caller should return them directly without further parsing.
    Resolved(Vec<ProviderEvent>),
    /// Frame data is ready to hand off to the per-provider JSON mapper.
    Payload(&'a str),
}

/// Preparse a raw SSE frame using a shared triage routine. Adapters call
/// this once at the top of their per-frame mapper to handle the universal
/// concerns (empty data, end-of-stream sentinel, ignored keepalive events)
/// in one place. `done_sentinel` is the literal data payload that signals
/// end-of-stream (e.g. `Some("[DONE]")` for OpenAI-shaped streams, `None`
/// for protocols that use a structured event). `skip_event` lets callers
/// drop protocol-specific keepalive events like Anthropic's `ping`.
pub fn preparse_sse_frame<'a>(
    frame: &'a SseFrame,
    done_sentinel: Option<&str>,
    skip_event: Option<&str>,
) -> SsePreparse<'a> {
    if let Some(done) = done_sentinel {
        if frame.data == done {
            return SsePreparse::Resolved(vec![ProviderEvent::new(
                crate::stream::ProviderEventKind::StreamEnd { stop_reason: None },
            )]);
        }
    }
    if frame.data.is_empty() {
        return SsePreparse::Resolved(Vec::new());
    }
    if let Some(name) = skip_event {
        if frame.event == name {
            return SsePreparse::Resolved(Vec::new());
        }
    }
    SsePreparse::Payload(&frame.data)
}
