use dioxus::prelude::*;
use lazygo::game::Stone;

use crate::APP_ICON;

fn indicator_stone(x: f64, stone: Stone) -> Element {
    let (outer, inner) = match stone {
        Stone::Black => ("indicator-black-outer", "indicator-black-inner"),
        Stone::White => ("indicator-white-outer", "indicator-white-inner"),
    };

    rsx! {
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

#[derive(Clone, Copy, PartialEq)]
pub enum HistoryDirection {
    Undo,
    Redo,
}

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

    rsx! {
        button {
            class: "history-button",
            r#type: "button",
            aria_label: "{label}",
            disabled: !enabled,
            onclick: move |_| on_click.call(()),
            svg {
                class: "history-icon",
                view_box: "0 0 24 24",
                path { d: "{path}" }
            }
        }
    }
}

#[component]
pub fn Topbar() -> Element {
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
                aria_label: "More",
                svg {
                    class: "menu-icon",
                    view_box: "0 0 24 24",
                    path {
                        d: "M4 18h16q.425 0 .713-.288Q21 17.425 21 17t-.287-.712Q20.425 16 20 16H4q-.425 0-.712.288Q3 16.575 3 17t.288.712Q3.575 18 4 18Zm0-5h16q.425 0 .713-.288Q21 12.425 21 12t-.287-.712Q20.425 11 20 11H4q-.425 0-.712.288Q3 11.575 3 12t.288.712Q3.575 13 4 13Zm0-5h16q.425 0 .713-.288Q21 7.425 21 7t-.287-.712Q20.425 6 20 6H4q-.425 0-.712.288Q3 6.575 3 7t.288.712Q3.575 8 4 8Z"
                    }
                }
            }
        }
    }
}

#[component]
pub fn TurnIndicator(turn: Stone, on_pass: EventHandler<()>) -> Element {
    let (back, front) = match turn {
        Stone::Black => ((43.0, Stone::White), (21.0, Stone::Black)),
        Stone::White => ((21.0, Stone::Black), (43.0, Stone::White)),
    };

    rsx! {
        button {
            class: "turn-indicator",
            r#type: "button",
            aria_label: "Pass",
            onclick: move |_| on_pass.call(()),
            svg {
                class: "turn-stones",
                view_box: "6 0 52 40",
                {indicator_stone(back.0, back.1)}
                {indicator_stone(front.0, front.1)}
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
