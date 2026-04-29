# Bevy Ahoy Networking Plan (lightyear + bevy_enhanced_input)

Followup to the cleanup/restructure round (see `~/.claude/plans/floating-watching-kurzweil.md`).
Distilled from research of:
- `aidenstern/lightyear` @ `avian-0.6` examples: `avian_3d_character` (avian + leafwing) and `bevy_enhanced_inputs` (bei integration pattern).
- `../bevy_game/src/` — working leafwing-based reference (has the LogicalPlayer/RenderPlayer split, server/client/host-client wiring, scene collider hydration).

## Current state (after structural prep)

- `src/game/{cli,server,client,host}.rs` exist as feature-gated stubs.
- `cargo run` = host-client mode (single-player setup runs in all branches; networking not yet wired).
- `LogicalPlayer`, `PlayerId`, `RenderPlayer` types defined in `src/game/player.rs`.
- `src/networking.rs` (in the lib) registers `Position`/`Rotation`/`LinearVelocity`/`CharacterLook` for prediction. **Not wired into the binary's plugin list yet** — the binary doesn't currently call `AhoyNetworkingPlugin`.

## 1. Cargo.toml deltas

The `lightyear` dep currently has `["avian3d", "prediction", "interpolation", "replication"]`. For bei integration:

- Add `input_bei` to the lightyear feature list. Final: `["avian3d", "prediction", "interpolation", "replication", "input_bei"]`.
- Add per-mode lightyear sub-features so `client`/`server` only pull what they need:
  ```toml
  [features]
  default = ["client", "server"]
  client = ["networking", "lightyear/client", "lightyear/udp"]
  server = ["networking", "lightyear/server", "lightyear/udp"]
  serialize = ["dep:serde", "avian3d/serialize", "bevy_math/serialize"]
  networking = ["serialize", "dep:lightyear"]
  ```

## 2. Library `src/networking.rs` extension

Existing prediction registrations (`Position`, `Rotation`, `LinearVelocity`, `CharacterLook`) stay. Add:

- bei plugin per InputContext (one per context type):
  ```rust
  app.add_plugins(lightyear::input::bei::InputPlugin::<PlayerInput> {
      config: lightyear::input::config::InputConfig {
          rebroadcast_inputs: true,
          ..default()
      },
  });
  ```
- `register_input_action::<A>()` for every kept Action: `Movement`, `Jump`, `RotateCamera`, `Crouch`, `Mantle`, `Tac`, `Crane`, `Climbdown`, `SwimUp`.
- Component registrations:
  ```rust
  app.register_component::<CharacterController>()
      .add_prediction()
      .add_should_rollback(controller_should_rollback);
  app.register_component::<AccumulatedInput>().add_prediction();
  app.register_component::<CharacterControllerState>().add_prediction();
  app.register_component::<CharacterControllerDerivedProps>(); // no prediction; set up by setup_collider
  app.register_component::<CharacterControllerOutput>().add_prediction();
  app.register_component::<WaterState>().add_prediction(); // no-op in practice; kcc reads it
  app.register_component::<LogicalPlayer>().add_prediction();
  app.register_component::<PlayerId>();
  ```
  Write `controller_should_rollback` to compare only the few fields the kcc actually mutates each tick (height, ground_tick analogues, etc.) — see `~/dev/bevy/bevy_game/src/net/mod.rs:22-26` for shape.
- Pre-register `PlayerInput` (the bei InputContext Component) for replication:
  ```rust
  app.register_component::<PlayerInput>();
  ```
  This requires `PlayerInput` to derive `Serialize, Deserialize, Reflect, Clone, Debug, PartialEq` (currently just `Component, Default`).
- Disable Avian plugins per the lightyear pattern (replaces the `PhysicsPlugins::default()` line in `game::run`):
  ```rust
  app.add_plugins(lightyear::avian3d::plugin::LightyearAvianPlugin {
      replication_mode: AvianReplicationMode::Position,
      ..default()
  });
  app.add_plugins(
      PhysicsPlugins::default()
          .build()
          .disable::<PhysicsTransformPlugin>()
          .disable::<PhysicsInterpolationPlugin>()
          .disable::<IslandPlugin>()
          .disable::<IslandSleepingPlugin>(),
  );
  ```

## 3. Server module (`src/game/server.rs`)

