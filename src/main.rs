use avian3d::prelude::*;
use bevy::{
    camera::Exposure,
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    image::{ImageAddressMode, ImageSamplerDescriptor},
    input::common_conditions::input_just_pressed,
    light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap, light_consts::lux},
    pbr::{Atmosphere, ScatteringMedium},
    platform::collections::HashSet,
    post_process::bloom::Bloom,
    prelude::*,
    time::common_conditions::on_timer,
    window::{CursorGrabMode, CursorOptions},
};
use bevy_ahoy::{CharacterControllerOutput, PickupHoldConfig, PickupPullConfig, prelude::*};
use bevy_enhanced_input::prelude::{Press, Release, *};
use bevy_framepace::FramepacePlugin;
use bevy_mod_mipmap_generator::{MipmapGeneratorPlugin, generate_mipmaps};
use core::f32::consts::TAU;
use std::{collections::VecDeque, time::Duration};

const SPAWN_POINT: Vec3 = Vec3::new(0.0, 20.0, 0.0);
const NPC_SPAWN_POINT: Vec3 = Vec3::new(-55.0, 55.0, 1.0);

/// If set, the game will automatically exit after this many frames.
#[derive(Resource)]
struct ExitAfterFrames(u32);

fn main() -> AppExit {
    let exit_after = std::env::args()
        .position(|a| a == "--frames")
        .and_then(|i| std::env::args().nth(i + 1))
        .and_then(|v| v.parse::<u32>().ok());

    let mut app = App::new();
    app.add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Window {
                        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "macos")))]
                        present_mode: bevy::window::PresentMode::Mailbox,
                        ..default()
                    }
                    .into(),
                    ..default()
                })
                .set(ImagePlugin {
                    default_sampler: ImageSamplerDescriptor {
                        address_mode_u: ImageAddressMode::Repeat,
                        address_mode_v: ImageAddressMode::Repeat,
                        address_mode_w: ImageAddressMode::Repeat,
                        anisotropy_clamp: 16,
                        ..ImageSamplerDescriptor::linear()
                    },
                }),
            PhysicsPlugins::default(),
            EnhancedInputPlugin,
            AhoyPlugins::default(),
            MipmapGeneratorPlugin,
            FramepacePlugin,
        ))
        .add_input_context::<PlayerInput>()
        .add_input_context::<DebugInput>()
        .add_input_context::<Npc>()
        .add_systems(Startup, (setup, setup_ui, spawn_crosshair, spawn_npc))
        .add_systems(
            Update,
            (
                capture_cursor.run_if(input_just_pressed(MouseButton::Left)),
                release_cursor.run_if(input_just_pressed(KeyCode::Escape)),
                update_debug_text,
                tweak_materials,
                generate_mipmaps::<StandardMaterial>,
                calculate_stable_ground.run_if(on_timer(Duration::from_secs(1))),
                apply_last_stable_ground.after(calculate_stable_ground),
                turn_sun,
            ),
        )
        .add_systems(FixedUpdate, update_npc)
        .add_observer(reset_player)
        .add_observer(toggle_debug)
        .add_observer(tweak_camera)
        .add_observer(tweak_directional_light)
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .insert_resource(GlobalAmbientLight::NONE)
        .add_systems(Update, (
            add_map_colliders,
            debug_colliders.run_if(on_timer(Duration::from_secs(3))),
        ));

    if let Some(frames) = exit_after {
        app.insert_resource(ExitAfterFrames(frames))
            .add_systems(Update, auto_exit);
    }

    app.run()
}

fn auto_exit(
    frame_count: Res<bevy::diagnostic::FrameCount>,
    limit: Res<ExitAfterFrames>,
    mut exit: MessageWriter<AppExit>,
) {
    if frame_count.0 >= limit.0 {
        info!("Smoke test passed: exiting after {} frames", frame_count.0);
        exit.write(AppExit::Success);
    }
}

