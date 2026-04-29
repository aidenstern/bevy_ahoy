//! Player components and the safety-net respawn system.
//!
//! - [`LogicalPlayer`] — replicated marker on the authoritative player entity
//!   (the one that owns physics, character controller state, and bei input
//!   context). Spawned by the server in networked play; spawned locally in
//!   host-client / single-player mode.
//! - [`RenderPlayer`] — client-only marker on the camera entity that follows
//!   a [`LogicalPlayer`]. Holds the entity ref to its logical counterpart so
//!   render-side systems can pull authoritative state.
//! - [`PlayerId`] — replicated stable id (mirrors lightyear's client id), used
//!   to e.g. color-code remote players' meshes.
//! - [`CollisionLayer`] — physics layers used by the player and the world.
//! - [`respawn_below_floor`] — if a player drops below `RESPAWN_FLOOR_Y`,
//!   teleport them back to spawn.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::game::{GameState, scene::SPAWN_POINT};

pub const RESPAWN_FLOOR_Y: f32 = -50.0;

#[derive(Component, Reflect, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", reflect(Serialize, Deserialize))]
#[reflect(Component)]
pub struct LogicalPlayer;

#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", reflect(Serialize, Deserialize))]
#[reflect(Component)]
pub struct PlayerId(pub u64);

/// Camera-side marker that ties a render entity (with `Camera3d`) to its
/// authoritative [`LogicalPlayer`]. Client-only. Unused until the networking
/// phase wires `setup_local_player` (see `notes/networking-plan.md`).
#[allow(
    dead_code,
    reason = "Reserved for the networking phase; kept here so the type exists when client.rs gets fleshed out."
)]
#[derive(Component)]
pub struct RenderPlayer {
    pub logical_entity: Entity,
}

#[derive(Debug, PhysicsLayer, Default)]
pub enum CollisionLayer {
    #[default]
    Default,
    Player,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<LogicalPlayer>()
            .register_type::<PlayerId>()
            .add_systems(
                Update,
                respawn_below_floor.run_if(in_state(GameState::InGame)),
            );
    }
}

fn respawn_below_floor(
    mut players: Query<(&mut Position, &mut LinearVelocity), With<LogicalPlayer>>,
) {
    for (mut position, mut velocity) in &mut players {
        if position.y < RESPAWN_FLOOR_Y {
            position.0 = SPAWN_POINT;
            velocity.0 = Vec3::ZERO;
        }
    }
}
