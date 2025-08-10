use super::components::*;
use avian3d::prelude::Collider;
use bevy::prelude::*;

pub fn fps_controller_render(
    mut render_query: Query<(&mut Transform, &RenderPlayer), With<RenderPlayer>>,
    logical_query: Query<
        (&Transform, &Collider, &FpsController, &CameraConfig),
        (With<LogicalPlayer>, Without<RenderPlayer>),
    >,
) {
    for (mut render_transform, render_player) in render_query.iter_mut() {
        if let Ok((logical_transform, _collider, controller, camera_config)) =
            logical_query.get(render_player.logical_entity)
        {
            let camera_height = (controller.height / 2.0) + camera_config.height_offset;

            render_transform.translation = logical_transform.translation + Vec3::Y * camera_height;
            render_transform.rotation =
                Quat::from_euler(EulerRot::YXZ, controller.yaw, controller.pitch, 0.0);
        }
    }
}
