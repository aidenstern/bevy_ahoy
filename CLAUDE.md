# CLAUDE.md

Guidance for Claude Code working in this repo.

## Project Overview

`bevy_ahoy` started as janhohenheim's first-person Kinematic Character Controller library for Bevy 0.18 + Avian 0.6 + bevy_enhanced_input 0.24. This fork is being evolved into a server-authoritative multiplayer game using lightyear (`aidenstern/lightyear` @ `avian-0.6` branch).

The repo is **both a library and a game binary**:
- `src/lib.rs` + `src/{camera,kcc,input,dynamics,fixed_update_utils,water,networking}.rs` — the kcc library (publishable as `bevy_ahoy`).
- `src/main.rs` — three-line binary entry point.
- `src/game/` — the game built on top of the library.

The reference implementation for the networking architecture is `~/dev/bevy/bevy_game/` (uses leafwing-input-manager rather than bei, but same overall pattern).

## Run modes

```
just run                # host-client (server + client in one process). Default.
just smoke-test         # host-client for 300 frames, auto-exits 0.

# Per-mode (will only differ once networking is wired):
just run-server         # cargo run --no-default-features --features server -- server
just run-client 1       # cargo run --no-default-features --features client -- client --client-id 1
```

## Cargo features

- `default = ["client", "server"]` → host-client mode.
- `client = ["networking"]` — client gameplay code, gates `src/game/client.rs`.
- `server = ["networking"]` — server gameplay code, gates `src/game/server.rs`.
- `networking = ["serialize", "dep:lightyear"]` — pulls in lightyear and bei integration.
- `serialize = ["dep:serde", "avian3d/serialize", "bevy_math/serialize"]`.

`cargo check --no-default-features` (zero features) intentionally fails with a `compile_error!` — at least one of `client`/`server` must be on.

## Module layout

### Library (`src/`)
- `lib.rs` — `AhoyPlugins` plugin group, `CharacterController` (the big knobs struct), prelude.
- `camera.rs` — `CharacterControllerCameraOf` relationship for first-person camera follow.
- `input.rs` — bei `InputAction` types (`Movement`, `Jump`, `RotateCamera`, `Crouch`, `Mantle`, `Tac`, `Crane`, `Climbdown`, `SwimUp`, `YankCamera`) + `AccumulatedInput` + `apply_*` Fire observers.
- `kcc.rs` — the actual character controller move-and-slide implementation.
- `dynamics.rs` — applies impulses to dynamic rigid bodies the controller touches.
- `water.rs` — **inert** type defs (water gameplay was removed; types kept because kcc.rs queries them — never populated at runtime).
- `networking.rs` — `AhoyNetworkingPlugin`: registers `Position`/`Rotation`/`LinearVelocity`/`CharacterLook` for prediction. Will be extended in the networking phase (see `notes/networking-plan.md`).

### Game (`src/game/`)
- `mod.rs` — `pub fn run() -> AppExit`, parses CLI, dispatches per mode, owns `GameState`.
- `cli.rs` — clap `Cli` with `Server { bind_addr }` / `Client { client_id, server_addr }` subcommands.
- `scene.rs` — `playground.glb` load, `add_map_colliders` (waits for meshes loaded → adds `ColliderConstructorHierarchy`), `SPAWN_POINT`.
- `player.rs` — `LogicalPlayer` (replicated marker), `PlayerId` (replicated id), `RenderPlayer` (camera-side ref to logical entity, client-only), `CollisionLayer`, `respawn_below_floor` (Y < -50 → SPAWN_POINT).
- `bindings.rs` — `PlayerInput` bei context Component, attaches keyboard/gamepad bindings via `on_add` hook. **Note**: networking phase removes the on_add hook; client-side observer attaches bindings only on `Added<Predicted> + Has<Controlled>`.
- `setup.rs` — single-player spawn (player + camera + light). Replaced by per-mode setup in the networking phase.
- `debug.rs` — `DebugInput` bei context, `Reset`/`ToggleDebug` actions, debug HUD text.
- `cursor.rs` — click to capture / Esc to release.
- `visuals.rs` — `tweak_camera`/`tweak_directional_light` observers (Atmosphere, Bloom, EnvMap, sun rotation), mipmap gen, material roughness, crosshair UI.
- `server.rs` — `ServerPlugin` (stub). Networking phase: `handle_new_link`/`handle_connected` observers + LogicalPlayer spawn + `start_dedicated_server`.
- `client.rs` — `ClientPlugin` (stub). Networking phase: `setup_local_player` (Added<Predicted>+Controlled → bindings + camera+RenderPlayer), `setup_remote_player` (Added<Interpolated> → mesh).
- `host.rs` — `HostPlugin` (stub). Networking phase: spawns server + host-client entity, chains `(Start, Connect)`.

### Networking architecture
See `notes/networking-plan.md` for the full lightyear/bei integration plan. Key points:
- `LightyearAvianPlugin` with `AvianReplicationMode::Position`, disabling `PhysicsTransformPlugin`/`PhysicsInterpolationPlugin`/`IslandPlugin`/`IslandSleepingPlugin`.
- bei integration via `lightyear::input::bei::InputPlugin::<PlayerInput>`. Requires lightyear's `input_bei` feature flag.
- Server spawns `LogicalPlayer` with `Replicate::to_clients(All)`, `PredictionTarget::Single(client_id)`, `InterpolationTarget::AllExceptSingle(client_id)`, `ControlledBy { owner: link_entity }`.
- Client `setup_local_player`: filter `(Added<Predicted>, With<LogicalPlayer>, Has<Controlled>)`, attach `actions!(...)` + camera + `RenderPlayer { logical_entity }`.
- Per-LinkOf entity needs **both** `ReplicationSender` AND `ReplicationReceiver` — bei Action entities are spawned client-side and replicated UP to the server.
- Client entity needs `ReplicationSender` for the same reason.

## Key components (from the lib)

- `CharacterController` — the big knobs (speeds, jump_height, gravity, friction, mantle/crane/climb config). Replicated with rollback in networking phase.
- `CharacterControllerState` — controller state (orientation, grounded, crouching, mantle progress, last-X stopwatches). Replicated.
- `CharacterControllerOutput` — transient frame-output (touching entities, mantle output). Replicated.
- `CharacterLook` — yaw/pitch. The camera writes into this; kcc reads it. Replicated.
- `AccumulatedInput` — input collected since last fixed update tick. Cleared each tick. Replicated.
- `WaterState` — defaulted; kcc still references it but never populated.

## Schedule

- bei `Update` runs in `PreUpdate`, fires `Fire<Action>` events.
- `apply_*` observers (in `lib/src/input.rs`) write into `AccumulatedInput`.
- `AhoyKccPlugin` runs the kcc move-and-slide in `FixedPostUpdate` (configurable via `AhoyPlugins::new(schedule)`).
- `clear_accumulated_input` runs after each `FixedMainLoop` to reset.
- Camera transform sync runs in `RunFixedMainLoop` (before/after fixed loop).

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

**Structural prep (done)**: water/pickup/NPC removed; main.rs split into `src/game/`; `client`/`server`/`networking` Cargo features added; clap CLI added; `LogicalPlayer`/`PlayerId`/`RenderPlayer` types defined; respawn-below-floor system; networking plan written to `notes/networking-plan.md`.

**Networking (next)**: see `notes/networking-plan.md`. Wire lightyear into `src/networking.rs` (bei InputPlugin, register components for prediction). Implement `ServerPlugin`, `ClientPlugin`, `HostPlugin`. Move `actions!` from `PlayerInput::on_add` into client-side `setup_local_player`.
