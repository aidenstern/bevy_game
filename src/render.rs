use bevy::prelude::*;
use bevy_xpbd_3d::plugins::collision::Collider;
use super::components::*;

pub fn fps_controller_render(
    mut render_query: Query<(&mut Transform, &RenderPlayer), With<RenderPlayer>>,
    logical_query: Query<
        (&Transform, &Collider, &FpsController, &CameraConfig),
        (With<LogicalPlayer>, Without<RenderPlayer>),
    >, 
) {
    for (mut render_transform, render_player) in render_query.iter_mut() {
        if let Ok((logical_transform, collider, controller, camera_config)) =
            logical_query.get(render_player.logical_entity)
        {
            if let Some(capsule) = collider.shape_scaled().as_capsule() {
                let camera_height = capsule.segment.b.y
                    + capsule.radius * camera_config.radius_scale
                    + camera_config.height_offset;
                render_transform.translation =
                    logical_transform.translation + Vec3::Y * camera_height;
                render_transform.rotation =
                    Quat::from_euler(EulerRot::YXZ, controller.yaw, controller.pitch, 0.0);
            }
        }
    }
}
