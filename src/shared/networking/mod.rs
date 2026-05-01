//! Lightyear glue shared by both sides. Vendored from `andriyDev/lightyear_ahoy`.
//!
//! - [`avian::SimpleAvianSetupPlugin`] — registers Position/Rotation/LinearVelocity/AngularVelocity
//!   for prediction with rollback thresholds + correction + visual interpolation.
//! - [`protocol::ProtocolPlugin`] — registers `AccumulatedInput` and `CharacterLook` as
//!   replicated input components and `CharacterControllerState` as a predicted component.
//!
//! The per-side input plumbing (client copying its inputs into `ActionState`;
//! server copying received `ActionState` into `AccumulatedInput`) lives at
//! `crate::client::networking` and `crate::server::networking`.

pub mod avian;
pub mod protocol;
