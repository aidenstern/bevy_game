mod components;
mod input;
mod movement;
mod plugin;
mod render;
mod util;

use std::f32::consts::TAU;

use bevy::{core::Zeroable, prelude::*, render::camera::Exposure, window::PrimaryWindow};
use bevy_xpbd_3d::prelude::*;
use components::*;
use leafwing_input_manager::prelude::*;
use plugin::FpsControllerPlugin;
use util::*;

const SPAWN_POINT: Vec3 = Vec3::new(0.0, 50.0, 0.0);

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 10000.0,
        })
        .insert_resource(ClearColor(Color::hex("D4F5F5").unwrap()))
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

fn setup(
    mut commands: Commands,
    mut window: Query<&mut Window>,
    other_window: Query<Entity, With<PrimaryWindow>>,
    assets: Res<AssetServer>,
) {
    let mut window = window.single_mut();
    window.title = String::from("Minimal FPS Controller Example");

    commands.insert_resource(MainScene {
        handle: assets.load("playground.glb"),
        is_loaded: false,
    });

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: light_consts::lux::FULL_DAYLIGHT,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 7.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    let input_map = InputMap::new([(FpsActions::MousePosition, DualAxis::mouse_motion())])
        .insert_multiple([
            (FpsActions::Forward, KeyCode::KeyW),
            (FpsActions::Backward, KeyCode::KeyS),
            (FpsActions::Left, KeyCode::KeyA),
            (FpsActions::Right, KeyCode::KeyD),
            (FpsActions::Sprint, KeyCode::ShiftLeft),
            (FpsActions::Crouch, KeyCode::ControlLeft),
            (FpsActions::Jump, KeyCode::Space),
        ])
        .build();

    // Note that we have two entities for the player
    // One is a "logical" player that handles the physics computation and collision
    // The other is a "render" player that is what is displayed to the user
    // This distinction is useful for later on if you want to add multiplayer,
    // where often time these two ideas are not exactly synced up
    let logical_entity = commands
        .spawn((
            Collider::capsule(1.0, 0.5),
            Friction {
                dynamic_coefficient: 0.0,
                static_coefficient: 0.0,
                combine_rule: CoefficientCombine::Min,
            },
            Restitution {
                coefficient: 0.0,
                combine_rule: CoefficientCombine::Min,
            },
            // ActiveEvents::COLLISION_EVENTS,
            LinearVelocity::ZERO,
            RigidBody::Dynamic,
            LockedAxes::ROTATION_LOCKED,
            Mass(1.0),
            GravityScale(0.0),
            // Ccd { enabled: true }, // Prevent clipping when going fast
            TransformBundle::from_transform(Transform::from_translation(SPAWN_POINT)),
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
        .insert(InputManagerBundle::with_map(input_map))
        .id();

    // Capsule cast downwards to find ground
    // Better than a ray cast as it handles when you are near the edge of a surface
    let filter = SpatialQueryFilter::default().with_excluded_entities([logical_entity]);
    let cast_capsule = Collider::capsule(1.0, 0.45);
    let shape_caster = ShapeCaster::new(
        cast_capsule,
        SPAWN_POINT,
        Quat::zeroed(),
        Direction3d::NEG_Y,
    )
    .with_query_filter(filter);
    commands.entity(logical_entity).insert(shape_caster);

    commands.spawn((
        Camera3dBundle {
            projection: Projection::Perspective(PerspectiveProjection {
                fov: TAU / 5.0,
                ..default()
            }),
            exposure: Exposure::SUNLIGHT,
            ..default()
        },
        RenderPlayer { logical_entity },
    ));

    commands.spawn(
        TextBundle::from_section(
            "",
            TextStyle {
                font: assets.load("fira_mono.ttf"),
                font_size: 24.0,
                color: Color::BLACK,
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            left: Val::Px(5.0),
            ..default()
        }),
    );
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
