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

fn board_point(
    client_x: f64,
    client_y: f64,
    left: f64,
    top: f64,
    width: f64,
    height: f64,
    max: usize,
) -> Option<(usize, usize)> {
    if width <= 0.0 || height <= 0.0 || max == 0 {
        return None;
    }

    if client_x < left || client_x > left + width || client_y < top || client_y > top + height {
        return None;
    }

    let x = (client_x - left) / width * max as f64;
    let y = (client_y - top) / height * max as f64;
    let nearest_x = x.round();
    let nearest_y = y.round();

    if (x - nearest_x).powi(2) + (y - nearest_y).powi(2) > HIT_RADIUS.powi(2) {
        return None;
    }

    Some((nearest_x as usize, nearest_y as usize))
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
            onclick: move |event| {
                let client = event.data().client_coordinates();
                spawn(async move {
                    let mut eval = document::eval(
                        r#"
                        const rect = document.querySelector(".board").getBoundingClientRect();
                        dioxus.send(`${rect.left},${rect.top},${rect.width},${rect.height}`);
                        "#,
                    );
                    let Ok(value) = eval.recv::<String>().await else {
                        return;
                    };
                    let mut parts = value.split(',').filter_map(|part| part.parse::<f64>().ok());
                    let (Some(left), Some(top), Some(width), Some(height)) =
                        (parts.next(), parts.next(), parts.next(), parts.next())
                    else {
                        return;
                    };
                    if let Some(point) =
                        board_point(client.x, client.y, left, top, width, height, max)
                    {
                        on_select.call(point);
                    }
                });
            },
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
                }
            }
            if let Some(pending) = pending {
                {stone_node(pending.x, pending.y, game.turn(), false)}
                circle {
                    class: if pending.legal { "pending-frame" } else { "pending-frame illegal" },
                    cx: "{pending.x}",
                    cy: "{pending.y}",
                    r: "{PENDING_FRAME_RADIUS}",
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::board_point;

    #[test]
    fn hit_test_accepts_points_inside_the_radius() {
        assert_eq!(
            board_point(430.0, 400.0, 0.0, 0.0, 800.0, 800.0, 8),
            Some((4, 4))
        );
    }

    #[test]
    fn hit_test_ignores_gaps_and_outside_taps() {
        assert_eq!(board_point(450.0, 400.0, 0.0, 0.0, 800.0, 800.0, 8), None);
        assert_eq!(board_point(-1.0, 400.0, 0.0, 0.0, 800.0, 800.0, 8), None);
    }
}
