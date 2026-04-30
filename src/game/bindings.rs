//! `PlayerInput` `bevy_enhanced_input` context: marker Component that owns the
//! local player's keyboard/gamepad action bindings.
//!
//! In networked play, only the client attaches `PlayerInput` (and its
//! bindings) — see `client::setup_local_player`. The server never has
//! `EnhancedInputPlugin` and never spawns `PlayerInput`, since input arrives
//! via `lightyear`'s replicated `ActionState<AccumulatedInput>` /
//! `ActionState<CharacterLook>`.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

pub struct BindingsPlugin;

impl Plugin for BindingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_input_context::<PlayerInput>();
    }
}

#[derive(Component, Default)]
pub struct PlayerInput;
