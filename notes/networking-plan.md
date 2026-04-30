# Bevy Ahoy Networking Plan v2 — `lightyear_ahoy` Integration

This **supersedes** the prior plan (which was written against `aidenstern/lightyear@avian-0.6` + a hand-rolled bei integration). The new plan piggybacks on `lightyear_ahoy`, a small networking-glue crate that andriyDev already wrote specifically to make `bevy_ahoy` server-authoritative under lightyear.

`lightyear_ahoy` location (sibling of this repo):
- `~/dev/bevy/bevy_ahoy_testing/lightyear_ahoy/` — local clone.

`lightyear_ahoy` pins:
- `bevy_ahoy = { git = ".../andriyDev/bevy_ahoy", rev = "1ac0fed" }` (the `networked` branch).
- `lightyear = { git = ".../baszalmstra/lightyear", rev = "9f90deca6" }` — **different** from our existing `aidenstern/lightyear` pin.
- `avian3d = "0.6.1"` (we are currently on `0.6.0-rc.1`).
- `bevy = "0.18"`.

## Strategic decisions

### 1. Throw out the old custom networking work

Drop:
- `src/networking.rs` (replaced by `lightyear_ahoy::protocol::ProtocolPlugin`).
- The old `notes/networking-plan.md` content (this file replaces it).
- The `lightyear` direct dep on `aidenstern/lightyear`.
- The `bevy_transform_interpolation` patch.
- The `client`/`server`/`networking`/`serialize` Cargo feature scaffolding (simplified — see §3).

### 2. Use `baszalmstra/lightyear` (not `aidenstern/lightyear`)

`lightyear_ahoy` was built against `baszalmstra/lightyear@9f90deca6`. We must use the same lightyear or we get type incompatibilities (different `LightyearAvianPlugin`, different `InputPlugin`, etc.). We will:
- Point `Cargo.toml` directly at `baszalmstra/lightyear` at the same rev.
- Leave `aidenstern/lightyear` alone for now. If we ever need bevy_ahoy-specific lightyear changes, we can fork `baszalmstra` instead.

### 3. Camera/look semantics: adopt the `networked` branch's pattern

andriyDev's `networked` branch made one subtle but important change to `bevy_ahoy`'s camera layer that our fork hasn't picked up yet:

| | This fork (now) | `andriyDev/networked` |
|-|---|---|
| `rotate_camera` writes to | KCC `Transform.rotation` **and** camera `Transform.rotation` | camera `Transform.rotation` only |
| `spin_*` (platform spin) writes to | `Transform.rotation` (`spin_kcc`) | `CharacterLook` (`spin_character_look`) |
| Camera follows | KCC yaw via `copy_kcc_yaw_to_camera` (in `RunFixedMainLoop`) | `CharacterLook` via `copy_character_look_to_camera` (in `Update` + `PostUpdate`) |
| `AhoySystems::UpdateCameras` | does not exist | exists; camera-sync systems run inside it in `PostUpdate` |

`lightyear_ahoy::avian::SimpleAvianSetupPlugin` chains:
```
PostUpdate: (RollbackSystems::VisualCorrection, FrameInterpolationSystems::Interpolate) → AhoySystems::UpdateCameras
```
so camera updates run **after** interpolation/correction. That set must exist for the chain to compile.

Plan: port the `networked` branch's camera + spin + AhoySystems changes wholesale into our fork. **Net behavioral effect**: the KCC body Transform's yaw stops mirroring the camera. There is no body mesh in this POC, so this is invisible. If we later add a body mesh we re-derive its yaw from `CharacterLook` (one new system).

### 4. bei is client-side only; lightyear's native `InputPlugin` is the wire format