fn debug_colliders(
    map: Query<(Entity, Option<&Children>, Option<&Collider>, Option<&RigidBody>, Option<&ColliderConstructorHierarchy>), With<SceneRoot>>,
    all_colliders: Query<(Entity, &Collider, Option<&RigidBody>, Option<&ChildOf>)>,
    children_q: Query<&Children>,
) {
    for (entity, children, collider, rb, cch) in &map {
        let child_count = children.map(|c| c.len()).unwrap_or(0);
        let descendant_count = children_q.iter_descendants(entity).count();
        info!("Map entity {entity}: children={child_count}, descendants={descendant_count}, has_collider={}, has_rb={}, has_cch={}", collider.is_some(), rb.is_some(), cch.is_some());
    }
    let total = all_colliders.iter().count();
    let with_rb = all_colliders.iter().filter(|(_, _, rb, _)| rb.is_some()).count();
    info!("Total entities with Collider: {total}, of those with RigidBody: {with_rb}");
}

// --- Core setup ---

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    let player = commands
        .spawn((
            Player,
            Transform::from_translation(SPAWN_POINT),
            CollisionLayers::new(CollisionLayer::Player, LayerMask::ALL),
            PlayerInput,
            CharacterController::default(),
            RigidBody::Kinematic,
            Collider::cylinder(0.7, 1.8),
            Mass(90.0),
            StableGround::default(),
        ))
        .id();

    commands.spawn((
        Camera3d::default(),
        CharacterControllerCameraOf::new(player),
        PickupConfig {
            prop_filter: SpatialQueryFilter::from_mask(CollisionLayer::Prop),
            actor_filter: SpatialQueryFilter::from_mask(CollisionLayer::Player),
            obstacle_filter: SpatialQueryFilter::from_mask(CollisionLayer::Default),
            hold: PickupHoldConfig {
                preferred_distance: 0.9,
                linear_velocity_easing: 0.8,
                ..default()
            },
            pull: PickupPullConfig {
                max_prop_mass: 1000.0,
                ..default()
            },
            ..default()
        },
    ));

    commands.spawn((
        Transform::from_xyz(0.0, 1.0, 0.0).looking_at(vec3(1.0, -2.0, -2.0), Vec3::Y),
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
    ));

    commands.spawn((
        MapRoot,
        SceneRoot(assets.load("maps/playground.glb#Scene0")),
        RigidBody::Static,
    ));
}

#[derive(Component)]
struct MapRoot;

fn add_map_colliders(
    mut commands: Commands,
    map: Query<Entity, (With<MapRoot>, Without<ColliderConstructorHierarchy>)>,
    children: Query<&Children>,
    mesh_handles: Query<&Mesh3d>,
    meshes: Res<Assets<Mesh>>,
) {
    let Ok(map_entity) = map.single() else {
        return;
    };
    let all_meshes_loaded = children
        .iter_descendants(map_entity)
        .filter_map(|child| mesh_handles.get(child).ok())
        .all(|handle| meshes.contains(&handle.0));
    let has_any_mesh = children
        .iter_descendants(map_entity)
        .any(|child| mesh_handles.contains(child));
    if has_any_mesh && all_meshes_loaded {
        info!("All map mesh assets loaded, adding ColliderConstructorHierarchy");
        commands
            .entity(map_entity)
            .insert(ColliderConstructorHierarchy::new(ColliderConstructor::ConvexHullFromMesh));
    }
}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
struct Player;

#[derive(Debug, PhysicsLayer, Default)]
enum CollisionLayer {
    #[default]
    Default,
    Player,
    Prop,
}

// --- Player input ---

#[derive(Component, Default)]
#[component(on_add = PlayerInput::on_add)]
struct PlayerInput;

