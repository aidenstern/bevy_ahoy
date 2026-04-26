use std::{f32::consts::TAU, time::Duration};

#[cfg(feature = "pickup")]
use avian_pickup::actor::AvianPickupActor;
use bevy_ecs::{lifecycle::HookContext, relationship::Relationship, world::DeferredWorld};

use crate::{
    CharacterControllerDerivedProps, CharacterControllerState, CharacterLook, kcc::spin_kcc,
    prelude::*,
};

pub struct AhoyCameraPlugin;

impl Plugin for AhoyCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            RunFixedMainLoop,
            (
                copy_camera_to_character_look.in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
                sync_camera_transform.after(TransformEasingSystems::UpdateEasingTick),
                copy_kcc_yaw_to_camera,
            ),
        )
        .add_systems(Update, copy_character_look_to_camera.after(spin_kcc))
        .add_observer(rotate_camera)
        .add_observer(yank_camera);
    }
}

#[derive(Component, Clone, Copy, Debug)]
#[relationship(relationship_target = CharacterControllerCamera)]
#[require(Transform)]
#[cfg_attr(feature = "pickup", require(AvianPickupActor))]
#[component(on_add = Self::on_add)]
pub struct CharacterControllerCameraOf {
    #[relationship]
    pub character_controller: Entity,
    pub enable_smoothing: bool,
    pub step_smooth_time: Duration,
    pub teleport_detection_distance: f32,
    /// The yank speed (rotation rate) in **radians per second**.
    pub yank_speed: f32,
}

impl CharacterControllerCameraOf {
    pub fn new(character_controller: Entity) -> Self {
        Self {
            character_controller,
            enable_smoothing: true,
            step_smooth_time: Duration::from_millis(200),
            teleport_detection_distance: 10.0,
            yank_speed: 210.0_f32.to_radians(),
        }
    }
}

impl CharacterControllerCameraOf {
    fn on_add(mut world: DeferredWorld, ctx: HookContext) {
        let Some(kcc) = world.get::<Self>(ctx.entity).copied() else {
            return;
        };
        let Some(kcc_transform) = world.get::<Transform>(kcc.get()).copied() else {
            return;
        };
        let Some(mut camera_transform) = world.get_mut::<Transform>(ctx.entity) else {
            return;
        };
        *camera_transform = kcc_transform;
    }
}

#[derive(Component, Clone, Copy, Debug)]
#[relationship_target(relationship = CharacterControllerCameraOf)]
#[require(CharacterLook)]
pub struct CharacterControllerCamera(Entity);

impl CharacterControllerCamera {
    pub fn get(self) -> Entity {
        self.0
    }
}

pub(crate) fn sync_camera_transform(
    mut cameras: Query<
        (&mut Transform, &CharacterControllerCameraOf),
        (Without<CharacterControllerState>,),
    >,
    kccs: Query<(
        &Transform,
        &CharacterController,
        &CharacterControllerState,
        &CharacterControllerDerivedProps,
    )>,
    time: Res<Time>,
) {
    // TODO: DIY TransformHelper to use current global transform.
    // Can't use GlobalTransform directly: outdated -> jitter
    // Can't use TransformHelper directly: access conflict with &mut Transform
    // Can't use Position: that is not interpolated -> jitter
    for (mut camera_transform, camera) in cameras.iter_mut() {
        if let Ok((kcc_transform, cfg, state, derived)) = kccs.get(camera.character_controller) {
            let height = derived
                // changing the collider does not change the transform, so to get the correct position for the feet,
                // we need to use the collider we spawned with.
                .standing_collider
                .aabb(Vec3::default(), Rotation::default())
                .size()
                .y;
            let view_height = if state.crouching {
                cfg.crouch_view_height
            } else {
                cfg.standing_view_height
            };
            let new_translation =
                kcc_transform.translation + Vec3::Y * (-height / 2.0 + view_height);
            camera_transform.translation.x = new_translation.x;
            camera_transform.translation.z = new_translation.z;
            if !camera.enable_smoothing {
                camera_transform.translation.y = new_translation.y;
                return;
            }
            if state.last_step_up.elapsed() < camera.step_smooth_time
                || state.last_step_down.elapsed() < camera.step_smooth_time
            {
                let decay_rate = f32::ln(100000.0);
                camera_transform.translation.y.smooth_nudge(
                    &new_translation.y,
                    decay_rate,
                    time.delta_secs(),
                );
            } else if new_translation.y - camera_transform.translation.y
                < camera.teleport_detection_distance
            {
                let decay_rate = f32::ln(100_000_000.0);
                camera_transform.translation.y.smooth_nudge(
                    &new_translation.y,
                    decay_rate,
                    time.delta_secs(),
                );
            } else {
                camera_transform.translation.y = new_translation.y;
            }
        }
    }
}