```rust
use std::time::Duration;
use bevy::prelude::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use crate::game::player::{LogicalPlayer, PlayerId, CollisionLayer};
use crate::game::bindings::PlayerInput;
use crate::game::scene::SPAWN_POINT;

const SEND_INTERVAL: Duration = Duration::from_millis(100);

pub struct ServerPlugin;
impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(handle_new_link)
           .add_observer(handle_connected);
    }
}

// Per-client `LinkOf` entity: needs a sender (replicate state down) AND a
// receiver (bei Action entities are replicated UP from the client).
fn handle_new_link(trigger: On<Add, LinkOf>, mut commands: Commands) {
    commands.entity(trigger.entity).insert((
        ReplicationSender::new(SEND_INTERVAL, SendUpdatesMode::SinceLastAck, false),
        ReplicationReceiver::default(),
        Name::from("Client"),
    ));
}

fn handle_connected(
    trigger: On<Add, Connected>,
    query: Query<&RemoteId, With<ClientOf>>,
    mut commands: Commands,
) {
    let Ok(client_id) = query.get(trigger.entity) else { return };
    let client_id = client_id.0;
    info!("Client {client_id:?} connected, spawning LogicalPlayer");

    commands.spawn((
        LogicalPlayer,
        PlayerId(client_id.to_bits()),
        Transform::from_translation(SPAWN_POINT),
        // physics bundle
        avian3d::prelude::CollisionLayers::new(CollisionLayer::Player, avian3d::prelude::LayerMask::ALL),
        bevy_ahoy::prelude::CharacterController::default(),
        avian3d::prelude::RigidBody::Kinematic,
        avian3d::prelude::Collider::cylinder(0.7, 1.8),
        avian3d::prelude::Mass(90.0),
        // bei InputContext component (NO actions / NO bindings — client attaches those)
        PlayerInput,
        // lightyear replication
        Replicate::to_clients(NetworkTarget::All),
        PredictionTarget::to_clients(NetworkTarget::Single(client_id)),
        InterpolationTarget::to_clients(NetworkTarget::AllExceptSingle(client_id)),
        ControlledBy { owner: trigger.entity, lifetime: Default::default() },
    ));
}
```

For the dedicated server boot:
```rust
fn start_dedicated_server(mut commands: Commands, bind_addr: SocketAddr) {
    let server = commands.spawn((
        lightyear::netcode::NetcodeServer::new(NetcodeConfig::default()),
        LocalAddr(bind_addr),
        ServerUdpIo::default(),
    )).id();
    commands.trigger(Start { entity: server });
}
```
Wire it in `game::run`'s `Some(Mode::Server { bind_addr })` branch as a `Startup` system that captures `bind_addr`.

The respawn-below-floor system already exists in `src/game/player.rs` and queries `With<LogicalPlayer>` → it works server-side automatically once the server is the one moving the player.

## 4. Client module (`src/game/client.rs`)

```rust
use std::time::Duration;
use bevy::prelude::*;
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use lightyear::input::bei::prelude::{Action, ActionOf, Bindings, Cardinal};
use bevy_enhanced_input::prelude::{Press, Hold, *};
use crate::game::player::{LogicalPlayer, PlayerId, RenderPlayer};
use crate::game::bindings::PlayerInput;
use bevy_ahoy::prelude::*;

const SEND_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Component)]
struct RemotePlayerInitialized;

pub struct ClientPlugin;
impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (setup_local_player, setup_remote_player));
    }
}

fn setup_local_player(
    mut commands: Commands,
    new_local: Query<(Entity, Has<Controlled>), (Added<Predicted>, With<LogicalPlayer>)>,
) {
    for (entity, controlled) in &new_local {
        if !controlled { continue; }
        info!("Setting up local controlled player on {entity:?}");

        // Attach bei actions ONLY on the client side (server never has bindings).
        // Replaces the `PlayerInput::on_add` hook in src/game/bindings.rs — that
        // hook should be removed during the networking phase.
        commands.entity(entity).insert(actions!(PlayerInput[
            (Action::<Movement>::new(), DeadZone::default(),
             Bindings::spawn((Cardinal::wasd_keys(), Axial::left_stick()))),
            (Action::<Jump>::new(), Press::default(),
             bindings![KeyCode::Space, GamepadButton::South, Binding::mouse_wheel()]),
            (Action::<Tac>::new(), Press::default(),
             bindings![KeyCode::Space, GamepadButton::South, Binding::mouse_wheel()]),
            (Action::<Crane>::new(), Press::default(),
             bindings![KeyCode::Space, GamepadButton::South, Binding::mouse_wheel()]),
            (Action::<Mantle>::new(), Hold::new(0.2),
             bindings![KeyCode::Space, GamepadButton::South]),
            (Action::<Climbdown>::new(),
             bindings![KeyCode::ControlLeft, GamepadButton::LeftTrigger2]),
            (Action::<Crouch>::new(),
             bindings![KeyCode::ControlLeft, GamepadButton::LeftTrigger2]),
            (Action::<RotateCamera>::new(),
             Bindings::spawn((
                 Spawn((Binding::mouse_motion(), Scale::splat(0.07))),
                 Axial::right_stick().with((Scale::splat(4.0), DeadZone::default())),
             ))),
        ]));

        // Spawn camera + RenderPlayer link. The existing `tweak_camera` observer
        // (in src/game/visuals.rs) attaches Atmosphere/Bloom/Exposure/EnvMap on
        // Insert<Camera3d>, so post-processing is automatic.
        commands.spawn((
            Camera3d::default(),
            CharacterControllerCameraOf::new(entity),
            RenderPlayer { logical_entity: entity },
        ));
    }
}

fn setup_remote_player(
    mut commands: Commands,
    new_remote: Query<Entity, (
        With<Interpolated>, With<avian3d::prelude::Position>,
        With<avian3d::prelude::Rotation>, Without<RemotePlayerInitialized>,
    )>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    ids: Query<&PlayerId>,
) {
    for entity in &new_remote {
        let hue = ids.get(entity).map(|p| ((p.0 * 137) % 360) as f32).unwrap_or(0.0);
        commands.entity(entity).insert((
            Mesh3d(meshes.add(Cylinder::new(0.7, 1.8))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::hsl(hue, 0.7, 0.5),
                ..default()
            })),
            RemotePlayerInitialized,
        ));
    }
}
```