impl PlayerInput {
    fn on_add(mut world: DeferredWorld, ctx: HookContext) {
        world
            .commands()
            .entity(ctx.entity)
            .insert(actions!(PlayerInput[
                (
                    Action::<Movement>::new(),
                    DeadZone::default(),
                    Bindings::spawn((Cardinal::wasd_keys(), Axial::left_stick()))
                ),
                (
                    Action::<Jump>::new(),
                    Press::default(),
                    bindings![
                        KeyCode::Space,
                        GamepadButton::South,
                        Binding::mouse_wheel(),
                    ],
                ),
                (
                    Action::<Tac>::new(),
                    Press::default(),
                    bindings![
                        KeyCode::Space,
                        GamepadButton::South,
                        Binding::mouse_wheel(),
                    ],
                ),
                (
                    Action::<Crane>::new(),
                    Press::default(),
                    bindings![
                        KeyCode::Space,
                        GamepadButton::South,
                        Binding::mouse_wheel(),
                    ],
                ),
                (
                    Action::<Mantle>::new(),
                    Hold::new(0.2),
                    bindings![KeyCode::Space, GamepadButton::South],
                ),
                (
                    Action::<Climbdown>::new(),
                    bindings![KeyCode::ControlLeft, GamepadButton::LeftTrigger2],
                ),
                (
                    Action::<Crouch>::new(),
                    bindings![KeyCode::ControlLeft, GamepadButton::LeftTrigger2],
                ),
                (
                    Action::<SwimUp>::new(),
                    bindings![KeyCode::Space, GamepadButton::South],
                ),
                (
                    Action::<PullObject>::new(),
                    ActionSettings { consume_input: true, ..default() },
                    Press::default(),
                    bindings![MouseButton::Right],
                ),
                (
                    Action::<DropObject>::new(),
                    ActionSettings { consume_input: true, ..default() },
                    Press::default(),
                    bindings![MouseButton::Right],
                ),
                (
                    Action::<ThrowObject>::new(),
                    ActionSettings { consume_input: true, ..default() },
                    Press::default(),
                    bindings![MouseButton::Left],
                ),
                (
                    Action::<RotateCamera>::new(),
                    Bindings::spawn((
                        Spawn((Binding::mouse_motion(), Scale::splat(0.07))),
                        Axial::right_stick().with((Scale::splat(4.0), DeadZone::default())),
                    ))
                ),
            ]));
    }
}

// --- Cursor ---

fn capture_cursor(mut cursor: Single<&mut CursorOptions>) {
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
}

fn release_cursor(mut cursor: Single<&mut CursorOptions>) {
    cursor.visible = true;
    cursor.grab_mode = CursorGrabMode::None;
}

// --- Debug UI ---

#[derive(Component)]
struct DebugText;

#[derive(Component, Default)]
struct DebugInput;

#[derive(Debug, InputAction)]
#[action_output(bool)]
struct Reset;

#[derive(Debug, InputAction)]
#[action_output(bool)]
struct ToggleDebug;

fn setup_ui(mut commands: Commands) {
    commands.spawn((Node::default(), Text::default(), Visibility::Hidden, DebugText));
    commands.spawn((
        Node {
            justify_self: JustifySelf::End,
            justify_content: JustifyContent::End,
            align_self: AlignSelf::End,
            padding: UiRect::all(px(10.0)),
            ..default()
        },
        Text::new(
            "Controls:\nWASD: move\nSpace: jump\nCtrl: crouch\nEsc: free mouse\nR: reset position\nBacktick: Toggle Debug Menu",
        ),
    ));
    commands.spawn((
        DebugInput,
        actions!(DebugInput[
            (
                Action::<Reset>::new(),
                bindings![KeyCode::KeyR, GamepadButton::Select],
                Release::default(),
            ),
            (
                Action::<ToggleDebug>::new(),
                bindings![KeyCode::Backquote, GamepadButton::Start],
                Release::default(),
            ),
        ]),
    ));
}

fn reset_player(
    _fire: On<Fire<Reset>>,
    mut player: Query<(&mut Position, &mut LinearVelocity), With<Player>>,
    mut camera: Query<&mut Transform, (With<Camera3d>, Without<Player>)>,
) {
    let Ok((mut position, mut velocity)) = player.single_mut() else {
        return;
    };
    velocity.0 = Vec3::ZERO;
    position.0 = SPAWN_POINT;
    let Ok(mut camera_transform) = camera.single_mut() else {
        return;
    };
    camera_transform.rotation = Quat::IDENTITY;
}

