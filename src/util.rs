use crate::{FpsController, LogicalPlayer};
use avian3d::prelude::*;
use bevy::{
    gltf::{Gltf, GltfMesh, GltfNode},
    prelude::*,
    window::CursorGrabMode,
};

use super::components::MainScene;

// pub fn overhang_component(
//     entity: Entity,
//     transform: &Transform,
//     physics_context: &RapierContext,
//     velocity: Vec3,
//     dt: f32,
// ) -> Option<Vec3> {
//   // Cast a segment (zero radius on capsule) from our next position back towards us
//     // If there is a ledge in front of us we will hit the edge of it
//     // We can use the normal of the hit to subtract off the component that is overhanging
//     let cast_capsule = Collider::capsule(Vec3::Y * 0.125, -Vec3::Y * 0.125, 0.0);
//     let filter = QueryFilter::default().exclude_rigid_body(entity);
//     let future_position = transform.translation + velocity * dt;
//     let cast = physics_context.cast_shape(
//         future_position,
//         transform.rotation,
//         -velocity,
//         &cast_capsule,
//         0.5,
//         true,
//         filter,
//     );
//     if let Some((_, toi_details)) = toi_details_unwrap(cast) {
//         let cast = physics_context.cast_ray(
//             future_position + Vec3::Y * 0.125,
//             -Vec3::Y,
//             0.375,
//             false,
//             filter,
//         );
//         // Make sure that this is actually a ledge, e.g. there is no ground in front of us
//         if cast.is_none() {
//             let normal = -toi_details.normal1;
//             let alignment = Vec3::dot(velocity, normal);
//             return Some(alignment * normal);
//         }
//     }
//     None
// }

pub fn acceleration(
    wish_direction: Vec3,
    wish_speed: f32,
    acceleration: f32,
    velocity: Vec3,
    dt: f32,
) -> Vec3 {
    let velocity_projection = Vec3::dot(velocity, wish_direction);
    let add_speed = wish_speed - velocity_projection;
    if add_speed <= 0.0 {
        return Vec3::ZERO;
    }

    let acceleration_speed = f32::min(acceleration * wish_speed * dt, add_speed);
    wish_direction * acceleration_speed
}

pub fn display_text(
    mut controller_query: Query<(&Transform, &LinearVelocity), With<LogicalPlayer>>,
    mut text_query: Query<&mut Text>,
) {
    for (transform, velocity) in &mut controller_query {
        for mut text in &mut text_query {
            **text = format!(
                "vel: {:.2}, {:.2}, {:.2}\npos: {:.2}, {:.2}, {:.2}\nspd: {:.2}",
                velocity.x,
                velocity.y,
                velocity.z,
                transform.translation.x,
                transform.translation.y,
                transform.translation.z,
                velocity.xz().length()
            );
        }
    }
}

pub fn scene_colliders(
    mut commands: Commands,
    mut main_scene: ResMut<MainScene>,
    gltf_assets: Res<Assets<Gltf>>,
    gltf_mesh_assets: Res<Assets<GltfMesh>>,
    gltf_node_assets: Res<Assets<GltfNode>>,
    mesh_assets: Res<Assets<Mesh>>,
) {
    if main_scene.is_loaded {
        return;
    }

    let gltf = gltf_assets.get(&main_scene.handle);

    if let Some(gltf) = gltf {
        let scene = gltf.scenes.first().unwrap().clone();
        commands.spawn(SceneRoot(scene));
        for node in &gltf.nodes {
            let node = gltf_node_assets.get(node).unwrap();
            if let Some(gltf_mesh) = node.mesh.clone() {
                let gltf_mesh = gltf_mesh_assets.get(&gltf_mesh).unwrap();
                for mesh_primitive in &gltf_mesh.primitives {
                    let mesh = mesh_assets.get(&mesh_primitive.mesh).unwrap();
                    commands.spawn((
                        Collider::trimesh_from_mesh(mesh).unwrap(),
                        RigidBody::Static,
                        node.transform,
                    ));
                }
            }
        }
        main_scene.is_loaded = true;
    }
}

pub fn manage_cursor(
    btn: Res<ButtonInput<MouseButton>>,
    key: Res<ButtonInput<KeyCode>>,
    mut window_query: Query<&mut Window>,
    mut controller_query: Query<&mut FpsController>,
) {
    let Ok(mut window) = window_query.get_single_mut() else { return; };
    if btn.just_pressed(MouseButton::Left) {
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        window.cursor_options.visible = false;
        for mut controller in &mut controller_query {
            controller.enable_input = true;
        }
    }
    if key.just_pressed(KeyCode::Escape) {
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
        for mut controller in &mut controller_query {
            controller.enable_input = false;
        }
    }
}
