use avian3d::prelude::*;
use bevy_app::prelude::*;
use lightyear::avian3d::plugin::{AvianReplicationMode, LightyearAvianPlugin};
use lightyear::prelude::*;

use crate::CharacterLook;

/// Plugin that configures networking for bevy_ahoy using lightyear.
///
/// This plugin:
/// - Adds the [`LightyearAvianPlugin`] with [`AvianReplicationMode::Position`]
/// - Registers [`Position`], [`Rotation`], [`LinearVelocity`], and [`CharacterLook`]
///   for replication with prediction, rollback, correction, and interpolation
///
/// # Usage
///
/// Add this plugin to your app **after** adding [`lightyear::prelude::ClientPlugins`] or
/// [`lightyear::prelude::ServerPlugins`], and **before** adding physics plugins.
///
/// When adding [`PhysicsPlugins`], you **must** disable the following plugins
/// since lightyear handles their responsibilities:
///
/// ```ignore
/// app.add_plugins(
///     PhysicsPlugins::default()
///         .build()
///         .disable::<PhysicsTransformPlugin>()
///         .disable::<PhysicsInterpolationPlugin>()
///         .disable::<IslandPlugin>()
///         .disable::<IslandSleepingPlugin>(),
/// );
/// ```
pub struct AhoyNetworkingPlugin;

impl Plugin for AhoyNetworkingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LightyearAvianPlugin {
            replication_mode: AvianReplicationMode::Position,
            ..Default::default()
        });

        app.register_component::<Position>()
            .add_prediction()
            .add_should_rollback(position_should_rollback)
            .add_linear_correction_fn()
            .add_linear_interpolation();

        app.register_component::<Rotation>()
            .add_prediction()
            .add_should_rollback(rotation_should_rollback)
            .add_linear_correction_fn()
            .add_linear_interpolation();

        app.register_component::<LinearVelocity>()
            .add_prediction()
            .add_should_rollback(linear_velocity_should_rollback);

        app.register_component::<CharacterLook>()
            .add_prediction();
    }
}

fn position_should_rollback(this: &Position, that: &Position) -> bool {
    (this.0 - that.0).length() >= 0.01
}

fn rotation_should_rollback(this: &Rotation, that: &Rotation) -> bool {
    this.angle_between(*that) >= 0.01
}

fn linear_velocity_should_rollback(this: &LinearVelocity, that: &LinearVelocity) -> bool {
    (this.0 - that.0).length() >= 0.01
}
