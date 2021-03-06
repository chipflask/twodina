use bevy::prelude::*;

use super::{config::Config, state::{AppState, TransientState}};

// Tag for the menu system UI.
struct MenuUi;

pub enum MenuButton {
    OnePlayer,
    TwoPlayers,
}

pub enum MenuAction {
    Nil,
    LoadPlayers { num_players: u8 },
}

#[derive(Debug, Default)]
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
        .add_system_set(SystemSet::on_enter(AppState::Menu).with_system(setup_menu_system.system()))
        .add_system_set(SystemSet::on_exit(AppState::Menu).with_system(cleanup_menu_system.system()));
    }
}

pub fn menu_system(
    transient_state: ResMut<TransientState>,
    mut interaction_query: Query<
        (&Interaction, &mut Handle<ColorMaterial>, &MenuButton),
        (Changed<Interaction>, With<Button>),
    >,
) -> MenuAction {
    let mut action = MenuAction::Nil;
    for (interaction, mut material, button_choice) in
        interaction_query.iter_mut()
    {
        match *interaction {
            Interaction::Clicked => match button_choice {
                MenuButton::OnePlayer => {
                    action = MenuAction::LoadPlayers { num_players: 1 };
                }
                MenuButton::TwoPlayers => {
                    action = MenuAction::LoadPlayers { num_players: 2 };
                }
            },
            Interaction::Hovered => {
                *material = transient_state.button_hovered_color.clone();
            }
            Interaction::None => {
                *material = transient_state.button_pressed_color.clone();
            }
        }
    }

    action
}

fn setup_menu_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    transient_state: Res<TransientState>,
    config: Res<Config>,
) {
    commands
        // Root
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                flex_direction: FlexDirection::ColumnReverse,
                // Horizontally center child text
                justify_content: JustifyContent::Center,
                // Vertically center child text
                align_items: AlignItems::Center,
                ..Default::default()
            },
            material: materials.add(Color::NONE.into()),
            ..Default::default()
        })
        .insert(MenuUi {})
        .with_children(|parent| {
            // Title
            parent.spawn_bundle(TextBundle {
                style: Style {
                    margin: Rect::all(Val::Px(5.0)),
                    ..Default::default()
                },
                text: Text {
                    sections: vec![TextSection {
                        value: config.title.clone(),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 60.0,
                            color: Color::BLACK,
                            ..Default::default()
                        },
                    }],
                    ..Default::default()
                },
                ..Default::default()
            });

            // Start button 1 player.
            parent
                .spawn_bundle(ButtonBundle {
                    style: Style {
                        size: Size::new(Val::Px(170.0), Val::Px(65.0)),
                        margin: Rect::all(Val::Px(5.0)),
                        // Horizontally center child text
                        justify_content: JustifyContent::Center,
                        // Vertically center child text
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    material: transient_state.button_color.clone(),
                    ..Default::default()
                })
                .insert(MenuButton::OnePlayer)
                .with_children(|parent| {
                    parent.spawn_bundle(TextBundle {
                        text: Text {
                            sections: vec![TextSection {
                                value: "1 Player".to_string(),
                                style: TextStyle {
                                    font: asset_server
                                        .load("fonts/FiraSans-Bold.ttf"),
                                    font_size: 40.0,
                                    color: Color::rgb(0.9, 0.9, 0.9),
                                    ..Default::default()
                                },
                            }],
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                });

            // Start button 2 players.
            parent
                .spawn_bundle(ButtonBundle {
                    style: Style {
                        size: Size::new(Val::Px(170.0), Val::Px(65.0)),
                        margin: Rect::all(Val::Px(5.0)),
                        // Horizontally center child text
                        justify_content: JustifyContent::Center,
                        // Vertically center child text
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    material: transient_state.button_color.clone(),
                    ..Default::default()
                })
                .insert(MenuButton::TwoPlayers)
                .with_children(|parent| {
                    parent.spawn_bundle(TextBundle {
                        text: Text {
                            sections: vec![TextSection {
                                value: "2 Players".to_string(),
                                style: TextStyle {
                                    font: asset_server
                                        .load("fonts/FiraSans-Bold.ttf"),
                                    font_size: 40.0,
                                    color: Color::rgb(0.9, 0.9, 0.9),
                                    ..Default::default()
                                },
                            }],
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                });
        });
}

fn cleanup_menu_system(
    mut commands: Commands,
    query: Query<Entity, With<MenuUi>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
