//! Client-side plugin.
//!
//! **Stub** for the structural-prep round. The networking phase fills this in
//! per `notes/networking-plan.md`: `setup_local_player` (Added<Predicted> +
//! Has<Controlled> → attach bei bindings + camera + `RenderPlayer`),
//! `setup_remote_player` (Added<Interpolated> → attach mesh), and the
//! `start_client` system that triggers `Connect` on a `NetcodeClient` entity.

use bevy::prelude::*;

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, _app: &mut App) {
        // Networking wiring lands in the followup phase.
    }
}
