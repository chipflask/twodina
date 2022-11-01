use bevy::prelude::*;
use bevy::render::camera::{Camera, OrthographicProjection};

use crate::core::geometry::Rect;
use crate::players::Player;

fn bounding_box(translation: Vec3, size: Vec2) -> Rect {
    Rect {
        left: translation.x,
        right: translation.x + size.x,
        top: translation.y,
        bottom: translation.y + size.y,
    }
}

fn viewport(
    camera_translation: Vec3,
    projection: &OrthographicProjection,
) -> Rect {
    Rect {
        left: projection.left + camera_translation.x,
        right: projection.right + camera_translation.x,
        top: projection.top + camera_translation.y,
        bottom: projection.bottom + camera_translation.y,
    }
}

// Returns true if r1 is completely contained withing r2.
fn is_rect_completely_inside(r1: &Rect, r2: &Rect) -> bool {
    r1.left > r2.left
        && r1.right < r2.right
        && r1.bottom > r2.bottom
        && r1.top < r2.top
}

pub(crate) fn update_camera_system(
    mut player_query: Query<(&GlobalTransform, &Player)>,
    mut camera_query: Query<
        (&mut Transform, &GlobalTransform, &OrthographicProjection),
        With<Camera>,
    >,
) {
    for (player_transform, player) in player_query.iter_mut() {
        // Is sprite in view frame?
        // println!("player {:?}", player_transform.translation);
        let char_translation = player_transform.translation();
        // TODO: Use player scaling.
        let char_rect = bounding_box(
            char_translation,
            Vec2::new(player.width, player.height),
        );
        // println!("char_rect {:?}", char_rect);
        for (mut camera_transform, camera_global, projection) in
            camera_query.iter_mut()
        {
            // println!("projection {:?}", projection);
            let camera_rect = viewport(camera_global.translation(), projection);
            // println!("camera_rect {:?}", camera_rect);
            let is_player_in_view =
                is_rect_completely_inside(&char_rect, &camera_rect);
            if !is_player_in_view {
                camera_transform.translation = char_translation;
            }
        }
    }
}
