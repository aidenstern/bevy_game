use super::components::*;
use bevy::prelude::*;
use leafwing_input_manager::{prelude::*};
use std::f32::consts::*;

const ANGLE_EPSILON: f32 = 0.001953125;

pub fn fps_controller_input(
    action_state_query: Query<&ActionState<FpsActions>>,
    mut query: Query<(&FpsController, &mut FpsControllerInput)>,
) {
    let action_state = action_state_query.single();
    for (controller, mut input) in query.iter_mut() {
        if !controller.enable_input {
            continue;
        }

        if let Some(mouse_movement) = action_state.axis_pair(&FpsActions::MousePosition) {
            let mouse_delta = mouse_movement.xy() * controller.sensitivity;

            input.pitch = (input.pitch - mouse_delta.y)
                .clamp(-FRAC_PI_2 + ANGLE_EPSILON, FRAC_PI_2 - ANGLE_EPSILON);
            input.yaw -= mouse_delta.x;
            if input.yaw.abs() > PI {
                input.yaw = input.yaw.rem_euclid(TAU);
            }
        }

        // Other input handling can go here, for movement, etc.
    }
}
