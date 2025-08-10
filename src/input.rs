use super::components::*;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use std::f32::consts::*;

const ANGLE_EPSILON: f32 = 0.001953125;

pub fn fps_controller_input(
    action_state_query: Query<&ActionState<FpsActions>>,
    mut query: Query<(&FpsController, &mut FpsControllerInput)>,
) {
    let Ok((controller, mut input)) = query.get_single_mut() else { return; };

    if !controller.enable_input {
        return;
    }

    for action_state in action_state_query.iter() {
        let mouse_movement = action_state.axis_pair(&FpsActions::MousePosition);
        let mouse_delta = mouse_movement.xy() * controller.sensitivity;

        input.pitch = (input.pitch - mouse_delta.y)
            .clamp(-FRAC_PI_2 + ANGLE_EPSILON, FRAC_PI_2 - ANGLE_EPSILON);
        input.yaw -= mouse_delta.x;
        if input.yaw.abs() > PI {
            input.yaw = input.yaw.rem_euclid(TAU);
        }

        input.movement = Vec3::new(
            get_axis(&action_state, FpsActions::Right, FpsActions::Left),
            get_axis(&action_state, FpsActions::Jump, FpsActions::Sprint),
            get_axis(&action_state, FpsActions::Forward, FpsActions::Backward),
        );

        input.fly = action_state.just_pressed(&FpsActions::Fly);
        input.sprint = action_state.pressed(&FpsActions::Sprint);
        input.jump = action_state.pressed(&FpsActions::Jump);
        input.crouch = action_state.pressed(&FpsActions::Crouch);
    }
}

fn get_pressed(key_input: &ActionState<FpsActions>, key: FpsActions) -> f32 {
    if key_input.pressed(&key) {
        1.0
    } else {
        0.0
    }
}

fn get_axis(key_input: &ActionState<FpsActions>, key_pos: FpsActions, key_neg: FpsActions) -> f32 {
    get_pressed(key_input, key_pos) - get_pressed(key_input, key_neg)
}
