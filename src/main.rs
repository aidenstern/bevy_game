mod components;
mod input;
mod movement;
mod plugin;
mod render;
mod util;

use std::f32::consts::TAU;

use avian3d::{math::Quaternion, prelude::*};
use bevy::{prelude::*, render::camera::Exposure};
use components::*;
use leafwing_input_manager::prelude::*;
use plugin::FpsControllerPlugin;
use util::*;

const SPAWN_POINT: Vec3 = Vec3::new(0.0, 5.0, 0.0);

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 10000.0,
            affects_lightmapped_meshes: false,
        })
        .insert_resource(ClearColor(Color::Srgba(Srgba::hex("D4F5F5").unwrap())))
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(DefaultPlugins)
        .add_plugins(InputManagerPlugin::<FpsActions>::default())
        .add_plugins(FpsControllerPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (manage_cursor, scene_colliders, display_text, respawn),
        )
        .run();
}

fn setup(mut commands: Commands, mut window: Query<&mut Window>, assets: Res<AssetServer>) {
    let Ok(mut window) = window.single_mut() else {
        return;
    };
    window.title = String::from("Game");

    commands.insert_resource(MainScene {
        handle: assets.load("playground.glb"),
        is_loaded: false,
    });

    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::FULL_DAYLIGHT,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 7.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Rust analyzer complains if I put the mouse motion with the key bindings in one array in the constructor
    // So I'm doing it in two steps
    let mut input_map = InputMap::default();
    input_map.insert_dual_axis(FpsActions::MousePosition, MouseMove::default());
    input_map.insert_multiple([
        (FpsActions::Forward, KeyCode::KeyW),
        (FpsActions::Backward, KeyCode::KeyS),
        (FpsActions::Left, KeyCode::KeyA),
        (FpsActions::Right, KeyCode::KeyD),
        (FpsActions::Sprint, KeyCode::ShiftLeft),
        (FpsActions::Crouch, KeyCode::ControlLeft),
        (FpsActions::Jump, KeyCode::Space),
        (FpsActions::Fly, KeyCode::AltLeft),
    ]);

    let logical_entity_collider = Collider::capsule(1.0, 0.5);

    // Note that we have two entities for the player
    // One is a "logical" player that handles the physics computation and collision
    // The other is a "render" player that is what is displayed to the user
    let logical_entity = commands
        .spawn((
            logical_entity_collider.clone(),
            Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
            Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
            LinearVelocity::ZERO,
            RigidBody::Dynamic,
            LockedAxes::ROTATION_LOCKED,
            Mass(1.0),
            GravityScale(0.0),
            Transform::from_translation(SPAWN_POINT),
            LogicalPlayer,
            FpsControllerInput {
                pitch: -TAU / 12.0,
                yaw: TAU * 5.0 / 8.0,
                ..default()
            },
            FpsController {
                air_acceleration: 80.0,
                ..default()
            },
        ))
        .insert(CameraConfig {
            height_offset: 0.0,
            radius_scale: 0.75,
        })
        .insert(input_map)
        .id();

    // Capsule cast downwards to find ground
    // Better than a ray cast as it handles when you are near the edge of a surface
    let filter = SpatialQueryFilter::default().with_excluded_entities([logical_entity]);
    let mut cast_capsule = logical_entity_collider.clone();
    cast_capsule.set_scale(Vec3::ONE * 0.99, 10);
    let shape_caster = ShapeCaster::new(
        cast_capsule,
        SPAWN_POINT,
        Quaternion::default(),
        Dir3::NEG_Y,
    )
    .with_query_filter(filter)
    .with_max_hits(10)
    .with_max_distance(6.0);

    commands.entity(logical_entity).insert(shape_caster);

    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: TAU / 5.0,
            ..default()
        }),
        Transform::default(),
        Exposure::SUNLIGHT,
        RenderPlayer { logical_entity },
    ));

    commands.spawn((
        Text::new(""),
        TextFont {
            font: assets.load("fira_mono.ttf"),
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::BLACK),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            left: Val::Px(5.0),
            ..default()
        },
    ));
}

fn respawn(mut query: Query<(&mut Transform, &mut LinearVelocity)>) {
    // println!("Respawning");
    for (mut transform, mut velocity) in &mut query {
        if transform.translation.y > -50.0 {
            continue;
        }

        *velocity = LinearVelocity::ZERO;
        transform.translation = SPAWN_POINT;
    }
}

// fn check_grounded(mut query: Query<&LinearVelocity, With<Grounded>>) {
//     for velocity in &mut query {
//         println!("Grounded {:?}", velocity);
//     }
// }
