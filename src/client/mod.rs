//! Client-only game systems: local-player setup, render visuals, debug HUD,
//! cursor capture, and the network input replication plumbing.

pub mod bindings;
pub mod setup;
pub mod cursor;
pub mod debug;
pub mod debug_net;
pub mod networking;
pub mod player;
pub mod visuals;

pub use setup::{spawn_client, start_client};

use bevy::prelude::*;
use bevy_enhanced_input::EnhancedInputPlugin;
use bevy_framepace::FramepacePlugin;
use bevy_mod_mipmap_generator::MipmapGeneratorPlugin;

/// Aggregator: registers all client-only plugins in one shot.
pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            EnhancedInputPlugin,
            FramepacePlugin,
            MipmapGeneratorPlugin,
            networking::ClientPlugin,
            bindings::BindingsPlugin,
            cursor::CursorPlugin,
            debug::DebugPlugin,
            debug_net::ClientNetworkDebugPlugin,
            player::ClientPlayerPlugin,
            visuals::VisualsPlugin,
        ));
    }
}
