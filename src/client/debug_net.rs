//! Periodic client-side network diagnostics. Logs every 2s so we can sanity-check
//! the local connection and which players we're seeing as predicted/interpolated.

use std::time::Duration;

use avian3d::prelude::Position;
use bevy::prelude::*;
use crate::kcc::input::AccumulatedInput;
use lightyear::prelude::input::native::{ActionState, InputMarker};
use lightyear::prelude::server::ClientOf;
use lightyear::prelude::*;

use crate::shared::player::{LogicalPlayer, PlayerId};

const TICK: Duration = Duration::from_secs(2);

pub struct ClientNetworkDebugPlugin;

impl Plugin for ClientNetworkDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, log_client_state.run_if(should_tick));
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

/// Dumps the client's view of received entities (predicted and interpolated).
fn log_client_state(
    client: Query<
        (
            Entity,
            &Client,
            Option<&LocalId>,
            Has<IsSynced<InputTimeline>>,
            Has<InputTimeline>,
            Has<PredictionManager>,
            Has<IsSynced<InterpolationTimeline>>,
            Has<InterpolationTimeline>,
        ),
        // Don't log host-server's own client-link entities — those are server-side
        // and double-counted with the server log.
        Without<ClientOf>,
    >,
    players: Query<
        (
            Entity,
            Option<&PlayerId>,
            Option<&Position>,
            Has<Predicted>,
            Has<Interpolated>,
            Option<&AccumulatedInput>,
            Option<&ActionState<AccumulatedInput>>,
            Has<InputMarker<AccumulatedInput>>,
        ),
        With<LogicalPlayer>,
    >,
) {
    let Some((
        client_entity,
        client_state,
        local_id,
        is_synced,
        has_input_tl,
        has_pred_mgr,
        is_synced_interp,
        has_interp_tl,
    )) = client.iter().next()
    else {
        return;
    };
    let player_count = players.iter().count();
    info!(
        "[client] entity={client_entity:?} state={:?} local_id={:?} players={player_count} input_synced={is_synced} input_tl={has_input_tl} pred_mgr={has_pred_mgr} interp_synced={is_synced_interp} interp_tl={has_interp_tl}",
        client_state.state,
        local_id.map(|l| l.0)
    );
    for (entity, player_id, pos, predicted, interpolated, input, action_state, has_marker) in
        &players
    {
        info!(
            "[client]   player {entity:?} id={:?} pos={:?} predicted={predicted} interpolated={interpolated} accum_move={:?} action_move={:?} input_marker={has_marker}",
            player_id.map(|p| p.0),
            pos.map(|p| p.0),
            input.and_then(|i| i.last_movement),
            action_state.and_then(|a| a.0.last_movement),
        );
    }
}