fn toggle_debug(
    _fire: On<Fire<ToggleDebug>>,
    mut visibility: Single<&mut Visibility, With<DebugText>>,
) {
    **visibility = match **visibility {
        Visibility::Hidden => Visibility::Inherited,
        _ => Visibility::Hidden,
    };
}

fn update_debug_text(
    mut text: Single<&mut Text, With<DebugText>>,
    kcc: Single<
        (
            &CharacterControllerState,
            &CharacterControllerOutput,
            &LinearVelocity,
            &CollidingEntities,
            &ColliderAabb,
            &StableGround,
        ),
        (With<CharacterController>, With<CharacterControllerCamera>),
    >,
    camera: Single<&Transform, With<Camera>>,
    names: Query<NameOrEntity>,
) {
    let (state, output, velocity, colliding_entities, aabb, stable_ground) = kcc.into_inner();
    let velocity = **velocity;
    let speed = velocity.length();
    let horizontal_speed = velocity.xz().length();
    let camera_position = camera.translation;
    let collisions = names
        .iter_many(
            output
                .touching_entities
                .iter()
                .map(|e| e.entity)
                .collect::<HashSet<_>>(),
        )
        .map(|name| {
            name.name
                .map(|n| format!("{} ({})", name.entity, n))
                .unwrap_or_else(|| format!("{}", name.entity))
        })
        .collect::<Vec<_>>();
    let real_collisions = names
        .iter_many(colliding_entities.iter())
        .map(|name| {
            name.name
                .map(|n| format!("{} ({})", name.entity, n))
                .unwrap_or_else(|| format!("{}", name.entity))
        })
        .collect::<Vec<_>>();
    let ground = state
        .grounded
        .and_then(|ground| names.get(ground.entity).ok())
        .map(|name| {
            name.name
                .map(|n| format!("{} ({})", name.entity, n))
                .unwrap_or(format!("{}", name.entity))
        });
    let stable_ground = stable_ground.previous.back();
    text.0 = format!(
        "Speed: {speed:.3}\nHorizontal Speed: {horizontal_speed:.3}\nVelocity: [{:.3}, {:.3}, {:.3}]\nCamera Position: [{:.3}, {:.3}, {:.3}]\nCollider Aabb:\n  min:[{:.3}, {:.3}, {:.3}]\n  max:[{:.3}, {:.3}, {:.3}]\nReal Collisions: {:#?}\nCollisions: {:#?}\nGround: {:?}\nLast Stable Ground: {:?}",
        velocity.x, velocity.y, velocity.z,
        camera_position.x, camera_position.y, camera_position.z,
        aabb.min.x, aabb.min.y, aabb.min.z,
        aabb.max.x, aabb.max.y, aabb.max.z,
        real_collisions, collisions, ground, stable_ground,
    );
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

// --- Camera and light tweaks ---

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
    assets: Res<AssetServer>,
) {
    for event in asset_events.read() {
        let AssetEvent::LoadedWithDependencies { id } = event else {
            continue;
        };
        let Some(mat) = mats.get_mut(*id) else {
            continue;
        };
        if mat
            .base_color_texture
            .as_ref()
            .and_then(|t| {
                assets
                    .get_path(t.id())?
                    .path()
                    .file_name()?
                    .to_string_lossy()
                    .to_lowercase()
                    .into()
            })
            .is_some_and(|name: String| name.contains("water_01"))
        {
            mat.base_color = Color::WHITE.with_alpha(0.85);
            mat.perceptual_roughness = 0.2;
            mat.alpha_mode = AlphaMode::Blend;
        } else {
            mat.perceptual_roughness = 0.8;
        }
    }
}

// --- Stable ground respawn ---

#[derive(Component, Reflect)]
struct StableGround {
    previous: VecDeque<Vec3>,
    fall_timer: Timer,
}

impl Default for StableGround {
    fn default() -> Self {
        Self {
            previous: VecDeque::default(),
            fall_timer: Timer::new(Duration::from_secs(5), TimerMode::Once),
        }
    }
}

