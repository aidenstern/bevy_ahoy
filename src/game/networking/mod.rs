//! Lightyear glue for `bevy_ahoy`. Vendored from `andriyDev/lightyear_ahoy`
//! so this game can be self-contained.
//!
//! - [`avian::SimpleAvianSetupPlugin`] — registers Position/Rotation/LinearVelocity/AngularVelocity
//!   for prediction with rollback thresholds + correction + visual interpolation.
//! - [`protocol::ProtocolPlugin`] — registers `AccumulatedInput` and `CharacterLook` as
//!   replicated input components and `CharacterControllerState` as a predicted component.
//! - [`client::ClientPlugin`] — copies local `AccumulatedInput`/`CharacterLook` into
//!   `ActionState` for replication, and reverses the copy during rollback.
//! - [`server::ServerPlugin`] — copies received `ActionState` into `AccumulatedInput`
//!   for the kcc to consume.

pub mod avian;
pub mod client;
pub mod protocol;
pub mod server;
