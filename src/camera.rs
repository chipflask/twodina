use bevy::{
    prelude::*,
    render::{camera::{self, Camera, CameraProjection, OrthographicProjection}, render_graph},
};

use crate::players::Player;

const CAMERA_BUFFER: f32 = 1.0;

// We have multiple cameras, so this one marks the camera that follows the
// player.
pub struct PlayerCamera;

pub fn initialize_camera (
    commands: &mut Commands
) {
    let far = 2000.0;
    let near = -2000.0;
    commands
        .spawn_bundle(OrthographicCameraBundle {
            camera: Camera {
                name: Some(render_graph::base::camera::CAMERA_2D.to_string()),
                ..Default::default()
            },
            orthographic_projection: OrthographicProjection {
                near,
                far,
                depth_calculation: camera::DepthCalculation::ZDifference,
                ..Default::default()
            },
            visible_entities: Default::default(),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            global_transform: Default::default(),
        })
        .insert(PlayerCamera {});
    commands
        .spawn_bundle(UiCameraBundle::default());
}


fn bounding_box(translation: Vec3, size: Vec2) -> Rect<f32> {
    let half_width = size.x / 2.0;
    let half_height = size.y / 2.0;
    Rect {
        left: translation.x - half_width,
        right: translation.x + half_width,
        top: translation.y + half_height,
        bottom: translation.y - half_height,
    }
}

// Returns the bounding box that includes both the given bounding boxes.
fn expand_bounding_box(r1: &Rect<f32>, r2: &Rect<f32>) -> Rect<f32> {
    Rect {
        left: r1.left.min(r2.left),
        right: r1.right.max(r2.right),
        top: r1.top.max(r2.top),
        bottom: r1.bottom.min(r2.bottom),
    }
}

fn rect_center(r: &Rect<f32>) -> Vec2 {
    // Don't overflow.
    Vec2::new(r.left + 0.5 * (r.right - r.left), r.bottom + 0.5 * (r.top - r.bottom))
}

#[allow(dead_code)]
fn rect_half_width_height(r: &Rect<f32>) -> Vec2 {
    Vec2::new(0.5 * (r.right - r.left), 0.5 * (r.top - r.bottom))
}

fn rect_width_height(r: &Rect<f32>) -> Vec2 {
    Vec2::new(r.right - r.left, r.top - r.bottom)
}

fn rect_expand_by(r: &Rect<f32>, amount: f32) -> Rect<f32> {
    Rect {
        left: r.left - amount,
        right: r.right + amount,
        top: r.top + amount,
        bottom: r.bottom - amount,
    }
}

// wh is width and height.
// aspect_ratio is the desired width / height.
fn expanded_to_aspect_ratio(wh: &Vec2, aspect_ratio: f32) -> Vec2 {
    let h_based_on_w = wh.x / aspect_ratio;
    if h_based_on_w > wh.y {
        Vec2::new(wh.x, h_based_on_w)
    } else {
        let w_based_on_h = wh.y * aspect_ratio;

        Vec2::new(w_based_on_h, wh.y)
    }
}

fn viewport(camera_transform: &GlobalTransform, projection: &OrthographicProjection) -> Rect<f32> {
    let translation = &camera_transform.translation;
    Rect {
        left: projection.left + translation.x,
        right: projection.right + translation.x,
        top: projection.top + translation.y,
        bottom: projection.bottom + translation.y,
    }
}

// Returns true if r1 is completely contained withing r2.
fn is_rect_completely_inside(r1: &Rect<f32>, r2: &Rect<f32>) -> bool {
    r1.left > r2.left && r1.right < r2.right &&
    r1.bottom > r2.bottom && r1.top < r2.top
}

