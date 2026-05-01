//! Server-only game systems: authoritative spawn, respawn, network input
//! plumbing, and connection diagnostics.

pub mod setup;
pub mod debug_net;
pub mod networking;
pub mod respawn;
pub mod spawn;

pub use setup::{spawn_server, start_server};

use bevy::prelude::*;

/// Aggregator: registers all server-only plugins in one shot.
pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            networking::ServerPlugin,
            spawn::SpawnPlugin,
            respawn::RespawnPlugin,
            debug_net::ServerNetworkDebugPlugin,
        ));
    }
}
