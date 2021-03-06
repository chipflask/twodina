use bevy::prelude::*;
use crate::{
    AppState,
    core::{
        config::Config,
        dialogue::{Dialogue, DialogueEvent, DialoguePlaceholder},
        game::Game,
        menu::MenuAction
    },
    loading::LoadProgress,
};

// The UI element that displays dialogue.
pub struct DialogueWindow;

pub fn display_dialogue_system(
    mut event_reader: EventReader<DialogueEvent>,
    mut text_query: Query<&mut Text, With<Dialogue>>,
    mut visible_query: Query<&mut Visible, With<DialogueWindow>>,
) {
    for event in event_reader.iter() {
        for mut ui_text in text_query.iter_mut() {
            match event {
                DialogueEvent::End => {
                    ui_text.sections[0].value = "".to_string();
                    for mut visible in visible_query.iter_mut() {
                        visible.is_visible = false;
                    }
                }
                DialogueEvent::Text(text) => {
                    ui_text.sections[0].value = text.clone();
                    for mut visible in visible_query.iter_mut() {
                        visible.is_visible = true;
                    }
                }
            }
        }
    }
}

pub fn setup_dialogue_window_runonce (
    In(menu_action): In<MenuAction>,
    mut commands: Commands,
    mut state: ResMut<State<AppState>>,
    mut game_state: ResMut<Game>,
    asset_server: Res<AssetServer>,
    config: Res<Config>,
    mut to_load: ResMut<LoadProgress>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // todo: dialogue for each player
    let _num_players = match menu_action {
        MenuAction::Nil => return,
        MenuAction::LoadPlayers { num_players } => num_players,
    };

    state.set(AppState::Loading).expect("Set Next failed");
    to_load.next_state = AppState::InGame;

    // Load dialogue.
    let level_dialogue = to_load.add(asset_server.load(config.start_dialogue.as_path()));
    // Root node.
    commands.spawn_bundle(NodeBundle {
        style: Style {
            size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
            flex_direction: FlexDirection::Column,
            // Aligns the dialogue window to the bottom of the window.  Yes, it
            // starts from the bottom!
            justify_content: JustifyContent::FlexStart,
            // Center horizontally.
            align_items: AlignItems::Center,
            ..Default::default()
        },
        material: materials.add(Color::NONE.into()),
        ..Default::default()
    })
    .with_children(|parent| {
        // Dialogue window.
        parent.spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(95.0), Val::Px(80.0)),
                flex_direction: FlexDirection::Column,
                // Aligns text to the top of the dialogue window.  Yes, it
                // starts from the bottom, so the end is the top!
                justify_content: JustifyContent::FlexEnd,
                // Left-align text.
                align_items: AlignItems::FlexStart,
                ..Default::default()
            },
            // Brown
            material: materials.add(Color::rgba(0.804, 0.522, 0.247, 0.9).into()),
            ..Default::default()
        })
        .insert(DialogueWindow {})
        .with_children(|parent| {
            let dialogue = parent.spawn_bundle(TextBundle {
                text: Text {
                    sections: vec![TextSection {
                        value: "".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 24.0,
                            color: Color::rgb(0.2, 0.2, 0.2),
                            ..Default::default()
                        },
                    }],
                    ..Default::default()
                },
                style: Style {
                    margin: Rect::all(Val::Px(10.0)),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(DialoguePlaceholder {
                handle: level_dialogue,
                ..Default::default()
            })
            .id();
            // end: let dialogue = ...

            game_state.current_dialogue = Some(dialogue);
            // todo: use event or look for placeholder tag appearance
        });
    });
}
