use crate::components::FpsActions;

use super::input::*;
use super::movement::*;
use super::render::*;

use bevy::prelude::*;
use leafwing_input_manager::systems::run_if_enabled;

pub struct FpsControllerPlugin;

impl Plugin for FpsControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                fps_controller_grounded,
                fps_controller_input,
                fps_controller_move,
                fps_controller_look,
                fps_controller_render,
            )
                .chain()
                .run_if(run_if_enabled::<FpsActions>),
        );
    }
}
