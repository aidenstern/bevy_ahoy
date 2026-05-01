//! Server-side player lifecycle: per-link replication setup and authoritative
//! `LogicalPlayer` spawn on connect.

use std::time::Duration;

use avian3d::prelude::*;
use bevy::prelude::*;
use lightyear::prelude::input::native::ActionState;
use lightyear::prelude::server::*;
use lightyear::prelude::*;

use crate::kcc::{CharacterLook, input::AccumulatedInput, prelude::*};
use crate::shared::{
    player::{CollisionLayer, LogicalPlayer, PlayerId},
    scene::SPAWN_POINT,
};

const SEND_INTERVAL: Duration = Duration::from_millis(100);

pub struct SpawnPlugin;

impl Plugin for SpawnPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(handle_new_link)
            .add_observer(handle_connected);
    }
}

/// Called when a new client link is established. The server-side `LinkOf`
/// entity needs **both** a sender (to push state to the client) and a
/// receiver (to read replicated `ActionState<…>` input from the client).
fn handle_new_link(trigger: On<Add, LinkOf>, mut commands: Commands) {
    commands.entity(trigger.entity).insert((
        ReplicationSender::new(SEND_INTERVAL, SendUpdatesMode::SinceLastAck, false),
        ReplicationReceiver::default(),
        Name::from("Client"),
    ));
}

/// Called once the client finishes connecting. Spawns the `LogicalPlayer`
/// for this client with prediction targeted at the owning client and
/// interpolation at everyone else.
fn handle_connected(
    trigger: On<Add, Connected>,
    query: Query<&RemoteId, With<ClientOf>>,
    mut commands: Commands,
) {
    let Ok(remote_id) = query.get(trigger.entity) else {
        return;
    };
    let peer_id = remote_id.0;
    let stable_id = peer_id_to_u64(peer_id);
    info!(
        "Client connected with peer-id {peer_id:?}, spawning LogicalPlayer (stable_id={stable_id})"
    );

    commands.spawn((
        // Identity / state.
        (
            LogicalPlayer,
            PlayerId(stable_id),
            Position(SPAWN_POINT),
            Rotation::default(),
            CharacterLook::default(),
        ),
        // ActionStates: lightyear updates these on the server-side entity from
        // the replicated client input. `crate::server::networking` then copies
        // ActionState<AccumulatedInput> -> AccumulatedInput each FixedPreUpdate
        // so the kcc reads the right input. Without these explicit defaults the
        // entity has no ActionState component and the copy never runs.
        (
            ActionState::<AccumulatedInput>::default(),
            ActionState::<CharacterLook>::default(),
        ),
        // Physics.
        (
            CharacterController::default(),
            RigidBody::Kinematic,
            Collider::cylinder(0.7, 1.8),
            Mass(90.0),
            CollisionLayers::new(CollisionLayer::Player, LayerMask::ALL),
        ),
        // Replication.
        (
            Replicate::to_clients(NetworkTarget::All),
            PredictionTarget::to_clients(NetworkTarget::Single(peer_id)),
            InterpolationTarget::to_clients(NetworkTarget::AllExceptSingle(peer_id)),
            ControlledBy {
                owner: trigger.entity,
                lifetime: Default::default(),
            },
        ),
    ));
}

/// Extract a u64 from a `PeerId` for use as a stable `PlayerId.0`.
/// All practical variants in this game (Netcode for real clients,
/// Local for the host-client) carry a u64 directly; other variants
/// fall back to 0.
fn peer_id_to_u64(peer: PeerId) -> u64 {
    match peer {
        PeerId::Netcode(id) | PeerId::Local(id) | PeerId::Steam(id) | PeerId::Entity(id) => id,
        PeerId::Raw(_) | PeerId::Server => 0,
    }
}
