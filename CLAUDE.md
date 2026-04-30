# CLAUDE.md

Guidance for Claude Code working in this repo.

## Project Overview

`bevy_ahoy` started as janhohenheim's first-person Kinematic Character Controller library for Bevy 0.18 + Avian 0.6 + bevy_enhanced_input 0.24. This fork has been evolved into a server-authoritative multiplayer game using lightyear (`baszalmstra/lightyear` @ rev `9f90deca6`).

The repo is **both a library and a game binary** in one Cargo package:
- `src/lib.rs` + `src/{camera,kcc,input,dynamics,fixed_update_utils,water}.rs` — the kcc library (still publishable as `bevy_ahoy`).
- `src/main.rs` — three-line binary entry point.
- `src/game/` — the game built on top of the library (CLI, scene, player components, debug HUD, visuals).
- `src/game/networking/` — vendored `lightyear_ahoy` glue: avian replication, protocol, client/server input plumbing.

## Run modes

```
just run                # host-client (server + client in one process). Default.
just smoke-test         # host-client for 300 frames, auto-exits 0.

just run-server         # cargo run --no-default-features --features server -- server
just run-client 1       # cargo run --no-default-features --features client -- client --client-id 1
```

## Cargo features

- `default = ["client", "server"]` → host-client mode.
- `client = ["serialize"]` — gates client-mode game code (`src/game/client.rs`, `host.rs`).
- `server = ["serialize"]` — gates server-mode game code (`src/game/server.rs`, `host.rs`).
- `serialize = ["dep:serde", "avian3d/serialize", "bevy_math/serialize"]` — enables `Serialize`/`Deserialize` derives on the lib's components so they can be replicated.

`cargo check --no-default-features` (zero features) intentionally fails with a `compile_error!` in `src/game/mod.rs` — at least one of `client`/`server` must be on.

## Module layout

### Library (`src/`, in the `bevy_ahoy` lib crate)
- `lib.rs` — `AhoyPlugins` plugin group, `AhoySystems` (`MoveCharacters` / `ApplyForcesToDynamicRigidBodies` / `UpdateCameras`), `CharacterController` (knobs), `CharacterLook` (yaw/pitch), `CharacterControllerState`, `CharacterControllerOutput`, `CharacterControllerDerivedProps`, prelude.
- `camera.rs` — `CharacterControllerCameraOf` relationship for first-person camera follow. The camera transform is the **only** writer of yaw input (`rotate_camera`/`yank_camera`); `copy_camera_to_character_look` (in `RunFixedMainLoop`) pulls camera yaw into `CharacterLook`. The reverse path (`copy_character_look_to_camera`) and `sync_camera_transform` run in `PostUpdate` under `AhoySystems::UpdateCameras`, which `lightyear_ahoy::avian::SimpleAvianSetupPlugin` chains *after* `RollbackSystems::VisualCorrection` and `FrameInterpolationSystems::Interpolate`.
- `input.rs` — bei `InputAction` types (`Movement`, `Jump`, `RotateCamera`, `Crouch`, `Mantle`, `Tac`, `Crane`, `Climbdown`, `SwimUp`, `YankCamera`) + `AccumulatedInput` (replicated as input via lightyear) + `apply_*` Fire observers.
- `kcc.rs` — the kcc move-and-slide implementation. `spin_character_look` rotates `CharacterLook` (not the kcc `Transform`) when standing on a spinning platform.
- `dynamics.rs` — applies impulses to dynamic rigid bodies the controller touches.
- `water.rs` — **inert** type defs (water gameplay was removed; types kept because kcc.rs queries them — never populated at runtime).