pub fn update_camera_system(
    windows: Res<Windows>,
    mut player_query: Query<(&GlobalTransform, &Player)>,
    mut camera_query: Query<(&mut Transform,
                            &GlobalTransform,
                            &mut OrthographicProjection,
                            &mut Camera),
                            With<PlayerCamera>>,
) {
    // Amount of margin between edge of view and character.
    let margin_1p = 75.0;
    let margin = 100.0;

    // Get bounding box of all players.
    let mut full_bb = None;
    let mut num_players = 0;
    let mut player_translation = Vec3::ZERO;
    for (player_transform, player) in player_query.iter_mut() {
        num_players += 1;
        // Is sprite in view frame?
        // println!("player translation {:?}", player_transform.translation);
        let char_translation = player_transform.translation;
        let char_size = Vec2::new(player.width * player_transform.scale.x, player.height * player_transform.scale.y);
        let char_rect = bounding_box(char_translation, char_size);
        // println!("char_rect {:?}", char_rect);
        full_bb = match full_bb {
            None => {
                player_translation = player_transform.translation;
                Some(char_rect)
            }
            Some(bb) => Some(expand_bounding_box(&bb, &char_rect)),
        };
    }

    if let Some(full_bb) = full_bb {
        let window = windows.get_primary().expect("should be at least one window so we can compute aspect ratio");
        let win_width = window.width();
        let win_height = window.height();
        let aspect_ratio = win_width / win_height;
        let margin_amount = if num_players <= 1 { margin_1p } else { margin };
        // Add margin.
        // TODO: Handle case when window is smaller than margin.
        let full_bb = rect_expand_by(&full_bb, margin_amount);

        // 1.2 is damping so we reach steady state instead of cycling
        let margin_vec =  Vec3::new(
            (win_width - margin_amount * 1.2) / win_width,
            (win_height - margin_amount * 1.2) / win_height, 1.0);

        for (mut camera_transform, camera_global, mut projection, mut camera) in camera_query.iter_mut() {
            // println!("projection {:?}", projection);
            // println!("camera_transform {:?}", camera_transform);
            // println!("camera_global {:?}", camera_global);
            // Note: We don't support camera rotation or scale.
            let camera_rect = viewport(&camera_global, &projection);
            // println!("camera_rect {:?}", camera_rect);
            if num_players <= 1 {
                // Center on the player if not in view.
                let is_player_in_view = is_rect_completely_inside(&full_bb, &camera_rect);
                if !is_player_in_view {
                    // Mutate the transform, never the global transform.
                    let mut v1 = camera_transform.translation.clone() - player_translation;

                    if v1.length() > (win_width * win_width + win_height * win_height).sqrt() / 3.0 {
                        camera_transform.translation = player_translation;
                    } else {
                        let mut new_cam_translation = camera_transform.translation.clone();
                        v1.x = margin_vec.x.min(((v1.x.abs() - CAMERA_BUFFER) / win_width).abs()) * v1.x.signum() * win_width;
                        v1.y = margin_vec.y.min(((v1.y.abs() - CAMERA_BUFFER) / win_height).abs()) * v1.y.signum() * win_height;
                        // println!("{:?} - {:?}", v1, margin_vec);
                        new_cam_translation = new_cam_translation - v1 * 2.0;
                        new_cam_translation.z = camera_transform.translation.z;
                        camera_transform.translation = new_cam_translation;
                    }


                }
            } else {
                // Center on the center of the bounding box of all players.
                let c = rect_center(&full_bb);
                camera_transform.translation.x = c.x;
                camera_transform.translation.y = c.y;

                // Zoom so that all players are in view.
                let mut wh = rect_width_height(&full_bb);
                wh = expanded_to_aspect_ratio(&wh, aspect_ratio);
                // Never zoom in smaller than the window.
                if wh.x < win_width || wh.y < win_height {
                    wh = Vec2::new(win_width, win_height);
                }
                projection.update(wh.x, wh.y);
                camera.projection_matrix = projection.get_projection_matrix();
                camera.depth_calculation = projection.depth_calculation();
            }
        }
    }
}
