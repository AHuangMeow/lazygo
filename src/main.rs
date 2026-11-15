use dioxus::prelude::*;
use lazygo::session::Session;

mod ui;

use ui::{
    AnalysisPanel, AnalysisUiState, Board, ConfirmButton, HistoryButton, HistoryDirection,
    KomiField, PickedSgf, ResetButton, RuleMenu, SettingsPage, SizeMenu, Topbar, TurnIndicator,
    PICK_SGF_SCRIPT,
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
    let mut sgf_busy = use_signal(|| false);
    let mut page = use_signal(|| Page::Game);
    let mut rule_open = use_signal(|| false);
    let mut size_open = use_signal(|| false);
    let mut reset_rotation = use_signal(|| 0);

    use_future(move || async move {
        loop {
            futures_timer::Delay::new(std::time::Duration::from_millis(120)).await;
            let state = session.read();
            let needs_tick = analysis.read().needs_tick(&state);
            if needs_tick {
                analysis.with_mut(|analysis| analysis.tick(&state));
            }
        }
    });

    let state = session();
    let current = state.game();
    let size = current.size();
    let rules = current.rules();
    let locked = state.settings_locked();
    let reset_armed = state.reset_armed();
    let pending = state.pending();
    let pending_clickable = pending.is_some() && !state.pass_armed() && !state.reset_armed();
    let komi = state.komi();
    let turn = current.turn();
    let can_undo = current.can_undo();
    let can_redo = current.can_redo();
    let board = current.view();
    let current_index = current.cursor();
    let move_history = current.all_moves().to_vec();
    let (
        analysis_enabled,
        analysis_open,
        perspective,
        mode,
        current_snapshot,
        current_matches,
        show_candidates,
        candidate_count,
        analysis_error,
    ) = {
        let analysis = analysis.read();
        (
            analysis.enabled(),
            analysis.panel_open(),
            analysis.perspective(),
            analysis.mode(),
            analysis.current().cloned(),
            analysis.current_matches(&state),
            analysis.show_candidates(),
            analysis.candidate_count(),
            analysis.error().map(str::to_string),
        )
    };
    let mut chart = analysis.read().chart(&state);
    if current_matches {
        if let Some(snapshot) = current_snapshot.as_ref() {
            if current_index < chart.len() {
                chart[current_index] = Some(snapshot.clone());
            }
        }
    }
    if current_index > 0 && current_index < chart.len() && chart[current_index].is_none() {
        // Keep the new turn visible while KataGo is still evaluating it.
        if let Some(previous) = chart[..current_index]
            .iter()
            .rev()
            .find_map(|snapshot| snapshot.clone())
        {
            chart[current_index] = Some(previous);
        }
    }
    let ownership = (analysis_open && !show_candidates)
        .then(|| {
            current_matches.then(|| {
                current_snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.ownership.clone())
            })?
        })
        .flatten();
    let candidates = (analysis_open && show_candidates && current_matches)
        .then(|| {
            current_snapshot.as_ref().map(|snapshot| {
                snapshot
                    .candidates
                    .iter()
                    .take(candidate_count as usize)
                    .cloned()
                    .collect::<Vec<_>>()
            })
        })
        .flatten();
    let max_visits = analysis.read().max_visits();
    let settings_open = page() == Page::Settings;
    let board_margin = format!(
        "--board-margin: {:.6}vw;",
        100.0 / (size.get() as f64 + 2.0)
    );
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
                        onclick: move |_| {
                            session.write().cancel_reset();
                            reset_rotation.with_mut(|rotation| *rotation -= 360);
                        },
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
                    candidate_count,
                    on_changed: move |_| analysis.write().reload(),
                    on_depth: move |value| analysis.write().set_max_visits(value),
                    on_candidate_count: move |value| analysis.write().set_candidate_count(value),
                }
            } else {
            section { class: "content", style: "{board_margin}",
                div { class: "board-column",
                    Board {
                        board,
                        pending,
                        pending_clickable,
                        ownership,
                        candidates,
                        on_select: move |(x, y)| session.write().select(x, y),
                        on_cancel: move |_| session.write().cancel_pending(),
                    }
                    div { class: "board-footer",
                        div { class: "board-actions",
                            TurnIndicator {
                                turn,
                                open: state.pass_armed(),
                                on_click: move |_| {
                                    rule_open.set(false);
                                    size_open.set(false);
                                    session.write().toggle_pass();
                                },
                            }
                            ResetButton {
                                armed: state.reset_armed(),
                                rotation: reset_rotation(),
                                on_click: move |_| {
                                    rule_open.set(false);
                                    size_open.set(false);
                                    reset_rotation.with_mut(|rotation| *rotation += 360);
                                    session.write().toggle_reset();
                                    if reset_armed {
                                        analysis.write().start_new_game(0);
                                    }
                                },
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
                                open: rule_open(),
                                on_toggle: move |_| {
                                    size_open.set(false);
                                    session.write().cancel_pass();
                                    rule_open.toggle();
                                },
                                on_select: move |rules| {
                                    rule_open.set(false);
                                    size_open.set(false);
                                    if locked {
                                        return;
                                    }
                                    session.write().set_rules(rules);
                                    analysis.write().start_new_game(0);
                                },
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
                            open: size_open(),
                            on_toggle: move |_| {
                                rule_open.set(false);
                                session.write().cancel_pass();
                                size_open.toggle();
                            },
                            on_select: move |size| {
                                rule_open.set(false);
                                size_open.set(false);
                                if locked {
                                    return;
                                }
                                session.write().set_size(size);
                                analysis.write().start_new_game(0);
                            },
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
                            show_candidates,
                            on_toggle: move |_| analysis.write().toggle_panel(),
                            on_perspective: move |_| analysis.write().toggle_perspective(),
                            on_mode: move |_| analysis.write().toggle_mode(),
                            on_candidates: move |_| analysis.write().toggle_candidates(),
                            on_replay: move |turn| session.write().replay_to(turn),
                        }
                        button {
                            class: "sgf-button",
                            r#type: "button",
                            disabled: sgf_busy(),
                            onclick: move |_| {
                                if sgf_busy() {
                                    return;
                                }
                                sgf_busy.set(true);

                                if locked {
                                    let text = session.read().sgf_text();
                                    let filename = lazygo::sgf::suggested_filename();
                                    spawn(async move {
                                        let result = tokio::task::spawn_blocking(move || {
                                            lazygo::sgf::save_to_downloads(&filename, &text)
                                        })
                                        .await;
                                        match result {
                                            Ok(Ok(_)) => {}
                                            Ok(Err(error)) => eprintln!("SGF 保存失败: {error}"),
                                            Err(error) => eprintln!("SGF 保存任务失败: {error}"),
                                        }
                                        sgf_busy.set(false);
                                    });
                                } else {
                                    spawn(async move {
                                        let mut eval = document::eval(PICK_SGF_SCRIPT);
                                        let message = match eval.recv::<String>().await {
                                            Ok(message) => message,
                                            Err(_) => {
                                                eprintln!("SGF 文件选择失败");
                                                sgf_busy.set(false);
                                                return;
                                            }
                                        };
                                        let picked: PickedSgf = match serde_json::from_str(&message) {
                                            Ok(picked) => picked,
                                            Err(_) => {
                                                eprintln!("无法读取 SGF 文件");
                                                sgf_busy.set(false);
                                                return;
                                            }
                                        };
                                        if picked.cancelled.unwrap_or(false) {
                                            sgf_busy.set(false);
                                            return;
                                        }
                                        if let Some(error) = picked.error {
                                            eprintln!("SGF 选择失败: {error}");
                                            sgf_busy.set(false);
                                            return;
                                        }
                                        let Some(text) = picked.text else {
                                            eprintln!("没有收到 SGF 内容");
                                            sgf_busy.set(false);
                                            return;
                                        };
                                        let imported = session.write().import_sgf(&text);
                                        match imported {
                                            Ok(()) => {
                                                let total = session.read().game().all_moves().len();
                                                analysis.write().start_new_game(total);
                                                rule_open.set(false);
                                                size_open.set(false);
                                                reset_rotation.set(0);
                                            }
                                            Err(error) => eprintln!("SGF 导入失败: {error}"),
                                        }
                                        sgf_busy.set(false);
                                    });
                                }
                            },
                            if sgf_busy() {
                                "处理中"
                            } else if locked {
                                "保存"
                            } else {
                                "导入"
                            }
                        }
                    }
                }
            }
            }
        }
    }
}
