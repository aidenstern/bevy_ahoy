//! Visual quality polish: post-processing on the camera, sun rotation,
//! material tweaks, mipmap generation hookup, crosshair UI.

use bevy::{
    camera::Exposure,
    light::{CascadeShadowConfigBuilder, light_consts::lux},
    pbr::{Atmosphere, ScatteringMedium},
    post_process::bloom::Bloom,
    prelude::*,
};
use bevy_mod_mipmap_generator::generate_mipmaps;
use core::f32::consts::TAU;

use crate::GameState;

pub struct VisualsPlugin;

impl Plugin for VisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::InGame),
            (spawn_crosshair, spawn_directional_light),
        )
        .add_systems(
            Update,
            (
                tweak_materials,
                generate_mipmaps::<StandardMaterial>,
                turn_sun,
            )
                .run_if(in_state(GameState::InGame)),
        )
        .add_observer(tweak_camera)
        .add_observer(tweak_directional_light);
    }
}

fn spawn_directional_light(mut commands: Commands) {
    commands.spawn((
        Transform::from_xyz(0.0, 1.0, 0.0).looking_at(vec3(1.0, -2.0, -2.0), Vec3::Y),
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
    ));
}

fn spawn_crosshair(mut commands: Commands, asset_server: Res<AssetServer>) {
    let crosshair_texture = asset_server.load("sprites/crosshair.png");
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(ImageNode::new(crosshair_texture).with_color(Color::WHITE.with_alpha(0.3)));
        });
}

fn tweak_camera(
    insert: On<Insert, Camera3d>,
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut scattering_mediums: ResMut<Assets<ScatteringMedium>>,
) {
    commands.entity(insert.entity).insert((
        EnvironmentMapLight {
            diffuse_map: assets.load("environment_maps/voortrekker_interior_1k_diffuse.ktx2"),
            specular_map: assets.load("environment_maps/voortrekker_interior_1k_specular.ktx2"),
            intensity: 600.0,
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: 70.0_f32.to_radians(),
            ..default()
        }),
        Atmosphere::earthlike(scattering_mediums.add(ScatteringMedium::default())),
        Exposure { ev100: 9.0 },
        Bloom::default(),
    ));
}

#[derive(Component)]
struct Tweaked;

fn tweak_directional_light(
    insert: On<Insert, DirectionalLight>,
    mut commands: Commands,
    directional_light: Query<&DirectionalLight, Without<Tweaked>>,
    tweaked: Query<Entity, With<Tweaked>>,
) {
    let Ok(light) = directional_light.get(insert.entity) else {
        return;
    };
    commands.entity(insert.entity).remove::<DirectionalLight>();

    for entity in tweaked.iter() {
        commands.entity(entity).despawn();
    }
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: lux::AMBIENT_DAYLIGHT,
            ..*light
        },
        Transform::IDENTITY,
        Tweaked,
        CascadeShadowConfigBuilder {
            maximum_distance: 500.0,
            overlap_proportion: 0.4,
            ..default()
        }
        .build(),
    ));
}

fn turn_sun(mut suns: Query<&mut Transform, With<DirectionalLight>>, time: Res<Time>) {
    for mut transform in suns.iter_mut() {
        transform.rotation =
            Quat::from_rotation_x(
                -((-time.elapsed_secs() / 100.0) + TAU / 8.0).sin().abs() * TAU / 2.05,
            ) * Quat::from_rotation_y(((-time.elapsed_secs() / 100.0) + 1.0).sin());
    }
}

fn tweak_materials(
    mut asset_events: MessageReader<AssetEvent<StandardMaterial>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    for event in asset_events.read() {
        let AssetEvent::LoadedWithDependencies { id } = event else {
            continue;
        };
        let Some(mat) = mats.get_mut(*id) else {
            continue;
        };
        mat.perceptual_roughness = 0.8;
    }
}
