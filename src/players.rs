use bevy::{math::Vec3Swizzles, prelude::*};

use crate::{DEBUG_MODE_DEFAULT, TransientState,
    character::{AnimatedSprite, Character},
    collider::{Collider, ColliderBehavior},
    loading::LoadProgress,
    items::Inventory,
    menu::MenuAction,
    motion::z_from_y,
    // todo: debug::{}
    PlayerPositionDisplay,
    Debuggable,
};


pub struct Player {
    pub id: u32,
}

// for 'naked base'
// const PLAYER_WIDTH: f32 = 26.0;
// const PLAYER_HEIGHT: f32 = 36.0;
pub const PLAYER_WIDTH: f32 = 31.0;
pub const PLAYER_HEIGHT: f32 = 32.0;

pub fn setup_players_runonce(
    In(menu_action): In<MenuAction>,
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    transient_state: Res<TransientState>,
    mut to_load: ResMut<LoadProgress>,
) -> MenuAction {
    let num_players = match menu_action {
        MenuAction::Nil => return menu_action,
        MenuAction::LoadPlayers { num_players } => num_players,
    };

    // Players.
    for i in 0..num_players {
        let texture_handle = to_load.add(asset_server.load(format!("sprites/azuna{}.png", i + 1).as_str()));
        let texture_atlas = TextureAtlas::from_grid(
            texture_handle,
            Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT),
            4,
            8,
        );
        let texture_atlas_handle = texture_atlases.add(texture_atlas);
        let scale = Vec3::splat(4.0);
        let collider_size = Vec2::new(13.0, 4.5);
        let collider_offset = Vec2::new(0.0, -12.5);
        // This should match the move_character_system.
        let initial_z = z_from_y(collider_offset.y);
        commands
            .spawn(SpriteSheetBundle {
                texture_atlas: texture_atlas_handle,
                transform: Transform::from_scale(scale)
                    .mul_transform(Transform::from_translation(
                        Vec3::new(PLAYER_WIDTH * i as f32 + 20.0, 0.0, initial_z))),
                ..Default::default()
            })
            .with(AnimatedSprite::with_frame_seconds(0.1))
            .with(Character::default())
            .with(Player { id: u32::from(i) })
            .with(Inventory::default())
            .with(Collider::new(
                ColliderBehavior::Obstruct,
                collider_size * scale.xy(),
                collider_offset * scale.xy(),
            ))
            .with_children(|parent| {
                // add a shadow sprite -- is there a more efficient way where we load this just once??
                let shadow_handle = to_load.add(asset_server.load("sprites/shadow.png"));
                parent.spawn(SpriteBundle {
                    transform: Transform {
                        translation: Vec3::new(0.0, -13.0, -0.01),
                        scale: Vec3::splat(0.7),
                        ..Default::default()
                    },
                    material: materials.add(shadow_handle.into()),
                    ..Default::default()
                });
                // collider debug indicator - TODO: refactor into Collider::new_with_debug(parent, collider_size, scale)
                parent.spawn(SpriteBundle {
                    material: transient_state.default_blue.clone(),
                    // Don't scale here since the whole character will be scaled.
                    sprite: Sprite::new(collider_size),
                    transform: Transform::from_translation(Vec3::new(collider_offset.x, collider_offset.y, 0.0)),
                    visible: Visible {
                        is_transparent: true,
                        is_visible: DEBUG_MODE_DEFAULT,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with(Debuggable::default());
                // Center debug indicator.
                parent.spawn(SpriteBundle {
                    material: transient_state.default_red.clone(),
                    // Don't scale here since the whole character will be scaled.
                    sprite: Sprite::new(Vec2::new(5.0, 5.0)),
                    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 100.0)),
                    visible: Visible {
                        is_transparent: true,
                        is_visible: DEBUG_MODE_DEFAULT,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with(Debuggable::default());
            })
            .spawn(TextBundle {
                text: Text {
                    sections: vec![TextSection {
                        value: "Position:".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 24.0,
                            color: Color::rgb(0.7, 0.7, 0.7),
                            ..Default::default()
                        },
                    }],
                    ..Default::default()
                },
                style: Style {
                    position_type: PositionType::Absolute,
                    position: Rect {
                        top: Val::Px(5.0 + i as f32 * 20.0),
                        left: Val::Px(5.0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                visible: Visible {
                    is_transparent: true,
                    is_visible: false,
                    ..Default::default()
                },
                ..Default::default()
            })
            .with(PlayerPositionDisplay { player_id: u32::from(i) })
            .with(Debuggable::default());
    }
    menu_action
}
