use dioxus::prelude::*;
use lazygo::game::{star_points, Game, Stone};
use lazygo::session::PendingMove;

const STAR_RADIUS: f64 = 50.0 / 304.0;
const HIT_RADIUS: f64 = 0.36;
const STONE_OUTER_RADIUS: f64 = 0.46;
const STONE_INNER_RADIUS: f64 = 0.40;
const PENDING_FRAME_RADIUS: f64 = 0.54;
const LAST_MOVE_RING_RADIUS: f64 = 0.34;
const LAST_MOVE_RING_WIDTH: f64 = 0.06;

fn stone_classes(stone: Stone) -> (&'static str, &'static str) {
    match stone {
        Stone::Black => ("stone-black-outer", "stone-black-inner"),
        Stone::White => ("stone-white-outer", "stone-white-inner"),
    }
}

fn stone_node(x: usize, y: usize, stone: Stone) -> Element {
    let (outer, inner) = stone_classes(stone);

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

    let x = (client_x - left) / width * max as f64;
    let y = (client_y - top) / height * max as f64;
    let nearest_x = x.round();
    let nearest_y = y.round();

    if !(0.0..=max as f64).contains(&nearest_x) || !(0.0..=max as f64).contains(&nearest_y) {
        return None;
    }
    if (x - nearest_x).powi(2) + (y - nearest_y).powi(2) > HIT_RADIUS.powi(2) {
        return None;
    }

    Some((nearest_x as usize, nearest_y as usize))
}

fn handle_board_click(
    event: Event<MouseData>,
    max: usize,
    on_select: EventHandler<(usize, usize)>,
    on_cancel: EventHandler<()>,
) {
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
        if let Some(point) = board_point(client.x, client.y, left, top, width, height, max) {
            on_select.call(point);
        } else {
            on_cancel.call(());
        }
    });
}

#[component]
pub fn Board(
    game: Game,
    pending: Option<PendingMove>,
    ownership: Option<Vec<f32>>,
    on_select: EventHandler<(usize, usize)>,
    on_cancel: EventHandler<()>,
) -> Element {
    let size = game.size();
    let max = size - 1;
    let stars = star_points(size);
    let last_move = game.last_move();
    let ownership_dots: Vec<(f64, f64, bool, bool, f32)> = ownership
        .as_ref()
        .map(|ownership| {
            let mut dots = Vec::new();
            for y in 0..=max {
                for x in 0..=max {
                    let value = ownership.get(y * size + x).copied().unwrap_or(0.0);
                    if value.abs() < 0.05 {
                        continue;
                    }
                    let opacity = 0.20 + 0.60 * value.abs().min(1.0);
                    let on_stone = game.stone_at(x, y).is_some();
                    dots.push((
                        x as f64 - 0.15,
                        y as f64 - 0.15,
                        value > 0.0,
                        on_stone,
                        opacity,
                    ));
                }
            }
            dots
        })
        .unwrap_or_default();

    rsx! {
        div { class: "board-frame",
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
                        {stone_node(x, y, stone)}
                    }
                }
            }
            for (dot_x, dot_y, black, on_stone, opacity) in ownership_dots {
                rect {
                    class: if black {
                        if on_stone { "ownership-dot black on-stone" } else { "ownership-dot black" }
                    } else if on_stone {
                        "ownership-dot white on-stone"
                    } else {
                        "ownership-dot white"
                    },
                    x: "{dot_x}",
                    y: "{dot_y}",
                    width: "0.30",
                    height: "0.30",
                    rx: "0.08",
                    style: "opacity: {opacity:.3};",
                }
            }
            if let Some((x, y)) = last_move {
                if let Some(stone) = game.stone_at(x, y) {
                    circle {
                        class: if stone == Stone::Black {
                            "last-move-ring black"
                        } else {
                            "last-move-ring white"
                        },
                        cx: "{x}",
                        cy: "{y}",
                        r: "{LAST_MOVE_RING_RADIUS}",
                        stroke_width: "{LAST_MOVE_RING_WIDTH}",
                    }
                }
            }
            if let Some(pending) = pending {
                {stone_node(pending.x, pending.y, game.turn())}
                circle {
                    class: if pending.legal { "pending-frame" } else { "pending-frame illegal" },
                    cx: "{pending.x}",
                    cy: "{pending.y}",
                    r: "{PENDING_FRAME_RADIUS}",
                }
            }
            }
            div {
                class: "board-hit-layer",
                onclick: move |event| handle_board_click(event, max, on_select, on_cancel),
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
    fn hit_test_ignores_gaps_and_far_outside_taps() {
        assert_eq!(board_point(450.0, 400.0, 0.0, 0.0, 800.0, 800.0, 8), None);
        assert_eq!(board_point(-100.0, 400.0, 0.0, 0.0, 800.0, 800.0, 8), None);
    }

    #[test]
    fn hit_test_uses_full_circles_at_edges_and_corners() {
        assert_eq!(
            board_point(-1.0, 400.0, 0.0, 0.0, 800.0, 800.0, 8),
            Some((0, 4))
        );
        assert_eq!(
            board_point(-1.0, -1.0, 0.0, 0.0, 800.0, 800.0, 8),
            Some((0, 0))
        );
        assert_eq!(
            board_point(801.0, 801.0, 0.0, 0.0, 800.0, 800.0, 8),
            Some((8, 8))
        );
    }
}
