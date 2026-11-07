use dioxus::prelude::*;
use lazygo::game::{star_points, Game, Stone};

const STAR_RADIUS: f64 = 40.0 / 304.0;
const HIT_RADIUS: f64 = 0.36;

#[component]
pub fn Board(game: Game, on_play: EventHandler<(usize, usize)>) -> Element {
    let max = game.size() - 1;
    let stars = star_points(game.size());
    let last_move = game.last_move();

    rsx! {
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
                    r: "{STAR_RADIUS}",
                }
            }
            for y in 0..=max {
                for x in 0..=max {
                    if let Some(stone) = game.stone_at(x, y) {
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
                        if last_move == Some((x, y)) {
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
                        r: "{HIT_RADIUS}",
                        onclick: move |_| on_play.call((x, y)),
                    }
                }
            }
        }
    }
}