fn calculate_stable_ground(
    mut kccs: Query<(&Transform, &CharacterControllerState, &mut StableGround)>,
) {
    for (transform, state, mut stable_ground) in &mut kccs {
        let Some(ground) = state.grounded else {
            continue;
        };
        let up_diff = (1. - ground.normal1.y).abs();
        if up_diff <= f32::EPSILON {
            stable_ground.previous.push_front(transform.translation);
            while stable_ground.previous.len() > 5 {
                stable_ground.previous.pop_back();
            }
        }
    }
}

fn apply_last_stable_ground(
    mut kccs: Query<(
        &mut Transform,
        &LinearVelocity,
        &CharacterController,
        &mut StableGround,
    )>,
    time: Res<Time>,
) {
    for (mut transform, velocity, controller, mut stable_ground) in &mut kccs {
        let speed_diff = 1. - (velocity.0.y.abs() / controller.max_speed);
        if speed_diff <= 0.01 {
            stable_ground.fall_timer.tick(time.elapsed());
        } else {
            stable_ground.fall_timer.reset();
        }
        if stable_ground.fall_timer.is_finished()
            && let Some(last_stable_ground) = stable_ground.previous.pop_front()
        {
            transform.translation = last_stable_ground;
        }
    }
}

// --- NPC ---

#[derive(Component, Default)]
#[component(on_add = Npc::on_add)]
#[require(
    CharacterController,
    RigidBody::Kinematic,
    Collider::cylinder(0.7, 1.8),
    Mass(90.0)
)]
struct Npc {
    step: usize,
    timer: Timer,
}

impl Npc {
    fn on_add(mut world: DeferredWorld, ctx: HookContext) {
        let Some(collider) = world
            .get::<Collider>(ctx.entity)
            .map(|c| c.shape_scaled().clone())
        else {
            return;
        };
        let mesh = world
            .resource_mut::<Assets<Mesh>>()
            .add(Cylinder::new(
                collider.as_cylinder().unwrap().radius,
                collider.as_cylinder().unwrap().half_height * 2.0,
            ));
        let material = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(Color::WHITE);
        world
            .commands()
            .entity(ctx.entity)
            .insert((Mesh3d(mesh), MeshMaterial3d(material)));
        world.commands().entity(ctx.entity).insert(actions!(Npc[
            (
                Action::<GlobalMovement>::new(),
                ActionMock {
                    state: TriggerState::Fired,
                    value: Vec3::ZERO.into(),
                    span: Duration::from_secs(2).into(),
                    enabled: false
                }
            ),
            (
                Action::<Jump>::new(),
                ActionMock {
                    state: TriggerState::Fired,
                    value: true.into(),
                    span: Duration::from_secs(2).into(),
                    enabled: false
                }
            ),
        ]));
    }
}

fn spawn_npc(mut commands: Commands) {
    commands.spawn((Npc::default(), Transform::from_translation(NPC_SPAWN_POINT)));
}

fn update_npc(mut commands: Commands, time: Res<Time>, mut npcs: Query<(Entity, &mut Npc)>) {
    for (entity, mut npc) in &mut npcs {
        npc.timer.tick(time.delta());
        if npc.timer.is_finished() {
            if npc.timer.duration() != Duration::ZERO {
                npc.step += 1;
            }
            let duration = match npc.step {
                0..=4 => 1.0,
                5 | 7 | 9 => 0.2,
                6 | 8 | 10 => 0.8,
                _ => {
                    npc.step = 0;
                    1.0
                }
            };
            npc.timer.set_duration(Duration::from_secs_f32(duration));
            npc.timer.reset();
        }

        let (move_vec, jump) = match npc.step {
            0 => (Vec3::NEG_Z, false),
            1 => (Vec3::Z, false),
            2 => (Vec3::NEG_X, false),
            3 => (Vec3::X, false),
            5..=9 => (Vec3::ZERO, true),
            _ => (Vec3::ZERO, false),
        };
        commands
            .entity(entity)
            .mock_once::<Npc, GlobalMovement>(TriggerState::Fired, move_vec);
        if jump {
            commands
                .entity(entity)
                .mock_once::<Npc, Jump>(TriggerState::Fired, true);
        }
    }
}
