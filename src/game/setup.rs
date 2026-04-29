//! Single-player scene setup: spawns the player, the camera that follows it,
//! and a directional light. Will be replaced by per-mode (server/client/host)
//! setup in the networking phase.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_ahoy::prelude::*;

use crate::game::{
    GameState,
    bindings::PlayerInput,
    player::{CollisionLayer, LogicalPlayer},
    scene::SPAWN_POINT,
};

pub struct SetupPlugin;

impl Plugin for SetupPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), setup);
    }
}

fn setup(mut commands: Commands) {
    let player = commands
        .spawn((
            LogicalPlayer,
            Transform::from_translation(SPAWN_POINT),
            CollisionLayers::new(CollisionLayer::Player, LayerMask::ALL),
            PlayerInput,
            CharacterController::default(),
            RigidBody::Kinematic,
            Collider::cylinder(0.7, 1.8),
            Mass(90.0),
        ))
        .id();

    commands.spawn((
        Camera3d::default(),
        CharacterControllerCameraOf::new(player),
    ));

    commands.spawn((
        Transform::from_xyz(0.0, 1.0, 0.0).looking_at(vec3(1.0, -2.0, -2.0), Vec3::Y),
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
    ));
}
