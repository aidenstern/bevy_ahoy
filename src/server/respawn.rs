//! Authoritative below-floor respawn. Server-only — only the server should
//! mutate the logical player's `Position`, since the client predicts that
//! position and the server replicates the corrected value back.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::GameState;
use crate::shared::{player::LogicalPlayer, scene::SPAWN_POINT};

pub const RESPAWN_FLOOR_Y: f32 = -50.0;

pub struct RespawnPlugin;

impl Plugin for RespawnPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
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
