//! `PlayerInput` bevy_enhanced_input context: keyboard/gamepad bindings for
//! the player's movement/jump/look actions.
//!
//! Phase 5 will move the `actions!` macro into a client-only setup observer
//! once networking lands (server spawns the context Component, client attaches
//! bindings only on the controlled-predicted entity). For now the bindings
//! attach via the `on_add` hook so single-player works out of the box.

use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};
use bevy_ahoy::prelude::*;
use bevy_enhanced_input::prelude::{Press, *};

pub struct BindingsPlugin;

impl Plugin for BindingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_input_context::<PlayerInput>();
    }
}

#[derive(Component, Default)]
#[component(on_add = PlayerInput::on_add)]
pub struct PlayerInput;

impl PlayerInput {
    fn on_add(mut world: DeferredWorld, ctx: HookContext) {
        world
            .commands()
            .entity(ctx.entity)
            .insert(actions!(PlayerInput[
                (
                    Action::<Movement>::new(),
                    DeadZone::default(),
                    Bindings::spawn((Cardinal::wasd_keys(), Axial::left_stick()))
                ),
                (
                    Action::<Jump>::new(),
                    Press::default(),
                    bindings![
                        KeyCode::Space,
                        GamepadButton::South,
                        Binding::mouse_wheel(),
                    ],
                ),
                (
                    Action::<Tac>::new(),
                    Press::default(),
                    bindings![
                        KeyCode::Space,
                        GamepadButton::South,
                        Binding::mouse_wheel(),
                    ],
                ),
                (
                    Action::<Crane>::new(),
                    Press::default(),
                    bindings![
                        KeyCode::Space,
                        GamepadButton::South,
                        Binding::mouse_wheel(),
                    ],
                ),
                (
                    Action::<Mantle>::new(),
                    Hold::new(0.2),
                    bindings![KeyCode::Space, GamepadButton::South],
                ),
                (
                    Action::<Climbdown>::new(),
                    bindings![KeyCode::ControlLeft, GamepadButton::LeftTrigger2],
                ),
                (
                    Action::<Crouch>::new(),
                    bindings![KeyCode::ControlLeft, GamepadButton::LeftTrigger2],
                ),
                (
                    Action::<RotateCamera>::new(),
                    Bindings::spawn((
                        Spawn((Binding::mouse_motion(), Scale::splat(0.07))),
                        Axial::right_stick().with((Scale::splat(4.0), DeadZone::default())),
                    ))
                ),
            ]));
    }
}
