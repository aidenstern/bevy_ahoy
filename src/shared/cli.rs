//! Command-line argument parsing.
//!
//! No subcommand → host-client mode (server + client in one process; default
//! when both `client` and `server` features are enabled).
//!
//! Subcommands are gated by feature so a binary compiled with only `client` or
//! only `server` exposes only the modes it can actually run.

use std::net::SocketAddr;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug, Default)]
#[command(name = "bevy_game")]
pub struct Cli {
    #[command(subcommand)]
    pub mode: Option<Mode>,

    /// Smoke-test: exit after this many frames.
    #[arg(long)]
    pub frames: Option<u32>,
}

#[derive(Subcommand, Debug, Clone, Copy)]
pub enum Mode {
    /// Run as dedicated server (headless).
    #[cfg(feature = "server")]
    Server {
        #[arg(long, default_value = "0.0.0.0:5000")]
        bind_addr: SocketAddr,
    },
    /// Run as client connecting to a server.
    #[cfg(feature = "client")]
    Client {
        #[arg(long, default_value_t = 1)]
        client_id: u64,
        #[arg(long, default_value = "127.0.0.1:5000")]
        server_addr: SocketAddr,
    },
}
