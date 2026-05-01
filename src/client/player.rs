//! Client-side player setup: turns server-spawned `LogicalPlayer` entities
//! into something playable, and provides the `RenderPlayer` camera marker.
//!
//! - `setup_local_player`: when the predicted local player appears, attach
//!   bei bindings, the `InputMarker`s lightyear's input layer reads, and a
//!   camera + `RenderPlayer` link.
//! - `setup_remote_player`: when an interpolated remote player appears,
//!   attach a colored cylinder mesh so we can see them.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::{Hold, Press, *};
use lightyear::prelude::*;
use lightyear::prelude::input::native::{ActionState, InputMarker};

use crate::client::bindings::PlayerInput;
use crate::kcc::{CharacterLook, input::AccumulatedInput, prelude::*};
use crate::shared::player::{CollisionLayer, LogicalPlayer, PlayerId};

/// Camera-side marker that ties a render entity (with `Camera3d`) to its
/// authoritative [`LogicalPlayer`]. The `logical_entity` ref is written by
/// [`setup_local_player`] and consumed by future per-player render systems.
#[derive(Component)]
pub struct RenderPlayer {
    pub logical_entity: Entity,
}

#[derive(Component)]
struct LocalPlayerInitialized;

#[derive(Component)]
struct RemotePlayerInitialized;

pub struct ClientPlayerPlugin;

impl Plugin for ClientPlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (setup_local_player, setup_remote_player));
    }
}

fn setup_local_player(
    mut commands: Commands,
    new_local: Query<
        Entity,
        (
            With<Predicted>,
            With<LogicalPlayer>,
            Without<LocalPlayerInitialized>,
        ),
    >,
) {
    for entity in &new_local {
        info!("Setting up local controlled player on {entity:?}");

        commands.entity(entity).insert((
            LocalPlayerInitialized,
            // Physics bundle for client-side prediction. The server has its own
            // copy; replication only ferries Position/Rotation/Velocity, not the
            // controller config or the collider.
            CharacterController::default(),
            RigidBody::Kinematic,
            Collider::cylinder(0.7, 1.8),
            Mass(90.0),
            CollisionLayers::new(CollisionLayer::Player, LayerMask::ALL),
            // Input plumbing.
            PlayerInput,
            InputMarker::<AccumulatedInput>::default(),
            InputMarker::<CharacterLook>::default(),
            ActionState::<AccumulatedInput>::default(),
            ActionState::<CharacterLook>::default(),
            actions!(PlayerInput[
                (
                    Action::<Movement>::new(),
                    DeadZone::default(),
                    Bindings::spawn((Cardinal::wasd_keys(), Axial::left_stick())),
                ),
                (
                    Action::<Jump>::new(),
                    Press::default(),
                    bindings![
                        KeyCode::Space,
                        GamepadButton::South,
                        Binding::mouse_wheel(),
                    ],
                ),
                (
                    Action::<Tac>::new(),
                    Press::default(),
                    bindings![
                        KeyCode::Space,
                        GamepadButton::South,
                        Binding::mouse_wheel(),
                    ],
                ),
                (
                    Action::<Crane>::new(),
                    Press::default(),
                    bindings![
                        KeyCode::Space,
                        GamepadButton::South,
                        Binding::mouse_wheel(),
                    ],
                ),
                (
                    Action::<Mantle>::new(),
                    Hold::new(0.2),
                    bindings![KeyCode::Space, GamepadButton::South],
                ),
                (
                    Action::<Climbdown>::new(),
                    bindings![KeyCode::ControlLeft, GamepadButton::LeftTrigger2],
                ),
                (
                    Action::<Crouch>::new(),
                    bindings![KeyCode::ControlLeft, GamepadButton::LeftTrigger2],
                ),
                (
                    Action::<RotateCamera>::new(),
                    Bindings::spawn((
                        Spawn((Binding::mouse_motion(), Scale::splat(0.07))),
                        Axial::right_stick().with((Scale::splat(4.0), DeadZone::default())),
                    )),
                ),
            ]),
        ));

        commands.spawn((
            Camera3d::default(),
            CharacterControllerCameraOf::new(entity),
            RenderPlayer {
                logical_entity: entity,
            },
        ));
    }
}

fn setup_remote_player(
    mut commands: Commands,
    new_remote: Query<
        (Entity, &PlayerId),
        (
            With<Interpolated>,
            With<LogicalPlayer>,
            Without<RemotePlayerInitialized>,
        ),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, player_id) in &new_remote {
        let hue = ((player_id.0 * 137) % 360) as f32;
        commands.entity(entity).insert((
            Mesh3d(meshes.add(Cylinder::new(0.7, 1.8))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::hsl(hue, 0.7, 0.5),
                ..default()
            })),
            RemotePlayerInitialized,
        ));
    }
}
