use dioxus::prelude::*;
use lazygo::game::{star_points, Game, Stone};
use lazygo::session::PendingMove;

const STAR_RADIUS: f64 = 50.0 / 304.0;
const HIT_RADIUS: f64 = 0.36;
const STONE_OUTER_RADIUS: f64 = 0.46;
const STONE_INNER_RADIUS: f64 = 0.40;
const MOVE_DOT_RADIUS: f64 = 0.12;
const PENDING_FRAME_RADIUS: f64 = 0.54;

fn stone_classes(stone: Stone) -> (&'static str, &'static str, &'static str) {
    match stone {
        Stone::Black => ("stone-black-outer", "stone-black-inner", "move-dot-black"),
        Stone::White => ("stone-white-outer", "stone-white-inner", "move-dot-white"),
    }
}

fn stone_node(x: usize, y: usize, stone: Stone, last_move: bool) -> Element {
    let (outer, inner, dot) = stone_classes(stone);

    rsx! {
        circle {
            class: "{outer}",
            cx: "{x}",
            cy: "{y}",
            r: "{STONE_OUTER_RADIUS}",
        }
        circle {
            class: "{inner}",
            cx: "{x}",
            cy: "{y}",
            r: "{STONE_INNER_RADIUS}",
        }
        if last_move {
            circle {
                class: "{dot}",
                cx: "{x}",
                cy: "{y}",
                r: "{MOVE_DOT_RADIUS}",
            }
        }
    }
}

#[component]
pub fn Board(
    game: Game,
    pending: Option<PendingMove>,
    on_select: EventHandler<(usize, usize)>,
) -> Element {
    let size = game.size();
    let max = size - 1;
    let stars = star_points(size);
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
            for &(x, y) in stars {
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
                        {stone_node(x, y, stone, last_move == Some((x, y)))}
                    }
                    if let Some(pending) = pending {
                        if pending.x == x && pending.y == y {
                            {stone_node(x, y, game.turn(), false)}
                            circle {
                                class: if pending.legal { "pending-frame" } else { "pending-frame illegal" },
                                cx: "{x}",
                                cy: "{y}",
                                r: "{PENDING_FRAME_RADIUS}",
                            }
                        }
                    }
                    circle {
                        class: "board-hit",
                        fill: "none",
                        cx: "{x}",
                        cy: "{y}",
                        r: "{HIT_RADIUS}",
                        onclick: move |_| on_select.call((x, y)),
                    }
                }
            }
        }
    }
}
