//! Periodic server-side network diagnostics. Logs every 2s so we can sanity-check
//! how many clients are connected and what state they're in.

use std::time::Duration;

use avian3d::prelude::Position;
use bevy::prelude::*;
use crate::kcc::input::AccumulatedInput;
use lightyear::prelude::input::native::{ActionState, InputMarker};
use lightyear::prelude::server::{ClientOf, Started};
use lightyear::prelude::*;

use crate::shared::player::{LogicalPlayer, PlayerId};

const TICK: Duration = Duration::from_secs(2);

pub struct ServerNetworkDebugPlugin;

impl Plugin for ServerNetworkDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, log_server_state.run_if(should_tick));
    }
}

fn should_tick(time: Res<Time>, mut last: Local<Duration>) -> bool {
    let now = time.elapsed();
    if now.saturating_sub(*last) >= TICK {
        *last = now;
        true
    } else {
        false
    }
}

/// Dumps the server's view of replicated state.
fn log_server_state(
    server: Query<(Entity, Has<Started>), With<Server>>,
    clients_of: Query<
        (Entity, &RemoteId, Has<ReplicationSender>, Has<ReplicationReceiver>),
        With<ClientOf>,
    >,
    players: Query<
        (
            Entity,
            Option<&PlayerId>,
            Option<&Position>,
            Option<&AccumulatedInput>,
            Option<&ActionState<AccumulatedInput>>,
            Has<InputMarker<AccumulatedInput>>,
        ),
        With<LogicalPlayer>,
    >,
) {
    let server_count = server.iter().count();
    if server_count == 0 {
        return;
    }
    let started: Vec<_> = server.iter().filter(|(_, s)| *s).map(|(e, _)| e).collect();
    let clients: Vec<_> = clients_of.iter().collect();
    let player_count = players.iter().count();

    info!(
        "[server] servers={server_count} started={started:?} clients={} players={player_count}",
        clients.len()
    );
    for (entity, remote_id, has_sender, has_receiver) in &clients {
        info!(
            "[server]   client {entity:?} peer={:?} sender={has_sender} receiver={has_receiver}",
            remote_id.0
        );
    }
    for (entity, player_id, pos, input, action_state, has_input_marker) in &players {
        info!(
            "[server]   player {entity:?} id={:?} pos={:?} accum_move={:?} action_move={:?} input_marker={has_input_marker}",
            player_id.map(|p| p.0),
            pos.map(|p| p.0),
            input.and_then(|i| i.last_movement),
            action_state.and_then(|a| a.0.last_movement),
        );
    }
}
