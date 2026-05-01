//! Client boot: spawn the `Client` entity and trigger `Connect`.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use bevy::prelude::*;
use lightyear::prelude::client::*;
use lightyear::prelude::*;

/// Spawn the `Client` entity (called at app construction time, before `Startup`).
/// Returns the entity id so the caller can chain it.
pub fn spawn_client(world: &mut World, client_id: u64, server_addr: SocketAddr) -> Entity {
    let auth = Authentication::Manual {
        server_addr,
        client_id,
        private_key: lightyear::netcode::Key::default(),
        protocol_id: 0,
    };
    let netcode =
        NetcodeClient::new(auth, NetcodeConfig::default()).expect("failed to build NetcodeClient");
    world
        .spawn((
            Name::new("Client"),
            Client::default(),
            Link::new(None),
            LocalAddr(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)),
            PeerAddr(server_addr),
            ReplicationReceiver::default(),
            // Required for client-side prediction + input messages to flow back to the server.
            // Adds InputTimelineConfig (and thus InputTimeline), LastConfirmedInput, etc.
            PredictionManager::default(),
            netcode,
            UdpIo::default(),
        ))
        .id()
}

/// Startup system: trigger `Connect` on the unique client entity.
pub fn start_client(mut commands: Commands, client: Single<Entity, With<Client>>) {
    commands.trigger(Connect {
        entity: client.into_inner(),
    });
}
