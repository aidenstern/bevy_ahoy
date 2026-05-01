use bevy::prelude::*;
use crate::kcc::{
  CharacterControllerState, CharacterLook, input::AccumulatedInput,
};
use lightyear::{
  input::native::plugin::InputPlugin,
  prelude::{
    AppComponentExt, PredictionRegistrationExt,
    input::native::{ActionState, InputMarker},
  },
};

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins((
        InputPlugin::<AccumulatedInput>::default(),
        InputPlugin::<CharacterLook>::default(),
      ))
      .add_systems(
        FixedUpdate,
        copy_remote_character_action_state_to_character_look,
      );

    app.register_component::<CharacterLook>();

    app
      .register_component::<CharacterControllerState>()
      .add_component_map_entities()
      .add_prediction()
      .add_should_rollback(character_controller_state_should_rollback);
  }
}

fn copy_remote_character_action_state_to_character_look(
  mut character_looks: Query<
    (&ActionState<CharacterLook>, &mut CharacterLook),
    Without<InputMarker<CharacterLook>>,
  >,
) {
  for (action_state, mut character_look) in character_looks.iter_mut() {
    *character_look = action_state.0.clone();
  }
}

fn character_controller_state_should_rollback(
  this: &CharacterControllerState,
  that: &CharacterControllerState,
) -> bool {
  // TODO: I think we could just have an unused const here to ensure that we've considered every
  // field here, but Stopwatch has no way to create an instance const, so we need to wait until
  // CharacterControllerState doesn't use Stopwatch.
  // const _PREVENT_SKEW_STATE: CharacterControllerState =
  //   CharacterControllerState {
  //     orientation: Quat::IDENTITY,
  //     platform_velocity: Vec3::ZERO,
  //     platform_angular_velocity: Vec3::ZERO,
  //     grounded: None,
  //     crouching: false,
  //     tac_velocity: 0.0,
  //     last_ground: Stopwatch::default(),
  //     last_tac: Stopwatch::default(),
  //     last_step_up: Stopwatch::default(),
  //     last_step_down: Stopwatch::default(),
  //     crane_height_left: None,
  //     mantle: None,
  //   };

  this.grounded.is_some() != that.grounded.is_some()
    || this.crouching != that.crouching
    || this.tac_velocity - that.tac_velocity >= 0.01
    || this.last_ground != that.last_ground
    || this.last_tac != that.last_tac
    || this.last_step_up != that.last_step_up
    || this.last_step_down != that.last_step_down
    || this.crane_height_left.is_some() != that.crane_height_left.is_some()
    || this.mantle.is_some() != that.mantle.is_some()
}
