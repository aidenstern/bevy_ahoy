# bevy_game

A server-authoritative multiplayer Bevy game with a Source-style first-person
kinematic character controller. Built on
[Bevy](https://github.com/bevyengine/bevy) +
[Avian](https://github.com/avianphysics/avian) +
[lightyear](https://github.com/cBournhonesque/lightyear) +
[BEI](https://github.com/simgine/bevy_enhanced_input).

## Run

```sh
just run                # host-client (server + client in one process). Default.
just run-server         # cargo run --no-default-features --features server -- server
just run-client 1       # cargo run --no-default-features --features client -- client --client-id 1

just smoke-test         # 300-frame host-client run, auto-exits 0
just check-all          # cargo check across all feature combinations
just lint               # clippy with -D warnings
```

## Layout

- `src/kcc/` — kinematic character controller. Forked from
  [`janhohenheim/bevy_ahoy`](https://github.com/janhohenheim/bevy_ahoy) and
  evolved for server-authoritative play. Module-internal; not republished.
- `src/shared/` — shared code between client/server.
- `src/client/` (`feature = "client"`) — local-player setup, render visuals,
  debug HUD, cursor capture, client-side input replication.
- `src/server/` (`feature = "server"`) — authoritative spawn / respawn, input
  plumbing, server diagnostics.
- `src/host/` (both features) — in-process server + client wiring for the
  default host-client mode.
- `src/lib.rs` exposes `pub fn run() -> AppExit`. `src/main.rs` just calls it.

## License

Dual licensed under MIT or Apache-2.0 (see `license-mit.txt` and
`license-apache-2.0.txt`). The character controller code under `src/kcc/`
retains the upstream `bevy_ahoy` licensing.
