use bevy::prelude::*;
use crate::kcc::input::AccumulatedInput;
use lightyear::{
  input::server::InputSystems,
  prelude::input::native::{ActionState, InputMarker},
};

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(
      FixedPreUpdate,
      copy_remote_action_state_to_accumulated_input
        .after(InputSystems::UpdateActionState),
    );
  }
}

/// Copies the [`ActionState`] of remote-simulated entities to their [`AccumulatedInput`].
///
/// Locally controlled entities should just use whatever values are present in the
/// [`AccumulatedInput`].
fn copy_remote_action_state_to_accumulated_input(
  mut targets: Query<
    (&ActionState<AccumulatedInput>, &mut AccumulatedInput),
    Without<InputMarker<AccumulatedInput>>,
  >,
) {
  for (action_state, mut input) in targets.iter_mut() {
    *input = action_state.0.clone();
  }
}
