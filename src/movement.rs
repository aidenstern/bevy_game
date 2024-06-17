use super::components::*;
use super::util::acceleration;
use bevy::prelude::*;
use bevy_xpbd_3d::prelude::*;

pub fn fps_controller_look(mut query: Query<(&mut FpsController, &FpsControllerInput)>) {
    for (mut controller, input) in query.iter_mut() {
        controller.pitch = input.pitch;
        controller.yaw = input.yaw;
    }
}

pub fn fps_controller_move(
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &FpsControllerInput,
        &mut FpsController,
        &mut Collider,
        &mut Transform,
        &mut LinearVelocity,
        &ShapeCaster,
        &ShapeHits,
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
) {
    if let Some(capsule) = collider.shape_scaled().as_capsule() {
        let speeds = Vec3::new(controller.side_speed, 0.0, controller.forward_speed);
        let mut move_to_world = Mat3::from_axis_angle(Vec3::Y, input.yaw);
        move_to_world.z_axis *= -1.0; // Forward is -Z
        let mut wish_direction = move_to_world * (input.movement * speeds);
        let mut wish_speed = wish_direction.length();
        if wish_speed > f32::EPSILON {
            // Avoid division by zero
            wish_direction /= wish_speed; // Effectively normalize, avoid length computation twice
        }
        let max_speed = if input.crouch {
            controller.crouched_speed
        } else if input.sprint {
            controller.run_speed
        } else {
            controller.walk_speed
        };
        wish_speed = f32::min(wish_speed, max_speed);

        println!("Wish speed: {}", wish_speed);

        // if there was no hits
        if shape_hits.as_slice().is_empty() {
            println!("No hits");
            controller.ground_tick = 0;
            wish_speed = f32::min(wish_speed, controller.air_speed_cap);

            let mut add = acceleration(
                wish_direction,
                wish_speed,
                controller.air_acceleration,
                velocity.xyz(),
                dt,
            );
            add.y = -controller.gravity * dt;
            *velocity = LinearVelocity(velocity.xyz() + add);

            let air_speed = velocity.xz().length();
            if air_speed > controller.max_air_speed {
                let ratio = controller.max_air_speed / air_speed;
                velocity.x *= ratio;
                velocity.z *= ratio;
            }
        }

        // process shape hits
        for shape_hit_data in shape_hits.as_slice().iter() {
            let has_traction =
                Vec3::dot(shape_hit_data.normal1, Vec3::Y) > controller.traction_normal_cutoff;

            // Only apply friction after at least one tick, allows b-hopping without losing speed
            if controller.ground_tick >= 1 && has_traction {
                let lateral_speed = velocity.xz().length();
                if lateral_speed > controller.friction_speed_cutoff {
                    let control = f32::max(lateral_speed, controller.stop_speed);
                    let drop = control * controller.friction * dt;
                    let new_speed = f32::max((lateral_speed - drop) / lateral_speed, 0.0);
                    velocity.x *= new_speed;
                    velocity.z *= new_speed;
                } else {
                    velocity.x = 0.0;
                    velocity.y = 0.0;
                    velocity.z = 0.0;
                }
                if controller.ground_tick == 1 {
                    velocity.y = -shape_hit_data.time_of_impact;
                }
            }

            let mut add = acceleration(
                wish_direction,
                wish_speed,
                controller.acceleration,
                velocity.xyz(),
                dt,
            );
            if !has_traction {
                add.y -= controller.gravity * dt;
            }
            *velocity = LinearVelocity(velocity.xyz() + add);

            if has_traction {
                *velocity = LinearVelocity(
                    Vec3::dot(**velocity, shape_hit_data.normal1) * shape_hit_data.normal1,
                );

                if input.jump {
                    velocity.y = controller.jump_speed;
                }
            }

            // Increment ground tick but cap at max value
            controller.ground_tick = controller.ground_tick.saturating_add(1);
        }

        /* Crouching */

        let crouch_height = controller.crouch_height;
        let upright_height = controller.upright_height;

        let crouch_speed = if input.crouch {
            -controller.crouch_speed
        } else {
            controller.uncrouch_speed
        };
        controller.height += dt * crouch_speed;
        controller.height = controller.height.clamp(crouch_height, upright_height);

        // if let Some(mut capsule) = collider.into() {
        //     capsule.set_segment(Vec3::Y * 0.5, Vec3::Y * controller.height);
        // }

        // Step offset
        // if controller.step_offset > f32::EPSILON && controller.ground_tick >= 1 {
        //     let cast_offset = velocity.normalize_or_zero() * controller.radius * 1.0625;
        //     let cast = spatial_query.cast_ray(
        //         transform.translation + cast_offset + Vec3::Y * controller.step_offset * 1.0625,
        //         Direction3d::NEG_Y,
        //         controller.step_offset * 0.9375,
        //         false,
        //         filter.clone(),
        //     );
        //     if let Some(ray_hit_data) = cast {
        //         transform.translation.y +=
        //             controller.step_offset * 1.0625 - ray_hit_data.time_of_impact;
        //         transform.translation += cast_offset;
        //     }
        // }

        println!("Linear velocity: {:?}", velocity.xyz());

        // Prevent falling off ledges
        // if controller.ground_tick >= 1 && input.crouch {
        // for _ in 0..2 {
        //     // Find the component of our velocity that is overhanging and subtract it off
        //     let overhang = overhang_component(
        //         entity,
        //         transform.as_ref(),
        //         physics_context.as_ref(),
        //         velocity.linvel,
        //         dt,
        //     );
        //     if let Some(overhang) = overhang {
        //         velocity.linvel -= overhang;
        //     }
        // }
        // // If we are still overhanging consider unsolvable and freeze
        // if overhang_component(
        //     entity,
        //     transform.as_ref(),
        //     physics_context.as_ref(),
        //     velocity.xyz(),
        //     dt,
        // )
        //     .is_some()
        // {
        //     *velocity = LinearVelocity(Vec3::ZERO);
        // }
        // }
    }
}
