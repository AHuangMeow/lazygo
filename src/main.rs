use dioxus::prelude::*;
use lazygo::session::Session;

mod ui;

use ui::{
    Board, ConfirmButton, HistoryButton, HistoryDirection, KomiField, ResetButton, RuleMenu,
    SizeMenu, Topbar, TurnIndicator,
};

const MAIN_CSS: &str = include_str!("../assets/main.css");
const APP_ICON: Asset = asset!("/assets/icon.png");
const APP_FONT: Asset = asset!("/assets/fonts/maiyuan-ii-regular.ttf");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut session = use_signal(Session::new);
    let state = session();
    let current = state.game();
    let size = current.size();
    let rules = current.rules();
    let locked = state.settings_locked();
    let pending = state.pending();
    let komi = state.komi().to_string();
    let turn = current.turn();
    let can_undo = current.can_undo();
    let can_redo = current.can_redo();
    let game = current.clone();
    let board_margin = format!("--board-margin: {:.6}vw;", 100.0 / (size as f64 + 2.0));
    let font_face = format!(
        r#"@font-face {{ font-family: "Maiyuan II"; src: url("{}") format("truetype"); font-style: normal; font-weight: 400; font-display: swap; }}"#,
        APP_FONT
    );

    rsx! {
        style { "{MAIN_CSS}\n{font_face}" }
        main { class: "page",
            Topbar {}
            if state.pass_armed() {
                div {
                    class: "pass-backdrop",
                    onclick: move |_| session.write().cancel_pass(),
                }
            }
            if state.reset_armed() {
                div {
                    class: "reset-backdrop",
                    onclick: move |_| session.write().cancel_reset(),
                }
            }
            section { class: "content", style: "{board_margin}",
                div { class: "board-column",
                    Board {
                        game,
                        pending,
                        on_select: move |(x, y)| session.write().select(x, y),
                    }
                    div { class: "board-footer",
                        div { class: "board-actions",
                            TurnIndicator {
                                turn,
                                open: state.pass_armed(),
                                on_click: move |_| session.write().toggle_pass(),
                            }
                            ResetButton {
                                armed: state.reset_armed(),
                                rotation: state.reset_rotation(),
                                on_click: move |_| session.write().toggle_reset(),
                            }
                            HistoryButton {
                                direction: HistoryDirection::Undo,
                                enabled: can_undo,
                                on_click: move |_| session.write().undo(),
                            }
                            HistoryButton {
                                direction: HistoryDirection::Redo,
                                enabled: can_redo,
                                on_click: move |_| session.write().redo(),
                            }
                        }
                        ConfirmButton {
                            enabled: state.can_confirm(),
                            on_click: move |_| session.write().confirm(),
                        }
                    }
                    div { class: "board-settings",
                        div { class: "settings-group",
                            RuleMenu {
                                value: rules,
                                locked,
                                open: state.rule_open(),
                                on_toggle: move |_| session.write().toggle_rule_menu(),
                                on_select: move |rules| session.write().set_rules(rules),
                            }
                            KomiField {
                                value: komi,
                                locked,
                                on_input: move |value| session.write().set_komi(value),
                            }
                        }
                        SizeMenu {
                            size,
                            locked,
                            open: state.size_open(),
                            on_toggle: move |_| session.write().toggle_size_menu(),
                            on_select: move |size| session.write().set_size(size),
                        }
                    }
                }
            }
        }
    }
}
