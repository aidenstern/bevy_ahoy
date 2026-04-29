//! Water gameplay was removed during the server-authoritative migration prep.
//!
//! These types are kept as inert markers because [`crate::kcc`] still references
//! [`WaterState`] and [`Water`] in its queries; nothing populates [`WaterState`]
//! anymore, so all water-aware code paths in the kcc are dead at runtime.

use crate::prelude::*;

#[derive(Component, Default, Copy, Reflect, PartialEq, Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", reflect(Serialize, Deserialize))]
#[reflect(Component)]
pub struct WaterState {
    pub level: WaterLevel,
    pub speed: f32,
}

#[derive(Default, Copy, Reflect, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", reflect(Serialize, Deserialize))]
pub enum WaterLevel {
    #[default]
    None,
    Feet,
    Waist,
    Head,
}

#[derive(Reflect, Component, Default)]
#[require(Sensor, Transform, GlobalTransform)]
#[reflect(Component)]
pub struct Water {
    pub speed: f32,
}
