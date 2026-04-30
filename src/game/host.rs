//! Host-client wiring: spawn a `NetcodeServer` and an in-process `Client`
//! that shares the link via `LinkOf { server }`. Chains Startup so the
//! server `Start`s before the client `Connect`s.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use bevy::prelude::*;
use lightyear::prelude::*;

use crate::game::{client, server};

pub const DEFAULT_HOST_BIND: SocketAddr =
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5000);

pub struct HostPlugin;

impl Plugin for HostPlugin {
    fn build(&self, app: &mut App) {
        let server_entity = server::spawn_server(app.world_mut(), DEFAULT_HOST_BIND);

        app.world_mut().spawn((
            Name::new("HostClient"),
            Client::default(),
            LinkOf {
                server: server_entity,
            },
        ));

        // CRITICAL: server must reach `Started` before the host-client `Connect`s.
        app.add_systems(
            Startup,
            (server::start_server, client::start_client).chain(),
        );
    }
}
