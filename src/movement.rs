use super::components::*;
use super::util::acceleration;
use avian3d::{math::*, prelude::*};
use bevy::prelude::*;

/// Updates the [`Grounded`] status for character controllers.
pub fn fps_controller_grounded(
    mut commands: Commands,
    mut query: Query<(Entity, &ShapeHits, &Rotation), With<FpsController>>,
) {
    for (entity, hits, rotation) in &mut query {
        // The character is grounded if the shape caster has a hit with a normal
        // that isn't too steep.
        let is_grounded = hits
            .iter()
            .any(|hit| (rotation * -hit.normal2).angle_between(Vector::Y).abs() <= 0.3); // Reduced from 0.5

        if is_grounded {
            commands.entity(entity).insert(Grounded);
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}

pub fn fps_controller_look(mut query: Query<(&mut FpsController, &FpsControllerInput)>) {
    for (mut controller, input) in query.iter_mut() {
        controller.pitch = input.pitch;
        controller.yaw = input.yaw;
    }
}

pub fn fps_controller_move(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &FpsControllerInput,
        &mut FpsController,
        &mut Collider,
        &mut Transform,
        &mut LinearVelocity,
        &ShapeCaster,
        &ShapeHits,
        Has<Grounded>,
    )>,
) {
    let dt = time.delta_seconds();

    for (
        entity,
        input,
        mut controller,
        mut collider,
        mut transform,
        mut velocity,
        _shape_caster,
        shape_hits,
        grounded,
    ) in query.iter_mut()
    {
        if input.fly {
            controller.move_mode = match controller.move_mode {
                MoveMode::Noclip => MoveMode::Ground,
                MoveMode::Ground => MoveMode::Noclip,
            }
        }

        shape_hits.as_slice().iter().for_each(|hit| {
            if hit.normal1.y > controller.traction_normal_cutoff {
                controller.ground_tick = 1;
            }
        });

        match controller.move_mode {
            MoveMode::Noclip => handle_noclip_mode(&input, &mut controller, &mut velocity),
            MoveMode::Ground => handle_ground_mode(
                entity,
                dt,
                &input,
                &mut controller,
                &mut collider,
                &mut transform,
                &mut velocity,
                shape_hits,
                grounded,
                &mut commands,
            ),
        }
    }
}

fn handle_noclip_mode(
    input: &FpsControllerInput,
    controller: &mut FpsController,
    velocity: &mut LinearVelocity,
) {
    if input.movement == Vec3::ZERO {
        *velocity = LinearVelocity::ZERO;
    } else {
        let fly_speed = if input.sprint {
            controller.fast_fly_speed
        } else {
            controller.fly_speed
        };
        let mut move_to_world = Mat3::from_euler(EulerRot::YXZ, input.yaw, input.pitch, 0.0);
        move_to_world.z_axis *= -1.0; // Forward is -Z
        move_to_world.y_axis = Vec3::Y; // Vertical movement aligned with world up
        *velocity = LinearVelocity(move_to_world * input.movement * fly_speed);
    }
}

fn handle_ground_mode(
    entity: Entity,
    dt: f32,
    input: &FpsControllerInput,
    controller: &mut FpsController,
    collider: &mut Collider,
    transform: &mut Transform,
    velocity: &mut LinearVelocity,
    shape_hits: &ShapeHits,
    grounded: bool,
    commands: &mut Commands,
) {
    if let Some(capsule) = collider.shape_scaled().as_capsule() {
        let (wish_direction, wish_speed) = calculate_wish_movement(input, controller);
        
        if !grounded {
            handle_air_movement(dt, controller, velocity, wish_direction, wish_speed);
        }

        for shape_hit_data in shape_hits.as_slice().iter() {
            handle_shape_hit(dt, input, controller, velocity, shape_hit_data, transform, entity, commands);
        }

        // Step offset
        if controller.step_offset > f32::EPSILON && controller.ground_tick >= 1 {
            handle_step_offset(controller, transform, velocity, shape_hits);
        }
    }
}

fn calculate_wish_movement(input: &FpsControllerInput, controller: &FpsController) -> (Vec3, f32) {
    let speeds = Vec3::new(controller.side_speed, 0.0, controller.forward_speed);
    let mut move_to_world = Mat3::from_axis_angle(Vec3::Y, input.yaw);
    move_to_world.z_axis *= -1.0; // Forward is -Z
    let mut wish_direction = move_to_world * (input.movement * speeds);
    let mut wish_speed = wish_direction.length();
    if wish_speed > f32::EPSILON {
        wish_direction /= wish_speed;
    }
    let max_speed = if input.crouch {
        controller.crouched_speed
    } else if input.sprint {
        controller.run_speed
    } else {
        controller.walk_speed
    };
    wish_speed = f32::min(wish_speed, max_speed);
    (wish_direction, wish_speed)
}

fn handle_air_movement(dt: f32, controller: &mut FpsController, velocity: &mut LinearVelocity, wish_direction: Vec3, wish_speed: f32) {
    controller.ground_tick = 0;
    let air_speed = f32::min(wish_speed, controller.air_speed_cap);

    let mut add = acceleration(
        wish_direction,
        air_speed,
        controller.air_acceleration,
        velocity.0,
        dt,
    );
    add.y = -controller.gravity * dt;
    velocity.0 += add;

    let air_speed = velocity.0.xz().length();
    if air_speed > controller.max_air_speed {
        let ratio = controller.max_air_speed / air_speed;
        velocity.0.x *= ratio;
        velocity.0.z *= ratio;
    }
}

fn handle_shape_hit(
    dt: f32,
    input: &FpsControllerInput,
    controller: &mut FpsController,
    velocity: &mut LinearVelocity,
    shape_hit_data: &ShapeHitData,
    transform: &mut Transform,
    entity: Entity,
    commands: &mut Commands,
) {
    let has_traction = Vec3::dot(shape_hit_data.normal1, Vec3::Y) > controller.traction_normal_cutoff;

    if controller.ground_tick >= 1 && has_traction {
        handle_ground_friction(dt, controller, velocity);
    }

    let wish_direction = velocity.0.normalize_or_zero();
    let wish_speed = velocity.0.length();

    let mut add = acceleration(
        wish_direction,
        wish_speed,
        controller.acceleration,
        velocity.0,
        dt,
    );
    if !has_traction {
        add.y -= controller.gravity * dt;
    }
    velocity.0 += add;

    if has_traction {
        let linvel = velocity.0;
        *velocity = LinearVelocity(
            linvel - Vec3::dot(linvel, shape_hit_data.normal1) * shape_hit_data.normal1,
        );

        if input.jump && (controller.ground_tick > 0) {
            velocity.0.y = controller.jump_speed;
            controller.ground_tick = 0;
            commands.entity(entity).remove::<Grounded>();
        }

        // Small offset to keep above ground
        transform.translation.y += 0.01;
    }

    controller.ground_tick = controller.ground_tick.saturating_add(1);
}

fn handle_ground_friction(dt: f32, controller: &FpsController, velocity: &mut LinearVelocity) {
    let lateral_speed = velocity.0.xz().length();
    if lateral_speed > controller.friction_speed_cutoff {
        let control = f32::max(lateral_speed, controller.stop_speed);
        let drop = control * controller.friction * dt;
        let new_speed = f32::max((lateral_speed - drop) / lateral_speed, 0.0);
        velocity.x *= new_speed;
        velocity.z *= new_speed;
    } else {
        velocity.x = 0.0;
        velocity.z = 0.0;
    }
}

fn handle_step_offset(
    controller: &FpsController,
    transform: &mut Transform,
    velocity: &mut LinearVelocity,
    shape_hits: &ShapeHits,
) {
    let forward_direction = (Quat::from_rotation_y(controller.yaw) * Vec3::NEG_Z).normalize();
    let cast_offset = forward_direction * controller.radius * 1.1;
    let cast_start = transform.translation + cast_offset + Vec3::Y * controller.step_offset * 1.1;
    
    for hit in shape_hits.iter() {
        let hit_point = hit.point1;
        let distance = (hit_point - cast_start).dot(Vec3::NEG_Y);
        
        if distance <= controller.step_offset {
            let step_height = controller.step_offset - distance;
            if step_height > 0.0 {
                transform.translation.y += step_height;
                velocity.0.y = velocity.0.y.max(0.0); // Prevent downward velocity when stepping up
            }
            break; // We only need to handle the first valid hit
        }
    }
}
