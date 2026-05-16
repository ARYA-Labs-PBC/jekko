//! `/api/v1/events` — Server-Sent Events stream of bus events.
//!
//! Subscribes to the wildcard channel on [`jekko_runtime::bus::Bus`] and
//! re-emits each [`jekko_runtime::bus::EventEnvelope`] as an SSE frame with
//! the bus `kind` as the `event:` field and the JSON payload as `data:`.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use axum::Router;
use futures_util::stream::Stream;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::state::AppState;

/// JSON payload emitted when an envelope's properties fail to serialise.
const EMPTY_JSON_OBJECT: &str = "{}";

/// Build the events router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", get(stream))
}

/// `GET /api/v1/events` — open an SSE stream.
#[utoipa::path(
    get,
    path = "/api/v1/events",
    responses((status = 200, description = "Server-sent events"))
)]
pub async fn stream(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.bus.subscribe_all();
    let bs = BroadcastStream::new(rx).filter_map(|res| match res {
        Ok(envelope) => {
            let data = match serde_json::to_string(&envelope.properties) {
                Ok(s) => s,
                Err(_) => EMPTY_JSON_OBJECT.to_string(),
            };
            let event = Event::default()
                .event(envelope.kind)
                .id(envelope.id)
                .data(data);
            Some(Ok::<Event, Infallible>(event))
        }
        Err(_lagged) => None,
    });
    Sse::new(bs).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}
