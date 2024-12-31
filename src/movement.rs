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
            .any(|hit| (rotation * -hit.normal2).angle_between(Vector::Y).abs() <= 0.5);

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

        if !grounded {
            controller.ground_tick = 0;
            wish_speed = f32::min(wish_speed, controller.air_speed_cap);

            let mut add = acceleration(
                wish_direction,
                wish_speed,
                controller.air_acceleration,
                velocity.0,
                dt,
            );
            // println!("Air acceleration: {:?}", add);
            add.y = -controller.gravity * dt;
            velocity.0 += add;

            let air_speed = velocity.0.xz().length();
            if air_speed > controller.max_air_speed {
                let ratio = controller.max_air_speed / air_speed;
                velocity.0.x *= ratio;
                velocity.0.z *= ratio;
            }
            // println!("Air velocity: {:?}", velocity.0);
        }

        for shape_hit_data in shape_hits.as_slice().iter() {
            println!("Hit: {:?}", shape_hit_data);
            let has_traction =
                Vec3::dot(shape_hit_data.normal1, Vec3::Y) > controller.traction_normal_cutoff;

            if controller.ground_tick >= 1 && has_traction {
                let lateral_speed = velocity.0.xz().length();
                if lateral_speed > controller.friction_speed_cutoff {
                    let control = f32::max(lateral_speed, controller.stop_speed);
                    let drop = control * controller.friction * dt;
                    let new_speed = f32::max((lateral_speed - drop) / lateral_speed, 0.0);
                    velocity.x *= new_speed;
                    velocity.z *= new_speed;
                } else {
                    *velocity = LinearVelocity::ZERO;
                }
                if controller.ground_tick == 1 {
                    velocity.y = -shape_hit_data.time_of_impact;
                }
                // println!("Ground velocity: {:?}", velocity.0);
            }

            let mut add = acceleration(
                wish_direction,
                wish_speed,
                controller.acceleration,
                velocity.0,
                dt,
            );
            // println!("Acceleration: {:?}", add);
            if !has_traction {
                add.y -= controller.gravity * dt;
            }
            velocity.0 += add;

            if has_traction {
                let linvel = velocity.0;
                *velocity = LinearVelocity(
                    linvel - Vec3::dot(linvel, shape_hit_data.normal1) * shape_hit_data.normal1,
                );

                if input.jump {
                    velocity.0.y = controller.jump_speed;
                }
            }

            controller.ground_tick = controller.ground_tick.saturating_add(1);
        }

        let crouch_height = controller.crouch_height;
        let upright_height = controller.upright_height;

        let crouch_speed = if input.crouch {
            -controller.crouch_speed
        } else {
            controller.uncrouch_speed
        };
        controller.height += dt * crouch_speed;
        controller.height = controller.height.clamp(crouch_height, upright_height);

        if let Some(capsule) = collider.into() {
            capsule.set_shape(Collider::capsule(0.5, controller.height).shape().clone());
        }

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

        // println!("Linear velocity: {:?}", velocity.xyz());
        // println!("Ground tick: {}", controller.ground_tick);

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