For the client connection boot:
```rust
fn start_client(mut commands: Commands, client_id: u64, server_addr: SocketAddr) {
    let auth = Authentication::Manual {
        server_addr, client_id,
        private_key: lightyear::netcode::Key::default(),
        protocol_id: 0,
    };
    let client = commands.spawn((
        Client::default(),
        LocalAddr(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)),
        PeerAddr(server_addr),
        Link::new(None),
        ReplicationReceiver::default(),
        // Required: bei Action entities replicate UP from client to server.
        ReplicationSender::new(SEND_INTERVAL, SendUpdatesMode::SinceLastAck, false),
        lightyear::netcode::NetcodeClient::new(auth, NetcodeConfig::default()).unwrap(),
        UdpIo::default(),
        // Tune input delay; start at 0, raise to ~10 if jitter is bad.
        InputTimelineConfig::default()
            .with_input_delay(InputDelayConfig::fixed_input_delay(0)),
    )).id();
    commands.trigger(Connect { entity: client });
}
```
Wire it in `game::run`'s `Some(Mode::Client { client_id, server_addr })` branch as a `Startup` system.

## 5. Bindings module changes (`src/game/bindings.rs`)

- Remove `#[component(on_add = PlayerInput::on_add)]` from `PlayerInput`. Reason: the on_add hook fires on the server when it spawns the context Component, and on the server we don't want bei to attempt to read physical input or attach bindings.
- Move the `actions!` macro invocation from `PlayerInput::on_add` into client-side `setup_local_player` (above).
- Change the derive on `PlayerInput`:
  ```rust
  #[derive(Component, Default, Reflect, Clone, Debug, PartialEq)]
  #[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
  #[cfg_attr(feature = "serialize", reflect(Serialize, Deserialize))]
  #[reflect(Component)]
  pub struct PlayerInput;
  ```

## 6. Host-client wiring (`src/game/host.rs`)

Mirror bevy_game's `setup_host_server` (`~/dev/bevy/bevy_game/src/main.rs:212-254`):

```rust
fn setup_host_server(app: &mut App) {
    let server = app.world_mut().spawn((
        lightyear::netcode::NetcodeServer::new(NetcodeConfig::default()),
        LocalAddr(DEFAULT_SERVER_ADDR),
        ServerUdpIo::default(),
    )).id();
    let client = app.world_mut().spawn((
        Client::default(),
        Name::new("HostClient"),
        LinkOf { server }, // shares process link, no UDP
    )).id();
    // CRITICAL: server must `Started` before client `Connect`s.
    app.add_systems(Startup, (
        move |mut c: Commands| { c.trigger(Start { entity: server }); },
        move |mut c: Commands| { c.trigger(Connect { entity: client }); },
    ).chain());
}
```
`HostPlugin::build` calls `setup_host_server(app)`.

## 7. game::run dispatch updates

