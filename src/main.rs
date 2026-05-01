//! `bevy_game`: server-authoritative multiplayer game built on a forked
//! first-person KCC. Public entry is [`run`]; the binary `src/main.rs` just
//! calls it.
//!
//! Internal layout follows a foxtrot-style domain split:
//! - [`kcc`] — kinematic character controller (forked from `janhohenheim/bevy_ahoy`)
//! - [`shared`] — markers, scene, CLI, replication protocol; always compiles
//! - [`client`] — gated `feature = "client"`: render, input, debug, prediction glue
//! - [`server`] — gated `feature = "server"`: spawn, respawn, input plumbing
//! - [`host`] — gated when both features are on: in-process server + client wiring

#[cfg(not(any(feature = "client", feature = "server")))]
compile_error!(
    "bevy_game needs at least one of the `client` or `server` features enabled \
     (the default `[\"client\", \"server\"]` is host-client mode)."
);

use core::time::Duration;

use avian3d::prelude::{
    PhysicsInterpolationPlugin, PhysicsPlugins, PhysicsTransformPlugin,
};
use bevy::{
    image::{ImageAddressMode, ImageSamplerDescriptor},
    light::DirectionalLightShadowMap,
    prelude::*,
};
use clap::Parser;

pub mod kcc;
pub mod shared;

#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "server")]
pub mod server;
#[cfg(all(feature = "client", feature = "server"))]
pub mod host;

use crate::shared::{cli::Cli, player::PlayerPlugin, scene::ScenePlugin};

#[cfg(any(feature = "client", feature = "server"))]
use crate::shared::cli::Mode;

/// Tick rate shared between client and server.
const TICK_DURATION: Duration = Duration::from_millis(1000 / 60);

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Loading,
    InGame,
}

/// If set, the game will automatically exit after this many frames.
#[derive(Resource)]
pub struct ExitAfterFrames(pub u32);

pub fn main() -> AppExit {
    let cli = Cli::parse();

    let mut app = App::new();

    match &cli.mode {
        #[cfg(feature = "server")]
        Some(Mode::Server { bind_addr }) => {
            info!("Starting in server mode (bind_addr={bind_addr})");
        }
        #[cfg(feature = "client")]
        Some(Mode::Client {
            client_id,
            server_addr,
        }) => {
            info!("Starting in client mode (client_id={client_id}, server_addr={server_addr})");
        }
        None => {
            info!("Starting in host-client mode");
        }
    }

    // Bevy + window plugins. Dedicated-server mode skips the primary window so
    // it runs effectively headless (winit still initialises but never opens a
    // window). Client and host-client modes get the full window/render stack.
    let window_plugin = match &cli.mode {
        #[cfg(feature = "server")]
        Some(Mode::Server { .. }) => WindowPlugin {
            primary_window: None,
            exit_condition: bevy::window::ExitCondition::DontExit,
            ..default()
        },
        _ => WindowPlugin {
            primary_window: Window {
                #[cfg(all(not(target_arch = "wasm32"), not(target_os = "macos")))]
                present_mode: bevy::window::PresentMode::Mailbox,
                ..default()
            }
            .into(),
            ..default()
        },
    };
    app.add_plugins(
        DefaultPlugins
            .set(window_plugin)
            .set(ImagePlugin {
                default_sampler: ImageSamplerDescriptor {
                    address_mode_u: ImageAddressMode::Repeat,
                    address_mode_v: ImageAddressMode::Repeat,
                    address_mode_w: ImageAddressMode::Repeat,
                    anisotropy_clamp: 16,
                    ..ImageSamplerDescriptor::linear()
                },
            }),
    );

    // Avian: lightyear's avian glue replaces the transform/interpolation
    // subplugins, so we disable them here.
    app.add_plugins(
        PhysicsPlugins::default()
            .build()
            .disable::<PhysicsTransformPlugin>()
            .disable::<PhysicsInterpolationPlugin>(),
    );

    // Lightyear core plugin groups (per side).
    #[cfg(feature = "client")]
    app.add_plugins(lightyear::prelude::client::ClientPlugins {
        tick_duration: TICK_DURATION,
    });
    #[cfg(feature = "server")]
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: TICK_DURATION,
    });

    // KCC core.
    app.add_plugins(kcc::AhoyPlugins::default());

    // Shared scene + replication setup. Avian + protocol registration must run
    // on both sides; component types they register are referenced by both
    // client-side prediction and server-side replication.
    app.add_plugins((
        shared::networking::avian::SimpleAvianSetupPlugin,
        shared::networking::protocol::ProtocolPlugin,
        ScenePlugin,
        PlayerPlugin,
    ))
    .init_state::<GameState>()
    .insert_resource(DirectionalLightShadowMap { size: 4096 })
    .insert_resource(GlobalAmbientLight::NONE);

    // Per-side plugin aggregators.
    #[cfg(feature = "client")]
    app.add_plugins(client::ClientPlugin);
    #[cfg(feature = "server")]
    app.add_plugins(server::ServerPlugin);

    // Per-mode connection bootstrapping.
    match cli.mode {
        #[cfg(feature = "server")]
        Some(Mode::Server { bind_addr }) => {
            server::spawn_server(app.world_mut(), bind_addr);
            app.add_systems(Startup, server::start_server);
        }
        #[cfg(feature = "client")]
        Some(Mode::Client {
            client_id,
            server_addr,
        }) => {
            client::spawn_client(app.world_mut(), client_id, server_addr);
            app.add_systems(Startup, client::start_client);
        }
        None => {
            #[cfg(all(feature = "client", feature = "server"))]
            app.add_plugins(host::HostPlugin);
        }
    }

    if let Some(frames) = cli.frames {
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
