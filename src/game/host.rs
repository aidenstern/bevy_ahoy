//! Host-client wiring helpers (server + client in one process).
//!
//! **Stub** for the structural-prep round. The networking phase fills this in
//! per `notes/networking-plan.md`: spawn a `NetcodeServer` entity, spawn a
//! `Client` entity with `LinkOf { server }` (no UDP — shares process link),
//! then chain `Startup` systems `(start_server_trigger, connect_host_client)`
//! so `Started` lands before `Connect`.

use bevy::prelude::*;

pub struct HostPlugin;

impl Plugin for HostPlugin {
    fn build(&self, _app: &mut App) {
        // Host-client networking wiring lands in the followup phase.
    }
}