Per-mode bevy plugin sets (currently all use `DefaultPlugins`):
- `Some(Mode::Server { bind_addr })`: swap `DefaultPlugins` → `MinimalPlugins` + `TransformPlugin`/`AssetPlugin`/`bevy::image::ImagePlugin`/`bevy::mesh::MeshPlugin`/`bevy::scene::ScenePlugin`/`bevy::state::app::StatesPlugin`/`bevy::gltf::GltfPlugin` + `init_asset::<StandardMaterial>()`. Drop `MipmapGeneratorPlugin`, `FramepacePlugin`, `EnhancedInputPlugin`, `DebugPlugin`, `CursorPlugin`, `VisualsPlugin`, `BindingsPlugin`, `SetupPlugin` (no local spawn). Keep `ScenePlugin`, `PlayerPlugin`, `AhoyPlugins`, `PhysicsPlugins` (with disabled subplugins per §2). Add `ServerPlugin` + a `Startup` system calling `start_dedicated_server(bind_addr)`.
- `Some(Mode::Client { client_id, server_addr })`: keep `DefaultPlugins` + visual plugins + `EnhancedInputPlugin`. Drop `SetupPlugin` (the client gets its player via replication, not local spawn). Add `ClientPlugin` + a `Startup` system calling `start_client(client_id, server_addr)`.
- `None` (host-client): keep current setup. Add `ServerPlugin` + `ClientPlugin` + `HostPlugin`. **Drop `SetupPlugin`** here too — the local player is spawned via the host-server flow (server's `handle_connected` triggers spawn, host-client receives via prediction).

In all modes, also add `AhoyNetworkingPlugin` (the lib's networking plugin) when `feature = "networking"` is on.

## 8. Camera-look replication

The current `CharacterLook` is computed client-side (camera transform → CharacterLook in `lib/src/camera.rs:147`). For server-authoritative play, two options:

- **(a)** Keep the client-side computation, add a "client→server" replication direction for `CharacterLook`. Simpler if lightyear supports it cleanly.
- **(b) (recommended)** Treat camera rotation as input: the existing `RotateCamera` bei action with Vec2 output already fires on mouse motion. Have the client *not* directly mutate the camera transform; instead, the bei action sends Vec2 deltas to the server, which accumulates yaw/pitch into `CharacterLook` server-side. Predicted clients also accumulate locally. The camera transform is then derived from `CharacterLook` (already done by `copy_character_look_to_camera` in `lib/src/camera.rs`).

Option (b) keeps all gameplay state server-authoritative and matches the lightyear input model.

## 9. Schedule alignment

`AhoyPlugins::default()` runs the kcc in `FixedPostUpdate`. Lightyear's prediction rollback re-runs `FixedFirst` through `FixedLast` (verify in lightyear source). If `FixedPostUpdate` is excluded for some reason, switch via `AhoyPlugins::new(FixedUpdate)` in `game::run`.

## 10. Open questions for execution

- `CharacterControllerState` contains `Stopwatch` (Duration-backed; serialize OK) and `Option<MoveHitData>` (has `Entity` ref → needs `MapEntities` impl on registration).
- The lightyear fork (`aidenstern/lightyear` @ `avian-0.6`) — verify it's still building and the `input_bei` feature still exists in the version pulled by `Cargo.lock`.
- `PlayerInput` as both a bei-context Component AND a lightyear-replicated Component — verify there's no conflict (the lightyear bei InputPlugin probably handles the dual role; cross-check the `bevy_enhanced_inputs` example in lightyear).
- `CollisionLayer` enum (in `src/game/player.rs`) won't replicate as-is (PhysicsLayer derive). Verify this doesn't break replication; it's only used at spawn time so should be fine.

## 11. Verification sequence (after networking lands)

```
just check-all                     # all feature combos
just smoke-test                    # host-client 300 frames

# Two-terminal real test:
cargo run --no-default-features --features server -- server
# (in another terminal)
cargo run --no-default-features --features client -- client --client-id 1
# Expected: client window shows player, WASD/mouse moves it,
# server logs `Client connected, spawning LogicalPlayer`.

# Multi-client smoke:
# Same as above plus a second client with --client-id 2 — both clients should
# see each other as colored cylinders.
```

## 12. References

- https://github.com/aidenstern/lightyear/tree/avian-0.6/examples/avian_3d_character (avian + leafwing pattern)
- https://github.com/aidenstern/lightyear/tree/avian-0.6/examples/bevy_enhanced_inputs (bei integration pattern)
- https://github.com/aidenstern/lightyear/tree/avian-0.6/lightyear_inputs_bei/src (bei integration internals — `setup.rs`, `marker.rs`, `input_message.rs`)
- `~/dev/bevy/bevy_game/src/{main,client,server}.rs` (working leafwing reference with LogicalPlayer/RenderPlayer split)
- `~/dev/bevy/bevy_game/CLAUDE.md` (architecture doc that mirrors what bevy_ahoy will look like)
