//! Replicated player markers and shared registration.
//!
//! - [`LogicalPlayer`] — marker on the authoritative player entity (owns physics,
//!   character controller state). Spawned by the server in networked play.
//! - [`PlayerId`] — replicated stable id (mirrors lightyear's client id).
//! - [`CollisionLayer`] — physics layers used by the player and the world.
//!
//! `PlayerPlugin` registers these for replication. Server-authoritative respawn
//! lives in `crate::server::respawn`; the client-side render marker and setup
//! observers live in `crate::client::player`.

use avian3d::prelude::*;
use bevy::prelude::*;
use lightyear::prelude::AppComponentExt;

#[derive(Component, Reflect, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", reflect(Serialize, Deserialize))]
#[reflect(Component)]
pub struct LogicalPlayer;

#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", reflect(Serialize, Deserialize))]
#[reflect(Component)]
pub struct PlayerId(pub u64);

#[derive(Debug, PhysicsLayer, Default)]
pub enum CollisionLayer {
    #[default]
    Default,
    Player,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<LogicalPlayer>()
            .register_type::<PlayerId>()
            // Both LogicalPlayer and PlayerId need to be registered with lightyear
            // so they ride the wire when the server spawns a player. Without this,
            // the client only sees Position/Rotation/etc and the marker filters
            // (`With<LogicalPlayer>`) in `setup_local_player` never match.
            .register_component::<LogicalPlayer>();
        app.register_component::<PlayerId>();
    }
}
