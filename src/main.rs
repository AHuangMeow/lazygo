use dioxus::prelude::*;
use lazygo::game::{Game, DEFAULT_SIZE};

mod ui;

use ui::{
    Board, ConfirmButton, HistoryButton, HistoryDirection, KomiField, ResetButton, RuleMenu,
    SizeMenu, Topbar, TurnIndicator, DEFAULT_RULE,
};

const MAIN_CSS: &str = include_str!("../assets/main.css");
const APP_ICON: Asset = asset!("/assets/icon.png");
const APP_FONT: Asset = asset!("/assets/fonts/maiyuan-ii-regular.ttf");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut game = use_signal(|| Game::new(DEFAULT_SIZE));
    let mut pending = use_signal(|| None::<(usize, usize)>);
    let mut menu_open = use_signal(|| false);
    let mut reset_armed = use_signal(|| false);
    let mut reset_rotation = use_signal(|| 0i32);
    let mut rule = use_signal(|| DEFAULT_RULE.to_string());
    let mut rule_open = use_signal(|| false);
    let mut komi = use_signal(|| "6.5".to_string());
    let current = game();
    let size = current.size();
    let board_margin = format!("--board-margin: {:.6}vw;", 100.0 / (size as f64 + 2.0));
    let initial = current.is_initial();
    let font_face = format!(
        r#"@font-face {{ font-family: "Maiyuan II"; src: url("{}") format("truetype"); font-style: normal; font-weight: 400; font-display: swap; }}"#,
        APP_FONT
    );

    rsx! {
        style { "{MAIN_CSS}\n{font_face}" }
        main { class: "page",
            Topbar {}
            if reset_armed() {
                div {
                    class: "reset-backdrop",
                    onclick: move |_| {
                        reset_armed.set(false);
                        reset_rotation.set(reset_rotation() - 360);
                    },
                }
            }
            section { class: "content", style: "{board_margin}",
                div { class: "board-column",
                    Board {
                        game: current.clone(),
                        pending: pending(),
                        on_select: move |(x, y)| {
                            if pending() == Some((x, y)) {
                                pending.set(None);
                            } else if game().play(x, y).is_some() {
                                pending.set(Some((x, y)));
                            }
                        },
                    }
                    div { class: "board-footer",
                        div { class: "board-actions",
                            TurnIndicator {
                                turn: current.turn(),
                                on_pass: move |_| {
                                    game.set(game().pass());
                                    pending.set(None);
                                    reset_armed.set(false);
                                    menu_open.set(false);
                                },
                            }
                            ResetButton {
                                armed: reset_armed(),
                                rotation: reset_rotation(),
                                on_click: move |_| {
                                    reset_rotation.set(reset_rotation() + 360);
                                    if reset_armed() {
                                        game.set(Game::new(size));
                                        pending.set(None);
                                        reset_armed.set(false);
                                    } else {
                                        reset_armed.set(true);
                                    }
                                    menu_open.set(false);
                                },
                            }
                            HistoryButton {
                                direction: HistoryDirection::Undo,
                                enabled: current.can_undo(),
                                on_click: move |_| {
                                    if let Some(next) = game().undo() {
                                        game.set(next);
                                        pending.set(None);
                                        reset_armed.set(false);
                                        menu_open.set(false);
                                    }
                                },
                            }
                            HistoryButton {
                                direction: HistoryDirection::Redo,
                                enabled: current.can_redo(),
                                on_click: move |_| {
                                    if let Some(next) = game().redo() {
                                        game.set(next);
                                        pending.set(None);
                                        reset_armed.set(false);
                                        menu_open.set(false);
                                    }
                                },
                            }
                        }
                        ConfirmButton {
                            enabled: pending().is_some(),
                            on_click: move |_| {
                                if let Some((x, y)) = pending() {
                                    if let Some(next) = game().play(x, y) {
                                        game.set(next);
                                    }
                                    pending.set(None);
                                    reset_armed.set(false);
                                    menu_open.set(false);
                                }
                            },
                        }
                    }
                    div { class: "board-settings",
                        div { class: "settings-group",
                            RuleMenu {
                                value: rule(),
                                open: rule_open(),
                                on_toggle: move |_| rule_open.set(!rule_open()),
                                on_select: move |value| {
                                    rule.set(value);
                                    rule_open.set(false);
                                },
                            }
                            KomiField {
                                value: komi(),
                                on_input: move |value| komi.set(value),
                            }
                        }
                        SizeMenu {
                            size,
                            initial,
                            open: menu_open(),
                            on_toggle: move |_| menu_open.set(!menu_open()),
                            on_select: move |option| {
                                game.set(Game::new(option));
                                pending.set(None);
                                menu_open.set(false);
                                reset_armed.set(false);
                            },
                        }
                    }
                }
            }
        }
    }
}
