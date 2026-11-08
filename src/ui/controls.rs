use dioxus::prelude::*;
use lazygo::game::Stone;

use crate::APP_ICON;

fn indicator_stone(x: f64, stone: Stone, visible: bool) -> Element {
    indicator_stone_dim(x, stone, visible, false)
}

fn indicator_stone_dim(x: f64, stone: Stone, visible: bool, dimmed: bool) -> Element {
    let (group, outer, inner) = match stone {
        Stone::Black => (
            "indicator-black",
            "indicator-black-outer",
            "indicator-black-inner",
        ),
        Stone::White => (
            "indicator-white",
            "indicator-white-outer",
            "indicator-white-inner",
        ),
    };

    let class = if !visible {
        format!("{group} hidden")
    } else if dimmed {
        format!("{group} dimmed")
    } else {
        group.to_string()
    };

    rsx! {
        g { class: "{class}",
            circle {
                class: "{outer}",
                cx: "{x}",
                cy: "20",
                r: "15",
            }
            circle {
                class: "{inner}",
                cx: "{x}",
                cy: "20",
                r: "13",
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum HistoryDirection {
    Undo,
    Redo,
}

const HOLD_SCRIPT: &str = r#"
    let holdTimer = null;
    let repeatTimer = null;
    const stop = () => {
        clearTimeout(holdTimer);
        clearInterval(repeatTimer);
        holdTimer = null;
        repeatTimer = null;
    };
    while (true) {
        const command = await dioxus.recv();
        if (command === "start") {
            stop();
            holdTimer = setTimeout(() => {
                dioxus.send("tick");
                repeatTimer = setInterval(() => dioxus.send("tick"), 70);
            }, 350);
        } else if (command === "stop") {
            stop();
        }
    }
"#;

#[component]
pub fn HistoryButton(
    direction: HistoryDirection,
    enabled: bool,
    on_click: EventHandler<()>,
) -> Element {
    let (label, path) = match direction {
        HistoryDirection::Undo => ("Undo", "M15 6l-9 6 9 6z"),
        HistoryDirection::Redo => ("Redo", "M9 6l9 6-9 6z"),
    };
    let mut pressing = use_signal(|| false);
    let mut holding = use_signal(|| false);
    let mut long_pressed = use_signal(|| false);
    let eval = use_hook(move || {
        let mut eval = document::eval(HOLD_SCRIPT);

        spawn(async move {
            while let Ok(message) = eval.recv::<String>().await {
                if message == "tick" && pressing() {
                    holding.set(true);
                    long_pressed.set(true);
                    on_click.call(());
                }
            }
        });

        eval
    });

    rsx! {
        button {
            class: if holding() { "history-button holding" } else { "history-button" },
            r#type: "button",
            aria_label: "{label}",
            disabled: !enabled && !pressing(),
            onpointerdown: move |_| {
                if !enabled {
                    return;
                }

                pressing.set(true);
                holding.set(false);
                long_pressed.set(false);
                let _ = eval.send("start");
            },
            onpointerup: move |_| {
                pressing.set(false);
                holding.set(false);
                let _ = eval.send("stop");
            },
            onpointercancel: move |_| {
                pressing.set(false);
                holding.set(false);
                let _ = eval.send("stop");
            },
            onpointerleave: move |_| {
                if pressing() {
                    pressing.set(false);
                    holding.set(false);
                    let _ = eval.send("stop");
                }
            },
            oncontextmenu: move |event| event.prevent_default(),
            onclick: move |_| {
                if long_pressed() {
                    long_pressed.set(false);
                } else {
                    on_click.call(());
                }
            },
            svg {
                class: "history-icon",
                view_box: "0 0 24 24",
                path { d: "{path}" }
            }
        }
    }
}

#[component]
pub fn Topbar(on_settings: EventHandler<()>) -> Element {
    rsx! {
        header { class: "topbar",
            div { class: "brand",
                img {
                    class: "brand-icon",
                    src: APP_ICON,
                    alt: "LazyGo",
                }
                span { class: "brand-name", "LazyGo" }
            }
            button {
                class: "menu-button",
                r#type: "button",
                aria_label: "Settings",
                onclick: move |_| on_settings.call(()),
                svg {
                    class: "menu-icon",
                    view_box: "0 0 24 24",
                    path {
                        d: "M19.14 12.94c.04-.32.07-.64.07-.94s-.03-.62-.07-.94l2.11-1.65a.5.5 0 0 0 .12-.64l-2-3.46a.5.5 0 0 0-.61-.22l-2.49 1a7.3 7.3 0 0 0-1.68-.97l-.38-2.65A.5.5 0 0 0 14 2h-4a.5.5 0 0 0-.49.42l-.38 2.65c-.6.24-1.16.57-1.68.97l-2.49-1a.5.5 0 0 0-.61.22l-2 3.46a.5.5 0 0 0 .12.64l2.11 1.65c-.04.32-.07.64-.07.94s.03.62.07.94l-2.11 1.65a.5.5 0 0 0-.12.64l2 3.46c.13.22.39.31.61.22l2.49-1c.52.4 1.08.73 1.68.97l.38 2.65c.04.24.25.42.49.42h4c.24 0 .45-.18.49-.42l.38-2.65c.6-.24 1.16-.57 1.68-.97l2.49 1c.22.09.48 0 .61-.22l2-3.46a.5.5 0 0 0-.12-.64l-2.11-1.65ZM12 15.5a3.5 3.5 0 1 1 0-7 3.5 3.5 0 0 1 0 7Z"
                    }
                }
            }
        }
    }
}

#[component]
pub fn TurnIndicator(turn: Stone, open: bool, on_click: EventHandler<()>) -> Element {
    rsx! {
        button {
            class: if open { "turn-indicator open" } else { "turn-indicator" },
            r#type: "button",
            aria_label: "Pass",
            onclick: move |_| on_click.call(()),
            svg {
                class: "turn-stones",
                view_box: "6 0 52 40",
                {indicator_stone(21.0, Stone::Black, true)}
                {indicator_stone(43.0, Stone::White, true)}
                {indicator_stone(21.0, Stone::Black, turn == Stone::Black)}
            }
        }
    }
}

#[component]
pub fn ResetButton(armed: bool, rotation: i32, on_click: EventHandler<()>) -> Element {
    rsx! {
        button {
            class: if armed { "reset-button armed" } else { "reset-button" },
            r#type: "button",
            aria_label: "Reset",
            style: "transform: rotate({rotation}deg);",
            onclick: move |_| on_click.call(()),
            svg {
                class: "reset-icon",
                view_box: "0 0 24 24",
                path {
                    d: "M17.65 6.35C16.2 4.9 14.21 4 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08c-.82 2.33-3.04 4-5.65 4-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z"
                }
            }
        }
    }
}

#[component]
pub fn ConfirmButton(enabled: bool, on_click: EventHandler<()>) -> Element {
    rsx! {
        button {
            class: "confirm-button",
            r#type: "button",
            disabled: !enabled,
            onclick: move |_| on_click.call(()),
            "确认"
        }
    }
}

#[component]
pub fn PerspectiveToggle(perspective: Stone, on_click: EventHandler<()>) -> Element {
    let (outer, inner) = match perspective {
        Stone::Black => ("indicator-black-outer", "indicator-black-inner"),
        Stone::White => ("indicator-white-outer", "indicator-white-inner"),
    };

    rsx! {
        button {
            class: "perspective-toggle",
            r#type: "button",
            aria_label: if perspective == Stone::Black {
                "切换到白方视角"
            } else {
                "切换到黑方视角"
            },
            onclick: move |_| on_click.call(()),
            svg {
                class: "perspective-stone",
                view_box: "0 0 40 40",
                circle {
                    class: "{outer}",
                    cx: "20",
                    cy: "20",
                    r: "16",
                }
                circle {
                    class: "{inner}",
                    cx: "20",
                    cy: "20",
                    r: "14",
                }
            }
        }
    }
}

#[component]
pub fn ModeToggle(
    mode: crate::ui::AnalysisMode,
    perspective: Stone,
    on_click: EventHandler<()>,
) -> Element {
    let winrate = mode == crate::ui::AnalysisMode::Winrate;

    rsx! {
        button {
            class: "mode-toggle",
            r#type: "button",
            aria_label: if winrate { "切换到目差" } else { "切换到胜率" },
            onclick: move |_| on_click.call(()),
            if winrate {
                span { class: "mode-percent", "%" }
            } else {
                span {
                    class: if perspective == Stone::Black {
                        "mode-score-glyph black"
                    } else {
                        "mode-score-glyph white"
                    },
                }
            }
        }
    }
}
