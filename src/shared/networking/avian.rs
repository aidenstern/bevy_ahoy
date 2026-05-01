use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::{
  app::{App, Plugin, PostUpdate},
  ecs::schedule::IntoScheduleConfigs,
  transform::components::Transform,
};
use crate::kcc::AhoySystems;
use lightyear::{
  avian3d::plugin::AvianReplicationMode,
  frame_interpolation::{FrameInterpolationPlugin, FrameInterpolationSystems},
  prelude::{
    AppComponentExt, InterpolationRegistrationExt, PredictionRegistrationExt,
    RollbackSystems,
  },
};

pub struct SimpleAvianSetupPlugin;

impl Plugin for SimpleAvianSetupPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins((
        lightyear::avian3d::plugin::LightyearAvianPlugin {
          replication_mode:
            AvianReplicationMode::PositionButInterpolateTransform,
          ..Default::default()
        },
        // FrameInterpolationPlugin::<Position>::default(),
        // FrameInterpolationPlugin::<Rotation>::default(),
        FrameInterpolationPlugin::<Transform>::default(),
      ))
      .configure_sets(
        PostUpdate,
        (
          (
            RollbackSystems::VisualCorrection,
            FrameInterpolationSystems::Interpolate,
          ),
          AhoySystems::UpdateCameras,
        )
          .chain(),
      );

    // Fully replicated, but not visual, so no need for lerp/corrections:
    app
      .register_component::<LinearVelocity>()
      .add_prediction()
      .add_should_rollback(linear_velocity_should_rollback);

    app
      .register_component::<AngularVelocity>()
      .add_prediction()
      .add_should_rollback(angular_velocity_should_rollback);

    // Position and Rotation have a `correction_fn` set, which is used to smear rollback errors
    // over a few frames, just for the rendering part in PostUpdate.
    //
    // We also set `interpolation_fn` which is used by the VisualInterpolationPlugin to smooth
    // out rendering between fixedupdate ticks.
    app
      .register_component::<Position>()
      .add_prediction()
      .add_should_rollback(position_should_rollback)
      // .add_linear_correction_fn()
      .enable_correction()
      .add_linear_interpolation();

    app
      .register_component::<Rotation>()
      .add_prediction()
      .add_should_rollback(rotation_should_rollback)
      // .add_linear_correction_fn()
      .enable_correction()
      .add_linear_interpolation();
  }
}

fn position_should_rollback(this: &Position, that: &Position) -> bool {
  (this.0 - that.0).length() >= 0.01
}

fn rotation_should_rollback(this: &Rotation, that: &Rotation) -> bool {
  this.angle_between(*that) >= 0.01
}

fn linear_velocity_should_rollback(
  this: &LinearVelocity,
  that: &LinearVelocity,
) -> bool {
  (this.0 - that.0).length() >= 0.01
}

fn angular_velocity_should_rollback(
  this: &AngularVelocity,
  that: &AngularVelocity,
) -> bool {
  (this.0 - that.0).length() >= 0.01
}
