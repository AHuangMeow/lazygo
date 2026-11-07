use dioxus::prelude::*;

const MAIN_CSS: Asset = asset!("/assets/main.css");
const APP_ICON: Asset = asset!("/assets/icon.png");
const APP_FONT: Asset = asset!("/assets/fonts/maiyuan-ii-regular.ttf");

fn star_points(size: usize) -> Vec<(usize, usize)> {
    match size {
        9 => vec![(2, 2), (2, 6), (4, 4), (6, 2), (6, 6)],
        13 => vec![(3, 3), (3, 9), (6, 6), (9, 3), (9, 9)],
        19 => vec![
            (3, 3),
            (3, 9),
            (3, 15),
            (9, 3),
            (9, 9),
            (9, 15),
            (15, 3),
            (15, 9),
            (15, 15),
        ],
        _ => Vec::new(),
    }
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut board_size = use_signal(|| 9usize);
    let mut menu_open = use_signal(|| false);
    let size = board_size();
    let max = size - 1;
    let star_radius = 40.0 / 304.0;
    let stars = star_points(size);
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
            section { class: "content",
                div { class: "board-column",
                    svg {
                        class: "board",
                        view_box: "-0.5 -0.5 {size} {size}",
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
                    }
                    div { class: "board-footer",
                        div { class: "size-menu",
                            button {
                                class: if menu_open() { "size-trigger open" } else { "size-trigger" },
                                r#type: "button",
                                aria_label: "Board size",
                                aria_expanded: if menu_open() { "true" } else { "false" },
                                onclick: move |_| menu_open.set(!menu_open()),
                                span { "{size}x{size}" }
                                span { class: "chevron" }
                            }
                            if menu_open() {
                                div {
                                    class: "size-backdrop",
                                    onclick: move |_| menu_open.set(false),
                                }
                                div { class: "size-options",
                                    for option in [9usize, 13usize, 19usize] {
                                        button {
                                            class: if size == option { "size-option selected" } else { "size-option" },
                                            r#type: "button",
                                            onclick: move |_| {
                                                board_size.set(option);
                                                menu_open.set(false);
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
