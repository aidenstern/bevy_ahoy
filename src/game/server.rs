//! Server-side plugin.
//!
//! **Stub** for the structural-prep round. The networking phase fills this in
//! per `notes/networking-plan.md`: `handle_new_link`/`handle_connected`
//! observers, lightyear `Replicate`/`PredictionTarget`/`InterpolationTarget`
//! on spawned `LogicalPlayer` entities, respawn-below-floor on the
//! authoritative position, and the `start_dedicated_server` system that
//! triggers `Start` on a `NetcodeServer` entity.

use bevy::prelude::*;

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, _app: &mut App) {
        // Networking wiring lands in the followup phase.
    }
}
