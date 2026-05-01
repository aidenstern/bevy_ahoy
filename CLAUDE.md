# CLAUDE.md

Guidance for Claude Code working in this repo.

## Project Overview

`bevy_game` is a server-authoritative multiplayer Bevy game built on lightyear
(`cBournhonesque/lightyear` branch `main`). The kinematic character controller
under `src/kcc/` is forked from
[`janhohenheim/bevy_ahoy`](https://github.com/janhohenheim/bevy_ahoy) and evolved
in place; we don't pretend to be a fork of that crate any more — the kcc is just
an internal module of the game now.

It's a single Cargo package (`bevy_game`) with a single binary entry point.
Internally the layout follows a foxtrot-style domain split.

## Run modes

```
just run                # host-client (server + client in one process). Default.
just smoke-test         # host-client for 300 frames, auto-exits 0.

just run-server         # cargo run --no-default-features --features server -- server
just run-client 1       # cargo run --no-default-features --features client -- client --client-id 1
```

## Cargo features

- `default = ["client", "server"]` → host-client mode.
- `client = ["dep:lightyear", "serialize"]` — gates `src/client/`. Pulls in lightyear.
- `server = ["dep:lightyear", "serialize"]` — gates `src/server/`. Pulls in lightyear.
- `serialize = ["dep:serde", "avian3d/serialize", "bevy_math/serialize"]` — turns on
  `Serialize`/`Deserialize` derives on replicated kcc components.

`cargo check --no-default-features` (zero features) intentionally fails with a
`compile_error!` in `src/lib.rs` — at least one of `client`/`server` must be on.
With both features off, lightyear is also off — the build is just the kcc + scene
+ shared protocol registration.

## Module layout

```
src/
├── main.rs                   # tiny entry: calls bevy_game::run()
├── lib.rs                    # pub fn run(); declares all modules; defines GameState
├── kcc/                      # the KCC (forked from janhohenheim/bevy_ahoy)
│   ├── mod.rs                # AhoyPlugins, AhoySystems, CharacterController, prelude
│   ├── controller.rs         # move-and-slide impl
│   ├── camera.rs             # CharacterControllerCameraOf
│   ├── input.rs              # bei InputAction types + AccumulatedInput
│   ├── dynamics.rs           # impulses to dynamic rigid bodies
│   ├── water.rs              # inert types kept for kcc internal queries
│   └── fixed_update_utils.rs # schedule helpers
├── shared/                   # always compiles
│   ├── mod.rs
│   ├── cli.rs                # clap CLI; Mode variants cfg-gated per side
│   ├── player.rs             # LogicalPlayer, PlayerId, CollisionLayer + PlayerPlugin
│   ├── scene.rs              # SPAWN_POINT, MapRoot, load_map, add_map_colliders
│   └── networking/
│       ├── mod.rs
│       ├── protocol.rs       # ProtocolPlugin (component registration for replication)
│       └── avian.rs          # SimpleAvianSetupPlugin (avian replication setup)
├── client/                   # #[cfg(feature = "client")]
│   ├── mod.rs                # ClientPlugin (aggregates) + pub use boot::*
│   ├── boot.rs               # spawn_client, start_client
│   ├── player.rs             # RenderPlayer + setup_local_player + setup_remote_player
│   ├── bindings.rs           # bei PlayerInput context
│   ├── cursor.rs             # mouse capture
│   ├── debug.rs              # debug HUD + reset/toggle inputs
│   ├── debug_net.rs          # ClientNetworkDebugPlugin (periodic state log)
│   ├── networking.rs         # client-side input replication into ActionState
│   └── visuals.rs            # atmosphere, bloom, env maps, crosshair
├── server/                   # #[cfg(feature = "server")]
│   ├── mod.rs                # ServerPlugin (aggregates) + pub use boot::*
│   ├── boot.rs               # spawn_server, start_server
│   ├── spawn.rs              # handle_new_link, handle_connected
│   ├── respawn.rs            # respawn_below_floor (server-authoritative)
│   ├── debug_net.rs          # ServerNetworkDebugPlugin
│   └── networking.rs         # server-side input plumbing (ActionState → AccumulatedInput)
└── host/                     # #[cfg(all(feature = "client", feature = "server"))]
    └── mod.rs                # HostPlugin: in-process server + client link
```

`crate::GameState` lives at the top of `src/lib.rs`.

## How replication actually works

1. **Player connects** — the server's `handle_new_link` (in `src/server/spawn.rs`) adds
   `ReplicationSender`+`ReplicationReceiver` to the new `LinkOf` entity.
   `handle_connected` then spawns a `LogicalPlayer` with replication targets and a
   `ControlledBy { owner: linkof_entity }`.
2. **Server → client** — `Replicate::to_clients(All)` pushes
   Position/Rotation/LinearVelocity/AngularVelocity/CharacterLook/CharacterControllerState
   to all clients. The owning client gets a `Predicted` copy; everyone else gets an
   `Interpolated` copy.
3. **Local player setup (client)** — `Added<Predicted> + With<LogicalPlayer>`
   triggers `setup_local_player` (in `src/client/player.rs`), which attaches the
   local-only physics bundle, bei bindings, and `InputMarker<AccumulatedInput>` /
   `InputMarker<CharacterLook>`.
4. **Local input collection** — bei observers in `src/kcc/input.rs` fill
   `AccumulatedInput`. The client also writes `CharacterLook` from camera mouse
   input.
5. **Client → server (input)** — `src/client/networking.rs` copies
   `AccumulatedInput → ActionState<AccumulatedInput>` (and same for `CharacterLook`).
   Lightyear's input plugin replicates these `ActionState`s up to the server.
6. **Server runs the kcc** — `src/server/networking.rs` copies the received
   `ActionState → AccumulatedInput` for each remote player; the server's
   `AhoyKccPlugin` simulates against that input. The kcc reads `CharacterLook` to
   set `CharacterControllerState.orientation`.
7. **Client predicts locally** — the predicted local player runs the same kcc
   against its local `AccumulatedInput`. Replicated state from the server triggers
   rollback when prediction diverges (per `position_should_rollback` etc.), with
   `character_controller_state_should_rollback` ignoring the `grounded.entity`
   field (entity refs differ between server and client).

## Schedule

- bei `Update` runs in `PreUpdate`, fires `Fire<Action>` events.
- `apply_*` observers (in `src/kcc/input.rs`) write into `AccumulatedInput`.
- `AhoyKccPlugin` runs the kcc move-and-slide in `FixedPostUpdate`.
- `clear_accumulated_input` runs after each `FixedMainLoop` to reset.
- Camera transform sync (`sync_camera_transform`, `copy_character_look_to_camera`)
  runs in `PostUpdate` inside `AhoySystems::UpdateCameras`, **after**
  `FrameInterpolationSystems::Interpolate` (so the camera follows the interpolated
  transform).

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

**Networking (done)**: full server-authoritative POC. Server-side player spawn,
replicated state (Position/Rotation/Velocity/Look/State), input replication via
lightyear's native `InputPlugin<AccumulatedInput>` + `InputPlugin<CharacterLook>`,
predicted local player + interpolated remotes, host-client mode, dedicated
server, dedicated client.

**Foxtrot-style restructure (done)**: collapsed the kcc out of a separately-named
lib into `src/kcc/`, moved game code into `src/{shared,client,server,host}/`,
tightened the per-side cfg discipline (each side's plugins, modules, and even
imports are gated), renamed the package to `bevy_game`, dropped the fork-style
metadata, made lightyear an optional dep that the `client`/`server` features pull
in.

**Future polish**:
- Multi-client visual verification (run two clients against a dedicated server and
  confirm both render the other's cylinder).
- Headless server: drop `DefaultPlugins` for `MinimalPlugins` + asset/mesh/scene/gltf
  in `src/lib.rs`'s `Server` mode branch.
- Better `PlayerId` derivation (currently grabs the inner u64 from `PeerId`'s
  `Netcode`/`Local`/`Steam`/`Entity` variants; non-netcode connections fall back
  to 0).
- Tune `InputDelayConfig` / send interval for production network conditions.
- The "ground entity" desync caveat: when client and server pick different ground
  entities, `character_controller_state_should_rollback` ignores the divergence —
  visually fine, but a re-run on the client picks up the wrong ground reference
  for one tick. Optionally: recompute the ground entity at rollback time (see
  `notes/networking-plan.md` §"Open questions").
- Both client and server independently run `ColliderConstructorHierarchy` on the
  loaded glb. Functionally fine (predicted client needs colliders); future work
  could replicate collider state instead.
- Consider renaming the `Ahoy*` types (`AhoyPlugins`, `AhoySystems`, `AhoyKccPlugin`,
  etc.) to something neutral now that the kcc is no longer a fork-with-the-same-name.
