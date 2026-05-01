//! Server boot: spawn the `NetcodeServer` entity and trigger `Start`.

use std::net::SocketAddr;

use bevy::prelude::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;

/// Spawn the `NetcodeServer` entity (called at app construction time, before
/// `Startup`). Returns the entity id so the caller can chain it.
pub fn spawn_server(world: &mut World, bind_addr: SocketAddr) -> Entity {
    world
        .spawn((
            Name::new("Server"),
            NetcodeServer::new(NetcodeConfig::default()),
            LocalAddr(bind_addr),
            ServerUdpIo::default(),
        ))
        .id()
}

/// Startup system: trigger `Start` on the unique server entity.
pub fn start_server(mut commands: Commands, server: Single<Entity, With<Server>>) {
    commands.trigger(Start {
        entity: server.into_inner(),
    });
}
