//! Client-side gameplay plugin: turns server-spawned `LogicalPlayer` entities
//! into something playable.
//!
//! - `setup_local_player`: when the predicted local player appears, attach
//!   bei bindings, the `InputMarker`s that lightyear's input layer reads,
//!   and a camera + `RenderPlayer` link.
//! - `setup_remote_player`: when an interpolated remote player appears,
//!   attach a colored cylinder mesh so we can see them.
//!
//! Boot (spawning the lightyear `Client` entity + triggering `Connect`)
//! lives in `spawn_client`/`start_client`, wired into the relevant per-mode
//! Startup chain in `crate::game::run`.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_ahoy::{CharacterLook, input::AccumulatedInput, prelude::*};
use bevy_enhanced_input::prelude::{Hold, Press, *};
use lightyear::prelude::client::*;
use lightyear::prelude::input::native::{ActionState, InputMarker};
use lightyear::prelude::*;

use crate::game::{
    bindings::PlayerInput,
    player::{CollisionLayer, LogicalPlayer, PlayerId, RenderPlayer},
};

#[derive(Component)]
struct LocalPlayerInitialized;

#[derive(Component)]
struct RemotePlayerInitialized;

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
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

/// Spawn the `Client` entity (called at app construction time, before `Startup`).
/// Returns the entity id so the caller can chain it.
pub fn spawn_client(world: &mut World, client_id: u64, server_addr: SocketAddr) -> Entity {
    let auth = Authentication::Manual {
        server_addr,
        client_id,
        private_key: lightyear::netcode::Key::default(),
        protocol_id: 0,
    };
    let netcode =
        NetcodeClient::new(auth, NetcodeConfig::default()).expect("failed to build NetcodeClient");
    world
        .spawn((
            Name::new("Client"),
            Client::default(),
            Link::new(None),
            LocalAddr(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)),
            PeerAddr(server_addr),
            ReplicationReceiver::default(),
            // Required for client-side prediction + input messages to flow back to the server.
            // Adds InputTimelineConfig (and thus InputTimeline), LastConfirmedInput, etc.
            PredictionManager::default(),
            netcode,
            UdpIo::default(),
        ))
        .id()
}

/// Startup system: trigger `Connect` on the unique client entity.
pub fn start_client(mut commands: Commands, client: Single<Entity, With<Client>>) {
    commands.trigger(Connect {
        entity: client.into_inner(),
    });
}