fn copy_camera_to_character_look(
    mut character_looks: Query<(&CharacterControllerCamera, &mut CharacterLook)>,
    transforms: Query<&Transform>,
) {
    for (camera, mut character_look) in &mut character_looks {
        let Ok(transform) = transforms.get(camera.get()) else {
            continue;
        };

        *character_look = CharacterLook::from_quat(transform.rotation);
    }
}

fn copy_character_look_to_camera(
    cameras: Query<(&CharacterLook, &CharacterControllerCamera)>,
    mut transforms: Query<&mut Transform, Without<CharacterControllerCamera>>,
) {
    for (character_look, camera) in &cameras {
        let Ok(mut transform) = transforms.get_mut(camera.get()) else {
            continue;
        };
        character_look.apply_to_quat(&mut transform.rotation);
    }
}

fn rotate_camera(
    rotate: On<Fire<RotateCamera>>,
    cameras: Query<&CharacterControllerCamera>,
    mut transforms: Query<&mut Transform>,
) {
    let Ok(camera) = cameras.get(rotate.context) else {
        return;
    };

    let delta = -rotate.value;

    // First, rotate the KCC's yaw.
    let Ok(mut kcc_transform) = transforms.get_mut(rotate.context) else {
        return;
    };
    let (mut kcc_yaw, pitch, roll) = kcc_transform.rotation.to_euler(EulerRot::YXZ);
    kcc_yaw += delta.x.to_radians();
    kcc_transform.rotation = Quat::from_euler(EulerRot::YXZ, kcc_yaw, pitch, roll);

    // Now, make the camera match the KCC's yaw and apply the pitch.
    let Ok(mut camera_transform) = transforms.get_mut(camera.get()) else {
        return;
    };
    let (_, mut pitch, roll) = camera_transform.rotation.to_euler(EulerRot::YXZ);
    pitch += delta.y.to_radians();
    pitch = pitch.clamp(-TAU / 4.0 + 0.01, TAU / 4.0 - 0.01);

    camera_transform.rotation = Quat::from_euler(EulerRot::YXZ, kcc_yaw, pitch, roll);
}

fn yank_camera(
    trigger: On<Fire<YankCamera>>,
    cameras: Query<&CharacterControllerCamera>,
    camera_ofs: Query<&CharacterControllerCameraOf>,
    time: Res<Time>,
    mut transforms: Query<&mut Transform>,
) {
    let Ok(camera) = cameras.get(trigger.context) else {
        return;
    };
    let Ok(camera_of) = camera_ofs.get(camera.get()) else {
        return;
    };

    // Rotate the KCC's yaw.
    let Ok(mut kcc_transform) = transforms.get_mut(trigger.context) else {
        return;
    };
    let rotation_delta = camera_of.yank_speed * trigger.value * time.delta_secs();
    let (mut kcc_yaw, pitch, roll) = kcc_transform.rotation.to_euler(EulerRot::YXZ);
    kcc_yaw += rotation_delta;
    kcc_transform.rotation = Quat::from_euler(EulerRot::YXZ, kcc_yaw, pitch, roll);

    // copy_kcc_yaw_to_camera will sync the camera on the next frame.
}

fn copy_kcc_yaw_to_camera(
    cameras: Query<(Entity, &CharacterControllerCamera)>,
    mut transforms: Query<&mut Transform>,
) {
    for (kcc, camera) in &cameras {
        let Ok([kcc_transform, mut camera_transform]) =
            transforms.get_many_mut([kcc, camera.get()])
        else {
            continue;
        };

        let (kcc_yaw, _, _) = kcc_transform.rotation.to_euler(EulerRot::YXZ);
        let (_, pitch, roll) = camera_transform.rotation.to_euler(EulerRot::YXZ);
        camera_transform.rotation = Quat::from_euler(EulerRot::YXZ, kcc_yaw, pitch, roll);
    }
}
