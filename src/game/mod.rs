//! Top-level game module: wires the App, owns [`GameState`], dispatches per-mode
//! setup. Networking glue lives in [`networking`]; per-mode boot lives in
//! [`server`] / [`client`] / [`host`].
//!
//! Single binary entry point lives at `src/main.rs`; it just calls [`run`].

#[cfg(not(any(feature = "client", feature = "server")))]
compile_error!(
    "bevy_ahoy needs at least one of the `client` or `server` features enabled \
     (the default `[\"client\", \"server\"]` is host-client mode)"
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
use bevy_ahoy::prelude::*;
use bevy_enhanced_input::EnhancedInputPlugin;
use bevy_framepace::FramepacePlugin;
use bevy_mod_mipmap_generator::MipmapGeneratorPlugin;
use clap::Parser;

pub mod bindings;
pub mod cli;
#[cfg(feature = "client")]
pub mod client;
pub mod cursor;
pub mod debug;
pub mod debug_net;
#[cfg(all(feature = "client", feature = "server"))]
pub mod host;
pub mod networking;
pub mod player;
pub mod scene;
#[cfg(feature = "server")]
pub mod server;
pub mod visuals;

use crate::game::{
    bindings::BindingsPlugin, cli::Cli, cursor::CursorPlugin, debug::DebugPlugin,
    player::PlayerPlugin, scene::ScenePlugin, visuals::VisualsPlugin,
};

#[cfg(any(feature = "client", feature = "server"))]
use crate::game::cli::Mode;

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

pub fn run() -> AppExit {
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

    // bevy_ahoy core + enhanced input.
    app.add_plugins((EnhancedInputPlugin, AhoyPlugins::default()));

    // Networking glue (vendored from lightyear_ahoy).
    app.add_plugins((
        networking::avian::SimpleAvianSetupPlugin,
        networking::protocol::ProtocolPlugin,
        networking::client::ClientPlugin,
        networking::server::ServerPlugin,
        debug_net::NetworkDebugPlugin,
    ));

    // Visual / input / scene plugins.
    app.add_plugins((
        MipmapGeneratorPlugin,
        FramepacePlugin,
        ScenePlugin,
        PlayerPlugin,
        BindingsPlugin,
        DebugPlugin,
        CursorPlugin,
        VisualsPlugin,
    ))
    .init_state::<GameState>()
    .insert_resource(DirectionalLightShadowMap { size: 4096 })
    .insert_resource(GlobalAmbientLight::NONE);

    // Per-mode game plugins + connection bootstrapping.
    match cli.mode {
        #[cfg(feature = "server")]
        Some(Mode::Server { bind_addr }) => {
            app.add_plugins(server::ServerPlugin);
            server::spawn_server(app.world_mut(), bind_addr);
            app.add_systems(Startup, server::start_server);
        }
        #[cfg(feature = "client")]
        Some(Mode::Client {
            client_id,
            server_addr,
        }) => {
            app.add_plugins(client::ClientPlugin);
            client::spawn_client(app.world_mut(), client_id, server_addr);
            app.add_systems(Startup, client::start_client);
        }
        None => {
            #[cfg(feature = "server")]
            app.add_plugins(server::ServerPlugin);
            #[cfg(feature = "client")]
            app.add_plugins(client::ClientPlugin);
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
