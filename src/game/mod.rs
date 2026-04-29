//! Top-level game module: wires the App, owns [`GameState`], dispatches per-mode
//! setup. The actual gameplay/input/render logic lives in the submodules.
//!
//! Single binary entry point lives at `src/main.rs`; it just calls [`run`].

#[cfg(not(any(feature = "client", feature = "server")))]
compile_error!(
    "bevy_ahoy needs at least one of the `client` or `server` features enabled \
     (the default `[\"client\", \"server\"]` is host-client mode)"
);

use avian3d::prelude::PhysicsPlugins;
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
#[cfg(all(feature = "client", feature = "server"))]
pub mod host;
pub mod player;
pub mod scene;
pub mod setup;
#[cfg(feature = "server")]
pub mod server;
pub mod visuals;

use crate::game::{
    bindings::BindingsPlugin, cli::Cli, cursor::CursorPlugin, debug::DebugPlugin,
    player::PlayerPlugin, scene::ScenePlugin, setup::SetupPlugin, visuals::VisualsPlugin,
};

#[cfg(feature = "client")]
use crate::game::client::ClientPlugin;
#[cfg(all(feature = "client", feature = "server"))]
use crate::game::host::HostPlugin;
#[cfg(feature = "server")]
use crate::game::server::ServerPlugin;

#[cfg(any(feature = "client", feature = "server"))]
use crate::game::cli::Mode;

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

    // Per-mode bevy plugin set.
    //
    // For the structural-prep round all three branches reuse the same gameplay
    // wiring (full single-player setup) so the binary keeps working under any
    // feature combination. The networking phase will swap server-mode to
    // `MinimalPlugins`, drop the local player spawn from client mode, etc.
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
    .add_plugins((
        ScenePlugin,
        SetupPlugin,
        PlayerPlugin,
        BindingsPlugin,
        DebugPlugin,
        CursorPlugin,
        VisualsPlugin,
    ))
    .init_state::<GameState>()
    .insert_resource(DirectionalLightShadowMap { size: 4096 })
    .insert_resource(GlobalAmbientLight::NONE);

    // Per-mode networking plugins (stubs in this phase).
    match &cli.mode {
        #[cfg(feature = "server")]
        Some(Mode::Server { .. }) => {
            app.add_plugins(ServerPlugin);
        }
        #[cfg(feature = "client")]
        Some(Mode::Client { .. }) => {
            app.add_plugins(ClientPlugin);
        }
        None => {
            #[cfg(feature = "server")]
            app.add_plugins(ServerPlugin);
            #[cfg(feature = "client")]
            app.add_plugins(ClientPlugin);
            #[cfg(all(feature = "client", feature = "server"))]
            app.add_plugins(HostPlugin);
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
