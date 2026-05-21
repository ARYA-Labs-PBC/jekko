//! HTTP+SSE bridge from the TUI prompt to a local jnoccio-fusion gateway.

use std::cmp::min;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use crate::action::{Action, RuntimeEvent};
use crate::engine::cancel::CancellationToken;

const GATEWAY_HOST: &str = "127.0.0.1";
const GATEWAY_PORT: u16 = 4317;
const GATEWAY_PATH: &str = "/v1/chat/completions";
const DEFAULT_MODEL: &str = "jnoccio/jnoccio-fusion";
const READ_TIMEOUT: Duration = Duration::from_millis(200);
const GATEWAY_READY_TIMEOUT: Duration = Duration::from_millis(8_100);
const GATEWAY_READY_RETRY_DELAY: Duration = Duration::from_millis(1_350);
const GATEWAY_READY_CANCEL_POLL: Duration = Duration::from_millis(25);

include!("chat_bridge/spawn.rs");
include!("chat_bridge/readiness.rs");
include!("chat_bridge/stream.rs");

#[cfg(test)]
include!("chat_bridge/tests.rs");