- `bevy_enhanced_input` (bei) maps physical input → `Fire<Action>` events on the client.
- Existing `apply_*` observers in `src/input.rs` translate `Fire<Action>` events into `AccumulatedInput` mutations on the client's local player. **Unchanged.**
- `lightyear_ahoy::protocol::ProtocolPlugin` registers `InputPlugin::<AccumulatedInput>` and `InputPlugin::<CharacterLook>`. These replicate the input components client → server as `ActionState<…>`.
- `lightyear_ahoy::client::ClientPlugin` copies `AccumulatedInput → ActionState` (write side, on the local player only) and copies `ActionState → AccumulatedInput` during rollback (so re-simulation reads the buffered tick's input). Same for `CharacterLook`.
- `lightyear_ahoy::server::ServerPlugin` copies `ActionState → AccumulatedInput` for remote players (so the KCC's normal input read still works).
- The server **never** runs bei. `EnhancedInputPlugin` and the `PlayerInput` bei context Component go on the **client only**.

### 5. The kcc/grounded entity ref problem — already solved by `lightyear_ahoy`

`CharacterControllerState.grounded: Option<MoveHitData>` contains an `Entity` ref to the ground collider. Across server↔client this is a different `Entity` value, but lightyear's `MapEntities` rewrites it. We already derive `MapEntities` on `CharacterControllerState`.

The remaining issue (per andriyDev's note): if the rolled-back state's grounded entity doesn't exist client-side, lightyear's default `should_rollback` would mismatch and force unnecessary rollback. Solution: `lightyear_ahoy::protocol::character_controller_state_should_rollback` ignores the entity ref entirely — only compares `is_some` plus the scalar fields. So replicated grounded data is "advisory" for the client; client recomputes the ground entity from its own physics during rollback. Acceptable for a POC.

### 6. Throw away the structural-prep `notes/`

`notes/networking-plan.md` (this file) replaces the old one. Drop the references to `~/.claude/plans/floating-watching-kurzweil.md` and `~/dev/bevy/bevy_game/`.

## Implementation phases

### Phase 1 — Library API changes (`my_bevy_ahoy_fork/bevy_ahoy/src/`)

Goal: bring the lib's surface into agreement with what `lightyear_ahoy` expects (the API of `andriyDev/bevy_ahoy@1ac0fed`).

#### 1a. `lib.rs`
- Add `UpdateCameras` to the `AhoySystems` enum:
  ```rust
  pub enum AhoySystems {
      MoveCharacters,
      ApplyForcesToDynamicRigidBodies,
      UpdateCameras,
  }
  ```
- No other lib.rs changes — `CharacterController`, `CharacterControllerState`, `CharacterLook`, etc. already match.

#### 1b. `camera.rs`
Port from `andriyDev/networked@1ac0fed:src/camera.rs`. Key diffs:
- `rotate_camera` and `yank_camera` write to camera `Transform.rotation` only, not KCC `Transform.rotation`.
- New `snap_camera_position_to_kcc_on_add` and `snap_camera_rotation_to_kcc_on_add` observers (so a camera added to an already-existing KCC immediately syncs).
- Drop `copy_kcc_yaw_to_camera` (the user-fork addition that mirrored kcc.Transform.yaw → camera.Transform.yaw).
- Move `copy_character_look_to_camera` and `sync_camera_transform` to `PostUpdate` inside `AhoySystems::UpdateCameras`, which is configured `before(TransformSystems::Propagate)`.
- `CharacterControllerCameraOf::on_add` reads `Position`/`Rotation` from KCC (not just `Transform`) and snaps the camera transform to those.

#### 1c. `kcc.rs`
- Rename `spin_kcc` → `spin_character_look` and change its body to operate on `&mut CharacterLook` instead of `&mut Transform`. Direct port from `andriyDev/networked@1ac0fed:src/kcc.rs`.
- The `AhoyKccPlugin::build` registration becomes:
  ```rust
  .add_systems(Update, (spin_character_look,))
  ```
- Update the camera.rs reference: `kcc::spin_character_look` (was `kcc::spin_kcc`).

#### 1d. `networking.rs`
**Delete this file.** lightyear plugin registration moves to `lightyear_ahoy::protocol::ProtocolPlugin`.

Remove the `pub mod networking` line and the `pub use crate::networking::AhoyNetworkingPlugin` re-export from `lib.rs`.

#### 1e. `Cargo.toml` (lib half)
- Bump `avian3d = "0.6.0-rc.1"` → `"0.6.1"`. Verify `0.6.1` doesn't introduce compile errors in `kcc.rs` (likely fine — it's a patch bump).
- Drop the `lightyear` direct dep.
- Drop the `bevy_transform_interpolation` patch.
- Simplify features:
  - Keep `serialize = ["dep:serde", "avian3d/serialize", "bevy_math/serialize"]`.
  - Drop `networking`. Drop the `dep:lightyear` optional dep.

The library no longer carries any lightyear knowledge directly — that all lives in `lightyear_ahoy`.

### Phase 2 — Binary `Cargo.toml` and feature flags

Treat `Cargo.toml` as having two halves: lib deps (above) and binary deps (here).

#### 2a. Add deps
```toml
lightyear_ahoy = { path = "../../lightyear_ahoy" }
lightyear = { git = "https://github.com/baszalmstra/lightyear.git", rev = "9f90deca6a23e7c98028d14c98f0961e4135902d", features = [
    "avian3d",
    "frame_interpolation",
    "prediction",
    "input_native",
    "netcode",
    "udp",
    "client",
    "server",
] }
```
The `client`/`server`/`netcode`/`udp` features are needed to spawn the lightyear `Client`/`NetcodeServer`/`UdpIo` entities in our `host`/`client`/`server` boot code. `avian3d`, `prediction`, `input_native`, `frame_interpolation` are required by `lightyear_ahoy` itself.

#### 2b. Patch lightyear_ahoy's bevy_ahoy ref to point at our fork
```toml
[patch."https://github.com/andriyDev/bevy_ahoy"]
bevy_ahoy = { path = "." }
```
Without this, `lightyear_ahoy` would compile against `andriyDev/bevy_ahoy@1ac0fed` AND we'd compile against our local fork — two different `bevy_ahoy` crate instances, types wouldn't match. The patch forces lightyear_ahoy to use our fork.

#### 2c. Drop the `aidenstern/lightyear` dep and `[patch.crates-io]` for bevy_transform_interpolation.

#### 2d. Simplify features
```toml
[features]
default = ["client", "server"]   # host-client mode
client = []                       # gates per-mode binary code
server = []
```
Drop `networking` and `serialize` from the binary. The lib still has `serialize`; we propagate as `bevy_ahoy/serialize` if we want it (lightyear_ahoy already requires it).

Final binary feature math:
```toml
[features]
default = ["client", "server"]
client = ["bevy_ahoy/serialize"]
server = ["bevy_ahoy/serialize"]
```

The `compile_error!` in `src/game/mod.rs` (zero-feature build fails) stays.

### Phase 3 — Game wiring (`src/game/`)

#### 3a. `mod.rs` — App construction
Per andriyDev's instructions, all four `lightyear_ahoy` plugins are added unconditionally (in any mode):
```rust
app.add_plugins((
    lightyear_ahoy::avian::SimpleAvianSetupPlugin,
    lightyear_ahoy::protocol::ProtocolPlugin,
    lightyear_ahoy::client::ClientPlugin,
    lightyear_ahoy::server::ServerPlugin,
));
```
Replace the existing `PhysicsPlugins::default()` with the disabled-subplugin variant per andriy:
```rust
PhysicsPlugins::default()
    .build()
    .disable::<PhysicsTransformPlugin>()
    .disable::<PhysicsInterpolationPlugin>(),
```

Per-mode plugin sets:
- **server-only build**: drop `DefaultPlugins`'s render/window/audio bits — use `MinimalPlugins` + `TransformPlugin` + `AssetPlugin` + `ImagePlugin` + `MeshPlugin` + `ScenePlugin` + `StatesPlugin` + `GltfPlugin` + `init_asset::<StandardMaterial>()`. Drop `MipmapGeneratorPlugin`, `FramepacePlugin`, `EnhancedInputPlugin`, `BindingsPlugin`, `DebugPlugin`, `CursorPlugin`, `VisualsPlugin`, `SetupPlugin`. Keep `ScenePlugin` (game scene), `PlayerPlugin`, `AhoyPlugins`, the four lightyear_ahoy plugins, `ServerPlugin` (our binary's, see 3c), and a `Startup` system that boots a `NetcodeServer` and triggers `Start`.
- **client-only build**: keep `DefaultPlugins`, `EnhancedInputPlugin`, all visual plugins. Drop `SetupPlugin` (no local spawn). Add our `ClientPlugin` (3d) and a `Startup` system that boots a lightyear `Client` and triggers `Connect`.
- **host-client (default)**: same as client-only plus `ServerPlugin` and `HostPlugin` (3e). Drop `SetupPlugin`. Local player gets created via the host-client connect flow.

The mode-dispatch in `mod.rs` already exists; we just fill in the per-mode plugins and Startup systems.

#### 3b. `bindings.rs` — strip the on_add hook
- Remove `#[component(on_add = PlayerInput::on_add)]` on `PlayerInput`.
- Move the `actions!(...)` call from `PlayerInput::on_add` into `client::setup_local_player` (3d).
- `PlayerInput` becomes `#[derive(Component, Default)] pub struct PlayerInput;`.
- `BindingsPlugin` still calls `app.add_input_context::<PlayerInput>();`.

#### 3c. `server.rs` — implement `ServerPlugin`
```rust
use std::time::Duration;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_ahoy::prelude::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use crate::game::player::{LogicalPlayer, PlayerId, CollisionLayer};
use crate::game::scene::SPAWN_POINT;

const SEND_INTERVAL: Duration = Duration::from_millis(100);

pub struct ServerPlugin;
impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(handle_new_link)
           .add_observer(handle_connected);
    }
}

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
    commands.spawn((
        LogicalPlayer,
        PlayerId(client_id.to_bits()),
        Position(SPAWN_POINT),
        Rotation::default(),
        CharacterLook::default(),
        CharacterController::default(),
        RigidBody::Kinematic,
        Collider::cylinder(0.7, 1.8),
        Mass(90.0),
        CollisionLayers::new(CollisionLayer::Player, LayerMask::ALL),
        Replicate::to_clients(NetworkTarget::All),
        PredictionTarget::to_clients(NetworkTarget::Single(client_id)),
        InterpolationTarget::to_clients(NetworkTarget::AllExceptSingle(client_id)),
        ControlledBy { owner: trigger.entity, lifetime: Default::default() },
    ));
}

pub fn start_dedicated_server(mut commands: Commands, bind_addr: SocketAddr) {
    let server = commands.spawn((
        lightyear::netcode::NetcodeServer::new(NetcodeConfig::default()),
        LocalAddr(bind_addr),
        ServerUdpIo::default(),
    )).id();
    commands.trigger(Start { entity: server });
}
```
The `respawn_below_floor` system already exists in `src/game/player.rs` and works server-side automatically once the server is the one moving the player.

#### 3d. `client.rs` — implement `ClientPlugin`
```rust
use std::{net::{IpAddr, Ipv4Addr, SocketAddr}, time::Duration};
use bevy::prelude::*;
use bevy_ahoy::prelude::*;
use bevy_enhanced_input::prelude::{Press, Hold, *};
use lightyear::prelude::*;
use lightyear::prelude::client::*;
use lightyear::prelude::input::native::{InputMarker};
use lightyear::input::native::ActionState;
use crate::game::{
    bindings::PlayerInput,
    player::{LogicalPlayer, PlayerId, RenderPlayer},
};

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
        // Tag the entity as "this is the locally-driven input" so lightyear's
        // input layer reads our AccumulatedInput / CharacterLook into ActionState.
        commands.entity(entity).insert((
            PlayerInput,
            InputMarker::<AccumulatedInput>::default(),
            InputMarker::<CharacterLook>::default(),
            actions!(PlayerInput[
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
            ]),
        ));
        commands.spawn((
            Camera3d::default(),
            CharacterControllerCameraOf::new(entity),
            RenderPlayer { logical_entity: entity },
        ));
    }
}

fn setup_remote_player(
    mut commands: Commands,
    new_remote: Query<
        (Entity, &PlayerId),
        (With<Interpolated>, With<LogicalPlayer>, Without<RemotePlayerInitialized>),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, player_id) in &new_remote {
        let hue = ((player_id.0 * 137) % 360) as f32;
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

pub fn start_client(mut commands: Commands, client_id: u64, server_addr: SocketAddr) {
    let auth = Authentication::Manual {
        server_addr,
        client_id,
        private_key: lightyear::netcode::Key::default(),
        protocol_id: 0,
    };
    let client = commands.spawn((
        Client::default(),
        LocalAddr(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)),
        PeerAddr(server_addr),
        Link::new(None),
        ReplicationReceiver::default(),
        ReplicationSender::new(SEND_INTERVAL, SendUpdatesMode::SinceLastAck, false),
        lightyear::netcode::NetcodeClient::new(auth, NetcodeConfig::default()).unwrap(),
        UdpIo::default(),
    )).id();
    commands.trigger(Connect { entity: client });
}
```

#### 3e. `host.rs` — implement `HostPlugin`
```rust
use bevy::prelude::*;
use lightyear::prelude::*;
use lightyear::prelude::server::*;

pub const DEFAULT_HOST_BIND: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5000);

pub struct HostPlugin;
impl Plugin for HostPlugin {
    fn build(&self, app: &mut App) {
        let server = app.world_mut().spawn((
            lightyear::netcode::NetcodeServer::new(NetcodeConfig::default()),
            LocalAddr(DEFAULT_HOST_BIND),
            ServerUdpIo::default(),
            Name::new("HostServer"),
        )).id();
        let client = app.world_mut().spawn((
            Client::default(),
            Name::new("HostClient"),
            LinkOf { server },        // shares process link, no UDP
        )).id();
        app.add_systems(Startup, (
            move |mut c: Commands| { c.trigger(Start { entity: server }); },
            move |mut c: Commands| { c.trigger(Connect { entity: client }); },
        ).chain());
    }
}
```

#### 3f. `setup.rs`
Delete. The local player no longer spawns from a `OnEnter(GameState::InGame)` system — it's spawned by `server::handle_connected` and the host-client receives it via prediction.

Remove `SetupPlugin` from `mod.rs`'s plugin list.

#### 3g. `player.rs`
- `LogicalPlayer`, `PlayerId`, `RenderPlayer`, `CollisionLayer`, `respawn_below_floor` — keep as-is.
- Add `LogicalPlayer` to lightyear's component registry. We can do that in our own per-binary registration helper, OR rely on `Replicate::to_clients` to register on first spawn. Confirm during implementation; if it doesn't auto-register, add an `app.register_component::<LogicalPlayer>().add_prediction()` call in our binary's `ClientPlugin`/`ServerPlugin` (NOT in the lib).
- Same for `PlayerId`.

#### 3h. `cli.rs`, `cursor.rs`, `debug.rs`, `visuals.rs`, `scene.rs`
No changes.

### Phase 4 — Verify

```
cd my_bevy_ahoy_fork/bevy_ahoy
cargo check                                 # default = host-client
cargo check --no-default-features --features client
cargo check --no-default-features --features server
just lint                                    # cargo clippy -D warnings
just smoke-test                              # 300 frames host-client, exits 0

# Two terminals:
just run-server                              # listens on 0.0.0.0:5000
just run-client --client-id 1 --server-addr 127.0.0.1:5000   # window opens, WASD moves player

# Three terminals (multi-client):
just run-server
just run-client --client-id 1
just run-client --client-id 2
# Both clients should see each other as colored cylinders.
```

### Phase 5 — Update `CLAUDE.md`

Replace the "What's done vs what's next" + "Networking architecture" sections to reflect the new lightyear_ahoy-based reality. Drop references to `~/dev/bevy/bevy_game/`, `aidenstern/lightyear`, and the bei-driven replication plan. Add a short "How replication actually works" section pointing at lightyear_ahoy.

## Open questions / things to verify during implementation

1. **`avian3d` 0.6.0-rc.1 → 0.6.1 compile**: a patch bump but pre-1.0 sometimes breaks. If it does, we either patch our kcc.rs or pin lightyear_ahoy to the same `0.6.0-rc.1` (would require modifying lightyear_ahoy locally — its `path = ` dep gives us that option).

2. **`MapEntities` derive on `AccumulatedInput`/`CharacterLook` from inside the lightyear input pipeline**: the input replication path may or may not call `MapEntities`. `CharacterLook` has no Entity refs so it's fine; `AccumulatedInput` similarly has none.

3. **Server collider hierarchy on `MapRoot`**: `add_map_colliders` queries meshes loaded into `Assets<Mesh>`. On a `MinimalPlugins` server we need `bevy_gltf::GltfPlugin` AND `MeshPlugin` AND `ScenePlugin` in the plugin list, otherwise the GLB won't hydrate. Verify which subset is actually needed.

4. **`PhysicsTransformPlugin` and `PhysicsInterpolationPlugin` disable**: confirm these are still the right plugins to disable on this avian + lightyear combo (lightyear_ahoy README/comments will say). The `lightyear_ahoy::avian::SimpleAvianSetupPlugin` already adds `LightyearAvianPlugin` and `FrameInterpolationPlugin::<Transform>`, so we just need to disable the avian-side equivalents.

5. **`LogicalPlayer`/`PlayerId` registration**: do they need explicit `register_component` calls, or does `Replicate::to_clients` on first spawn handle it? If explicit needed, where — in the lib or in the game binary?

6. **`InputMarker<AccumulatedInput>`/`InputMarker<CharacterLook>`**: confirm the lightyear native InputPlugin spawns these on the local player automatically, or whether `setup_local_player` has to add them (the snippet above adds them defensively; remove if redundant).

7. **`Cargo.lock` lightyear unification**: lightyear_ahoy will pull `baszalmstra/lightyear@9f90deca6` from its own `Cargo.toml`. Our binary depends directly on `baszalmstra/lightyear@9f90deca6`. Cargo should de-dup these because the git rev matches. Verify with `cargo tree -d` after the first build.

## Reference files (already cloned locally)

- `~/dev/bevy/bevy_ahoy_testing/lightyear_ahoy/src/{avian,protocol,client,server}.rs` — the integration we're consuming.
- `~/dev/bevy/bevy_ahoy_testing/bevy_ahoy/src/{lib,camera,kcc,input}.rs` (on `networked` branch) — the source code we're porting `AhoySystems::UpdateCameras` and `spin_character_look` from.
