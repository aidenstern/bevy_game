use crate::components::FpsActions;

use super::input::*;
use super::movement::*;
use super::render::*;

use bevy::{input::InputSystem, prelude::*};
use leafwing_input_manager::{plugin::InputManagerSystem, systems::run_if_enabled};

pub struct FpsControllerPlugin;

impl Plugin for FpsControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                fps_controller_input,
                fps_controller_look,
                fps_controller_move,
            )
                .chain()
                .run_if(run_if_enabled::<FpsActions>)
                .in_set(InputManagerSystem::ManualControl)
                .before(InputManagerSystem::ReleaseOnDisable)
                .after(InputManagerSystem::Tick)
                .after(InputManagerSystem::Update)
                .after(InputSystem),
        )
        .add_systems(Update, fps_controller_render);
    }
}
