use bevy::prelude::*;
use bevy_ahoy::{AhoySystems, CharacterLook, input::AccumulatedInput};
use lightyear::{
  frame_interpolation::FrameInterpolationSystems,
  input::client::InputSystems,
  prelude::{
    input::native::{ActionState, InputMarker},
    is_in_rollback,
  },
};

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(
        FixedPreUpdate,
        (
          (
            copy_local_character_accumulated_input_to_action_state,
            copy_local_character_look_to_action_state,
          )
            .run_if(not(is_in_rollback))
            .in_set(InputSystems::WriteClientInputs),
          (
            copy_all_character_action_state_to_accumulated_input,
            copy_all_character_action_state_to_camera_look,
          )
            .run_if(is_in_rollback)
            .after(InputSystems::BufferClientInputs),
        ),
      )
      // Make sure lightyear's interpolation happens before we try to move the camera to the
      // character.
      .configure_sets(
        PostUpdate,
        (FrameInterpolationSystems::Interpolate, AhoySystems::UpdateCameras)
          .chain(),
      );
  }
}

/// Copies the [`AccumulatedInput`] for a local player to the [`ActionState`] for replication to
/// other clients.
fn copy_local_character_accumulated_input_to_action_state(
  mut accumulated_input: Query<
    (&AccumulatedInput, &mut ActionState<AccumulatedInput>),
    With<InputMarker<AccumulatedInput>>,
  >,
) {
  for (input, mut action_state) in accumulated_input.iter_mut() {
    action_state.0 = input.clone();
  }
}

/// Copies the [`ActionState`] of **all** players to the [`AccumulatedInput`] for simulation.
// TODO: Add a version that works for predicting remote players.
fn copy_all_character_action_state_to_accumulated_input(
  mut accumulated_input: Query<(
    &ActionState<AccumulatedInput>,
    &mut AccumulatedInput,
  )>,
) {
  for (action_state, mut accumulated_input) in accumulated_input.iter_mut() {
    *accumulated_input = action_state.0.clone();
  }
}

/// Copies the look direction of a local character to the [`ActionState`] for replication to other
/// clients.
fn copy_local_character_look_to_action_state(
  mut players: Query<
    (&CharacterLook, &mut ActionState<CharacterLook>),
    With<InputMarker<CharacterLook>>,
  >,
) {
  for (character_look, mut action_state) in players.iter_mut() {
    **action_state = character_look.clone();
  }
}

/// Copies the [`ActionState`] to look direction of the character for simulation.
fn copy_all_character_action_state_to_camera_look(
  mut character_looks: Query<(&ActionState<CharacterLook>, &mut CharacterLook)>,
) {
  for (action_state, mut character_look) in character_looks.iter_mut() {
    *character_look = action_state.0.clone();
  }
}
