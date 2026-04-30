//! Map loading + collider hydration.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::game::GameState;

pub const SPAWN_POINT: Vec3 = Vec3::new(0.0, 20.0, 0.0);

#[derive(Component)]
struct MapRoot;

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_map)
            .add_systems(
                Update,
                add_map_colliders.run_if(in_state(GameState::Loading)),
            )
            .add_systems(OnEnter(GameState::InGame), || info!("Scene ready: GameState::InGame"));
    }
}

fn load_map(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((
        MapRoot,
        SceneRoot(assets.load("maps/playground.glb#Scene0")),
        RigidBody::Static,
    ));
}

fn add_map_colliders(
    mut commands: Commands,
    map: Query<Entity, (With<MapRoot>, Without<ColliderConstructorHierarchy>)>,
    children: Query<&Children>,
    mesh_handles: Query<&Mesh3d>,
    meshes: Res<Assets<Mesh>>,
    colliders: Query<(), With<Collider>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok(map_entity) = map.single() else {
        return;
    };
    let has_collider = children
        .iter_descendants(map_entity)
        .any(|child| colliders.contains(child));
    if has_collider {
        next_state.set(GameState::InGame);
        return;
    }
    let has_any_mesh = children
        .iter_descendants(map_entity)
        .any(|child| mesh_handles.contains(child));
    let all_meshes_loaded = children
        .iter_descendants(map_entity)
        .filter_map(|child| mesh_handles.get(child).ok())
        .all(|handle| meshes.contains(&handle.0));
    if has_any_mesh && all_meshes_loaded {
        commands
            .entity(map_entity)
            .insert(ColliderConstructorHierarchy::new(
                ColliderConstructor::ConvexHullFromMesh,
            ));
    }
}
