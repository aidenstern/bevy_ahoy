//! Debug overlay (toggleable text HUD) and player-reset key (R).
//!
//! Lives in its own bei [`DebugInput`] context so the bindings don't compete
//! with [`super::bindings::PlayerInput`].

use avian3d::prelude::*;
use bevy::{platform::collections::HashSet, prelude::*};
use bevy_ahoy::{CharacterControllerOutput, prelude::*};
use bevy_enhanced_input::prelude::{Release, *};

use crate::game::{GameState, player::LogicalPlayer, scene::SPAWN_POINT};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_input_context::<DebugInput>()
            .add_systems(OnEnter(GameState::InGame), setup_ui)
            .add_systems(
                Update,
                update_debug_text.run_if(in_state(GameState::InGame)),
            )
            .add_observer(reset_player)
            .add_observer(toggle_debug);
    }
}

#[derive(Component)]
pub struct DebugText;

#[derive(Component, Default)]
pub struct DebugInput;

#[derive(Debug, InputAction)]
#[action_output(bool)]
pub struct Reset;

#[derive(Debug, InputAction)]
#[action_output(bool)]
pub struct ToggleDebug;

fn setup_ui(mut commands: Commands) {
    commands.spawn((
        Node::default(),
        Text::default(),
        Visibility::Hidden,
        DebugText,
    ));
    commands.spawn((
        Node {
            justify_self: JustifySelf::End,
            justify_content: JustifyContent::End,
            align_self: AlignSelf::End,
            padding: UiRect::all(px(10.0)),
            ..default()
        },
        Text::new(
            "Controls:\nWASD: move\nSpace: jump\nCtrl: crouch\nEsc: free mouse\nR: reset position\nBacktick: Toggle Debug Menu",
        ),
    ));
    commands.spawn((
        DebugInput,
        actions!(DebugInput[
            (
                Action::<Reset>::new(),
                bindings![KeyCode::KeyR, GamepadButton::Select],
                Release::default(),
            ),
            (
                Action::<ToggleDebug>::new(),
                bindings![KeyCode::Backquote, GamepadButton::Start],
                Release::default(),
            ),
        ]),
    ));
}

fn reset_player(
    _fire: On<Fire<Reset>>,
    mut player: Query<(&mut Position, &mut LinearVelocity), With<LogicalPlayer>>,
    mut camera: Query<&mut Transform, (With<Camera3d>, Without<LogicalPlayer>)>,
) {
    let Ok((mut position, mut velocity)) = player.single_mut() else {
        return;
    };
    velocity.0 = Vec3::ZERO;
    position.0 = SPAWN_POINT;
    let Ok(mut camera_transform) = camera.single_mut() else {
        return;
    };
    camera_transform.rotation = Quat::IDENTITY;
}

fn toggle_debug(
    _fire: On<Fire<ToggleDebug>>,
    mut visibility: Single<&mut Visibility, With<DebugText>>,
) {
    **visibility = match **visibility {
        Visibility::Hidden => Visibility::Inherited,
        _ => Visibility::Hidden,
    };
}

fn update_debug_text(
    mut text: Single<&mut Text, With<DebugText>>,
    kcc: Single<
        (
            &CharacterControllerState,
            &CharacterControllerOutput,
            &LinearVelocity,
            &CollidingEntities,
            &ColliderAabb,
        ),
        (With<CharacterController>, With<CharacterControllerCamera>),
    >,
    camera: Single<&Transform, With<Camera>>,
    names: Query<NameOrEntity>,
) {
    let (state, output, velocity, colliding_entities, aabb) = kcc.into_inner();
    let velocity = **velocity;
    let speed = velocity.length();
    let horizontal_speed = velocity.xz().length();
    let camera_position = camera.translation;
    let collisions = names
        .iter_many(
            output
                .touching_entities
                .iter()
                .map(|e| e.entity)
                .collect::<HashSet<_>>(),
        )
        .map(|name| {
            name.name
                .map(|n| format!("{} ({})", name.entity, n))
                .unwrap_or_else(|| format!("{}", name.entity))
        })
        .collect::<Vec<_>>();
    let real_collisions = names
        .iter_many(colliding_entities.iter())
        .map(|name| {
            name.name
                .map(|n| format!("{} ({})", name.entity, n))
                .unwrap_or_else(|| format!("{}", name.entity))
        })
        .collect::<Vec<_>>();
    let ground = state
        .grounded
        .and_then(|ground| names.get(ground.entity).ok())
        .map(|name| {
            name.name
                .map(|n| format!("{} ({})", name.entity, n))
                .unwrap_or(format!("{}", name.entity))
        });
    text.0 = format!(
        "Speed: {speed:.3}\nHorizontal Speed: {horizontal_speed:.3}\nVelocity: [{:.3}, {:.3}, {:.3}]\nCamera Position: [{:.3}, {:.3}, {:.3}]\nCollider Aabb:\n  min:[{:.3}, {:.3}, {:.3}]\n  max:[{:.3}, {:.3}, {:.3}]\nReal Collisions: {:#?}\nCollisions: {:#?}\nGround: {:?}",
        velocity.x,
        velocity.y,
        velocity.z,
        camera_position.x,
        camera_position.y,
        camera_position.z,
        aabb.min.x,
        aabb.min.y,
        aabb.min.z,
        aabb.max.x,
        aabb.max.y,
        aabb.max.z,
        real_collisions,
        collisions,
        ground,
    );
}
