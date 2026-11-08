use dioxus::prelude::*;
use lazygo::session::Session;

mod ui;

use ui::{
    AnalysisPanel, AnalysisUiState, Board, ConfirmButton, HistoryButton, HistoryDirection,
    KomiField, ResetButton, RuleMenu, SettingsPage, SizeMenu, Topbar, TurnIndicator,
};

const MAIN_CSS: &str = include_str!("../assets/main.css");
const APP_ICON: Asset = asset!("/assets/icon.png");
const APP_FONT: Asset = asset!("/assets/fonts/maiyuan-ii-regular.ttf");

fn main() {
    dioxus::launch(App);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Game,
    Settings,
}

#[component]
fn App() -> Element {
    let mut session = use_signal(Session::new);
    let mut analysis = use_signal(AnalysisUiState::new);
    let mut page = use_signal(|| Page::Game);

    use_future(move || async move {
        loop {
            futures_timer::Delay::new(std::time::Duration::from_millis(120)).await;
            let state = session.read().clone();
            if analysis.read().needs_tick(&state) {
                analysis.with_mut(|analysis| analysis.tick(&state));
            }
        }
    });

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
    let current_index = game.cursor();
    let move_history = game.all_moves().to_vec();
    let (analysis_enabled, analysis_open, perspective, mode, current_snapshot, analysis_error) = {
        let analysis = analysis.read();
        (
            analysis.enabled(),
            analysis.panel_open(),
            analysis.perspective(),
            analysis.mode(),
            analysis.current().cloned(),
            analysis.error().map(str::to_string),
        )
    };
    let mut chart = analysis.read().chart(&state);
    if let Some(snapshot) = current_snapshot.as_ref() {
        if current_index < chart.len() {
            chart[current_index] = Some(snapshot.clone());
        }
    } else if current_index > 0 && current_index < chart.len() && chart[current_index].is_none() {
        // Keep the new turn visible while KataGo is still evaluating it.
        if let Some(previous) = chart[..current_index]
            .iter()
            .rev()
            .find_map(|snapshot| snapshot.clone())
        {
            chart[current_index] = Some(previous);
        }
    }
    let ownership = analysis_open
        .then(|| {
            current_snapshot
                .as_ref()
                .map(|snapshot| snapshot.ownership.clone())
        })
        .flatten();
    let max_visits = analysis.read().max_visits();
    let settings_open = page() == Page::Settings;
    let board_margin = format!("--board-margin: {:.6}vw;", 100.0 / (size as f64 + 2.0));
    let font_face = format!(
        r#"@font-face {{ font-family: "Maiyuan II"; src: url("{}") format("truetype"); font-style: normal; font-weight: 400; font-display: swap; }}"#,
        APP_FONT
    );

    rsx! {
        style { "{MAIN_CSS}\n{font_face}" }
        main { class: "page",
            Topbar {
                on_settings: move |_| {
                    page.set(if page() == Page::Settings {
                        Page::Game
                    } else {
                        Page::Settings
                    });
                },
            }
            if !settings_open {
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
                if pending.is_some() && !state.pass_armed() && !state.reset_armed() {
                    div {
                        class: "pending-backdrop",
                        onclick: move |_| session.write().cancel_pending(),
                    }
                }
            }
            if settings_open {
                SettingsPage {
                    max_visits,
                    on_changed: move |_| analysis.write().reload(),
                    on_depth: move |value| analysis.write().set_max_visits(value),
                }
            } else {
            section { class: "content", style: "{board_margin}",
                div { class: "board-column",
                    Board {
                        game,
                        pending,
                        ownership,
                        on_select: move |(x, y)| session.write().select(x, y),
                        on_cancel: move |_| session.write().cancel_pending(),
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
                    div { class: "analysis-row",
                        AnalysisPanel {
                            open: analysis_open,
                            enabled: analysis_enabled,
                            perspective,
                            mode,
                            error: analysis_error,
                            chart,
                            current_index,
                            moves: move_history,
                            on_toggle: move |_| analysis.write().toggle_panel(),
                            on_perspective: move |_| analysis.write().toggle_perspective(),
                            on_mode: move |_| analysis.write().toggle_mode(),
                            on_replay: move |turn| session.write().replay_to(turn),
                        }
                    }
                }
            }
            }
        }
    }
}
