use dioxus::prelude::*;
use lazygo::game::{star_points, Game, Stone, BOARD_SIZES, DEFAULT_SIZE};

const MAIN_CSS: Asset = asset!("/assets/main.css");
const APP_ICON: Asset = asset!("/assets/icon.png");
const APP_FONT: Asset = asset!("/assets/fonts/maiyuan-ii-regular.ttf");

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

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut game = use_signal(|| Game::new(DEFAULT_SIZE));
    let mut menu_open = use_signal(|| false);
    let mut reset_armed = use_signal(|| false);
    let mut reset_rotation = use_signal(|| 0i32);
    let current = game();
    let size = current.size();
    let max = size - 1;
    let board_margin = format!("--board-margin: {:.6}vw;", 100.0 / (size as f64 + 2.0));
    let initial = current.is_initial();
    let star_radius = 40.0 / 304.0;
    let hit_radius = 0.36;
    let stars = star_points(size);
    let (back, front) = match current.turn() {
        Stone::Black => ((43.0, Stone::White), (21.0, Stone::Black)),
        Stone::White => ((21.0, Stone::Black), (43.0, Stone::White)),
    };
    let font_face = format!(
        r#"@font-face {{ font-family: "Maiyuan II"; src: url("{}") format("truetype"); font-style: normal; font-weight: 400; font-display: swap; }}"#,
        APP_FONT
    );

    rsx! {
        style { "{font_face}" }
        document::Stylesheet { href: MAIN_CSS }
        main { class: "page",
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
                    svg {
                        class: "board",
                        view_box: "0 0 {max} {max}",
                        for index in 0..=max {
                            line {
                                class: "board-line",
                                x1: "0",
                                y1: "{index}",
                                x2: "{max}",
                                y2: "{index}",
                            }
                            line {
                                class: "board-line",
                                x1: "{index}",
                                y1: "0",
                                x2: "{index}",
                                y2: "{max}",
                            }
                        }
                        for (x, y) in stars {
                            circle {
                                class: "board-star",
                                cx: "{x}",
                                cy: "{y}",
                                r: "{star_radius}",
                            }
                        }
                        for y in 0..=max {
                            for x in 0..=max {
                                if let Some(stone) = current.stone_at(x, y) {
                                    circle {
                                        class: if stone == Stone::Black { "stone-black-outer" } else { "stone-white-outer" },
                                        cx: "{x}",
                                        cy: "{y}",
                                        r: "0.46",
                                    }
                                    circle {
                                        class: if stone == Stone::Black { "stone-black-inner" } else { "stone-white-inner" },
                                        cx: "{x}",
                                        cy: "{y}",
                                        r: "0.40",
                                    }
                                    if current.last_move() == Some((x, y)) {
                                        circle {
                                            class: if stone == Stone::Black { "move-dot-black" } else { "move-dot-white" },
                                            cx: "{x}",
                                            cy: "{y}",
                                            r: "0.12",
                                        }
                                    }
                                }
                                circle {
                                    class: "board-hit",
                                    cx: "{x}",
                                    cy: "{y}",
                                    r: "{hit_radius}",
                                    onclick: move |_| {
                                        let position = game();

                                        if let Some(next) = position.play(x, y) {
                                            game.set(next);
                                            reset_armed.set(false);
                                            menu_open.set(false);
                                        }
                                    },
                                }
                            }
                        }
                    }
                    div { class: "board-footer",
                        div { class: "board-actions",
                            button {
                                class: "turn-indicator",
                                r#type: "button",
                                aria_label: "Pass",
                                onclick: move |_| {
                                    game.set(game().pass());
                                    reset_armed.set(false);
                                    menu_open.set(false);
                                },
                                svg {
                                    class: "turn-stones",
                                    view_box: "6 0 52 40",
                                    {indicator_stone(back.0, back.1)}
                                    {indicator_stone(front.0, front.1)}
                                }
                            }
                            button {
                                class: if reset_armed() { "reset-button armed" } else { "reset-button" },
                                r#type: "button",
                                aria_label: "Reset",
                                style: "transform: rotate({reset_rotation}deg);",
                                onclick: move |_| {
                                    if reset_armed() {
                                        game.set(Game::new(size));
                                        reset_armed.set(false);
                                        menu_open.set(false);
                                        reset_rotation.set(reset_rotation() + 360);
                                    } else {
                                        reset_armed.set(true);
                                        menu_open.set(false);
                                        reset_rotation.set(reset_rotation() + 360);
                                    }
                                },
                                svg {
                                    class: "reset-icon",
                                    view_box: "0 0 24 24",
                                    path {
                                        d: "M17.65 6.35C16.2 4.9 14.21 4 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08c-.82 2.33-3.04 4-5.65 4-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z"
                                    }
                                }
                            }
                        }
                        div { class: "size-menu",
                            button {
                                class: if menu_open() && initial { "size-trigger open" } else { "size-trigger" },
                                r#type: "button",
                                aria_label: "Board size",
                                aria_expanded: if menu_open() && initial { "true" } else { "false" },
                                disabled: !initial,
                                onclick: move |_| {
                                    if initial {
                                        menu_open.set(!menu_open());
                                    }
                                },
                                span { "{size}x{size}" }
                                span { class: "chevron" }
                            }
                            if menu_open() && initial {
                                div {
                                    class: "size-backdrop",
                                    onclick: move |_| menu_open.set(false),
                                }
                                div { class: "size-options",
                                    for option in BOARD_SIZES {
                                        button {
                                            class: if size == option { "size-option selected" } else { "size-option" },
                                            r#type: "button",
                                            onclick: move |_| {
                                                game.set(Game::new(option));
                                                menu_open.set(false);
                                                reset_armed.set(false);
                                            },
                                            "{option}x{option}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
