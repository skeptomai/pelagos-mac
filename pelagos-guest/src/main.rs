//! pelagos-guest — vsock command daemon running inside the Linux VM.
//!
//! Listens on AF_VSOCK port VSOCK_PORT. For each connection, reads a JSON
//! `GuestCommand`, executes the appropriate pelagos sub-command, and streams
//! stdout/stderr back over the socket, followed by a `GuestResponse`.
//!
//! Cross-compiled to aarch64-unknown-linux-gnu and baked into the VM disk image
//! as a startup service (e.g. /etc/init.d/pelagos-guest or a systemd unit).
//!
//! # Protocol
//!
//! All messages are newline-delimited JSON.
//!
//! Request (host → guest):
//!   {"cmd":"run","image":"alpine","args":["/bin/sh","-c","echo hello"],"env":{}}
//!
//! Response stream (guest → host):
//!   {"stream":"stdout","data":"hello\n"}
//!   {"stream":"stdout","data":"..."}    (zero or more)
//!   {"exit":0}                          (terminal message)

use serde::{Deserialize, Serialize};

pub const VSOCK_PORT: u32 = 1024;

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum GuestCommand {
    /// Run a pelagos container and stream its output.
    Run {
        image: String,
        args: Vec<String>,
        #[serde(default)]
        env: std::collections::HashMap<String, String>,
    },
    /// Ping — returns {"pong": true}. Used to check readiness.
    Ping,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuestResponse {
    /// A chunk of output from the container.
    Stream { stream: StreamKind, data: String },
    /// Container exited. Terminal message.
    Exit { exit: i32 },
    /// Response to Ping.
    Pong { pong: bool },
    /// An error before the container started.
    Error { error: String },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamKind {
    Stdout,
    Stderr,
}

fn main() {
    env_logger::init();
    todo!("implement vsock listener and command dispatch — see docs/DESIGN.md §Guest Daemon");
}