### Game (`src/game/`, in the binary)
- `mod.rs` — `pub fn run() -> AppExit`. Builds the `App`, parses CLI, adds Bevy `DefaultPlugins`, Avian (`PhysicsPlugins` with `PhysicsTransformPlugin`/`PhysicsInterpolationPlugin` disabled — lightyear's avian glue replaces them), `lightyear::prelude::client::ClientPlugins` and/or `lightyear::prelude::server::ServerPlugins`, the four `networking::*` plugins, `AhoyPlugins`, visual + input plugins, then dispatches per-mode boot.
- `cli.rs` — clap CLI: `Server { bind_addr }` / `Client { client_id, server_addr }` subcommands, host-client when no subcommand.
- `scene.rs` — `playground.glb` load, `add_map_colliders` (waits for meshes loaded → adds `ColliderConstructorHierarchy`), `SPAWN_POINT`.
- `player.rs` — `LogicalPlayer` (replicated marker), `PlayerId(u64)` (replicated stable id), `RenderPlayer { logical_entity }` (client-side camera→logical link), `CollisionLayer`, `respawn_below_floor` (Y < -50 → SPAWN_POINT).
- `bindings.rs` — `BindingsPlugin` (registers the bei `PlayerInput` context). The `actions!` invocation lives in `client::setup_local_player` — the server never touches bei.
- `debug.rs` — `DebugInput` bei context, `Reset`/`ToggleDebug` actions, debug HUD text.
- `cursor.rs` — click to capture / Esc to release.
- `visuals.rs` — `tweak_camera`/`tweak_directional_light` observers (Atmosphere, Bloom, EnvMap, sun rotation), mipmap gen, material roughness, crosshair UI.
- `server.rs` — `ServerPlugin`: `handle_new_link` adds `ReplicationSender`/`ReplicationReceiver` per client; `handle_connected` spawns `LogicalPlayer` + physics bundle + `Replicate::to_clients(All)` + `PredictionTarget::Single(client)` + `InterpolationTarget::AllExceptSingle(client)` + `ControlledBy { owner: linkof_entity }`. Also `spawn_server`/`start_server` for boot.
- `client.rs` — `ClientPlugin`: `setup_local_player` (filter `(Added<Predicted>, With<LogicalPlayer>, Has<Controlled>)` → attach `CharacterController`/`Collider`/`RigidBody` for prediction + bei `actions!` + `InputMarker<AccumulatedInput>`/`InputMarker<CharacterLook>` + camera + `RenderPlayer`); `setup_remote_player` (interpolated remotes get a colored cylinder mesh). Also `spawn_client`/`start_client` for boot.
- `host.rs` — `HostPlugin`: spawns the server entity at app construction time, plus a `Client` with `LinkOf { server }` (no UDP — shares process link). `Startup` chain: `(start_server, start_client)`.

### Networking (`src/game/networking/`, vendored from `lightyear_ahoy`)
- `avian.rs` — `SimpleAvianSetupPlugin`: registers `Position`/`Rotation`/`LinearVelocity`/`AngularVelocity` for prediction with rollback thresholds, correction, and visual interpolation. Adds `lightyear::avian3d::LightyearAvianPlugin` + `FrameInterpolationPlugin::<Transform>`. Configures the `PostUpdate` chain so visual correction + frame interpolation happen before `AhoySystems::UpdateCameras`.
- `protocol.rs` — `ProtocolPlugin`: registers `InputPlugin::<AccumulatedInput>` and `InputPlugin::<CharacterLook>` (lightyear's native input replication); registers `CharacterLook` and `CharacterControllerState` for prediction (the latter with `add_component_map_entities` because of its `grounded.entity` ref + a `should_rollback` that compares scalars only — the entity is left as advisory).
- `client.rs` — `ClientPlugin`: copies local `AccumulatedInput`/`CharacterLook` into `ActionState` for the local-input entity (`InputMarker<…>`-tagged) so they replicate to the server; on rollback, copies `ActionState` back into `AccumulatedInput`/`CharacterLook` for re-simulation. Includes a workaround observer (`copy_interpolated_confirmed_to_real`) for [lightyear#1380](https://github.com/cBournhonesque/lightyear/issues/1380).
- `server.rs` — `ServerPlugin`: copies received `ActionState<AccumulatedInput>` into `AccumulatedInput` for remote-controlled entities (entities without `InputMarker`).

## How replication actually works

1. **Player connects** — server's `handle_new_link` adds `ReplicationSender`+`ReplicationReceiver` to the new `LinkOf` entity. `handle_connected` then spawns a `LogicalPlayer` with full replication targets and a `ControlledBy { owner: linkof_entity }`.
2. **Server → client** — `Replicate::to_clients(All)` pushes Position/Rotation/LinearVelocity/AngularVelocity/CharacterLook/CharacterControllerState to all clients. The owning client gets a `Predicted` copy; everyone else gets an `Interpolated` copy.
3. **Local player setup (client)** — `Added<Predicted> + With<LogicalPlayer> + Has<Controlled>` triggers `setup_local_player`, which attaches the local-only physics bundle, bei bindings, and `InputMarker<AccumulatedInput>`/`InputMarker<CharacterLook>`.
4. **Local input collection** — bei observers in `lib/src/input.rs` fill `AccumulatedInput`. The client also writes `CharacterLook` from camera mouse input.
5. **Client → server (input)** — `lightyear_ahoy::client::ClientPlugin` copies `AccumulatedInput → ActionState<AccumulatedInput>` (and same for `CharacterLook`). Lightyear's input plugin replicates these `ActionState`s up to the server.
6. **Server runs the kcc** — `lightyear_ahoy::server::ServerPlugin` copies the received `ActionState → AccumulatedInput` for each remote player; the server's `AhoyKccPlugin` simulates against that input. The kcc reads `CharacterLook` to set `CharacterControllerState.orientation`.
7. **Client predicts locally** — the predicted local player runs the same kcc against its local `AccumulatedInput`. Replicated state from the server triggers rollback when prediction diverges (per `position_should_rollback` etc.), with `character_controller_state_should_rollback` ignoring the `grounded.entity` field (entity refs differ between server and client).

## Schedule

- bei `Update` runs in `PreUpdate`, fires `Fire<Action>` events.
- `apply_*` observers (in `lib/src/input.rs`) write into `AccumulatedInput`.
- `AhoyKccPlugin` runs the kcc move-and-slide in `FixedPostUpdate`.
- `clear_accumulated_input` runs after each `FixedMainLoop` to reset.
- Camera transform sync (`sync_camera_transform`, `copy_character_look_to_camera`) runs in `PostUpdate` inside `AhoySystems::UpdateCameras`, **after** `FrameInterpolationSystems::Interpolate` (so the camera follows the interpolated transform).

## Common commands

```
cargo check                          # default features (host-client)
just check-all                       # all valid feature combos
just smoke-test                      # 300-frame run
just run                             # host-client play
just run-vulkan                      # WSL2 + NVIDIA: WGPU_BACKEND=vulkan cargo run
just lint                            # clippy with -D warnings
```

## What's done vs what's next

**Networking (done)**: full server-authoritative POC. Server-side player spawn, replicated state (Position/Rotation/Velocity/Look/State), input replication via lightyear's native `InputPlugin<AccumulatedInput>` + `InputPlugin<CharacterLook>`, predicted local player + interpolated remotes, host-client mode, dedicated server, dedicated client.

**Future polish**:
- Multi-client visual verification (run two clients against a dedicated server and confirm both render the other's cylinder).
- Headless server: drop `DefaultPlugins` for `MinimalPlugins` + asset/mesh/scene/gltf in `src/game/mod.rs` for the `Server` mode branch.
- Better `PlayerId` derivation (currently grabs the inner u64 from `PeerId::Netcode`/`Local`/`Steam`/`Entity`; non-netcode connections fall back to 0).
- Tune `InputDelayConfig` / send interval for production network conditions.
- The "ground entity" desync caveat: when client and server pick different ground entities, `character_controller_state_should_rollback` ignores the divergence — visually fine, but a re-run on the client picks up the wrong ground reference for one tick. Optionally: recompute the ground entity at rollback time (see `notes/networking-plan.md` §"Open questions").
