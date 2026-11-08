use dioxus::prelude::*;
use lazygo::analysis::engine::AnalysisSnapshot;
use lazygo::game::{Move, Stone};

use super::{AnalysisMode, ModeToggle, PerspectiveToggle};

const WIDTH: f64 = 320.0;
const HEIGHT: f64 = 168.0;
const LEFT: f64 = 20.0;
const RIGHT: f64 = 6.0;
const TOP: f64 = 14.0;
const BOTTOM: f64 = 14.0;
const PLOT_WIDTH: f64 = WIDTH - LEFT - RIGHT;
const PLOT_HEIGHT: f64 = HEIGHT - TOP - BOTTOM;

fn point_value(snapshot: &AnalysisSnapshot, perspective: Stone, mode: AnalysisMode) -> f64 {
    match mode {
        AnalysisMode::Winrate => {
            let value = if perspective == Stone::Black {
                snapshot.black_winrate as f64 * 100.0
            } else {
                (1.0 - snapshot.black_winrate as f64) * 100.0
            };
            value.clamp(0.0, 100.0)
        }
        AnalysisMode::Score => {
            let value = if perspective == Stone::Black {
                snapshot.black_score_lead as f64
            } else {
                -(snapshot.black_score_lead as f64)
            };
            value.clamp(-10.0, 10.0)
        }
    }
}

fn axis_bounds(mode: AnalysisMode) -> (f64, f64) {
    match mode {
        AnalysisMode::Winrate => (0.0, 100.0),
        AnalysisMode::Score => (-10.0, 10.0),
    }
}

fn is_pass(moves: &[Move], index: usize) -> bool {
    index > 0 && matches!(moves.get(index - 1), Some(Move::Pass))
}

fn chart_x(index: usize, total_moves: usize) -> f64 {
    LEFT + index as f64 * PLOT_WIDTH / (total_moves.max(10) + 1) as f64
}

#[component]
fn AnalysisChart(
    snapshots: Vec<Option<AnalysisSnapshot>>,
    perspective: Stone,
    mode: AnalysisMode,
    current_index: usize,
    moves: Vec<Move>,
    on_replay: EventHandler<usize>,
) -> Element {
    let (min, max) = axis_bounds(mode);
    let y_for = |value: f64| TOP + (max - value) / (max - min) * PLOT_HEIGHT;
    let zero_y = y_for(0.0);
    let total_moves = moves.len();
    let x_for = |index: usize| chart_x(index, total_moves);
    let labels: &[f64] = match mode {
        AnalysisMode::Winrate => &[100.0, 75.0, 50.0, 25.0, 0.0],
        AnalysisMode::Score => &[10.0, 5.0, 0.0, -5.0, -10.0],
    };
    let current = (current_index > 0)
        .then(|| {
            snapshots
                .get(current_index)
                .and_then(|snapshot| snapshot.as_ref())
        })
        .flatten();

    rsx! {
        svg {
            class: "analysis-chart-svg",
            view_box: "0 0 {WIDTH} {HEIGHT}",
            role: "img",
            line {
                class: "chart-axis",
                x1: "{LEFT}",
                y1: "{TOP}",
                x2: "{LEFT}",
                y2: "{TOP + PLOT_HEIGHT}",
            }
            line {
                class: "chart-axis",
                x1: "{LEFT}",
                y1: "{zero_y}",
                x2: "{LEFT + PLOT_WIDTH}",
                y2: "{zero_y}",
            }
            for value in labels {
                text {
                    class: "chart-label",
                    x: "{LEFT - 4.0}",
                    y: "{y_for(*value)}",
                    text_anchor: "end",
                    dominant_baseline: "middle",
                    "{value:.0}"
                }
            }
            for (index, snapshot) in snapshots.iter().enumerate() {
                if index > 0 {
                    if let Some(snapshot) = snapshot {
                        {
                            let value = point_value(snapshot, perspective, mode);
                            let x = x_for(index);
                            let y = y_for(value);
                            let pass = is_pass(&moves, index);
                            rsx! {
                                g {
                                    class: "chart-point-group",
                                    circle {
                                        class: "chart-hit",
                                        cx: "{x}",
                                        cy: "{y}",
                                        r: "9",
                                        onclick: move |_| on_replay.call(index),
                                    }
                                    if pass {
                                        text {
                                            class: "chart-pass",
                                            x: "{x}",
                                            y: "{y}",
                                            text_anchor: "middle",
                                            dominant_baseline: "middle",
                                            onclick: move |_| on_replay.call(index),
                                            "p"
                                        }
                                    } else {
                                        circle {
                                            class: "chart-point",
                                            cx: "{x}",
                                            cy: "{y}",
                                            r: "3.2",
                                            onclick: move |_| on_replay.call(index),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(snapshot) = current {
                {
                    let value = point_value(snapshot, perspective, mode);
                    let x = x_for(current_index);
                    let y = y_for(value);
                    let label_x = x.clamp(LEFT + 14.0, LEFT + PLOT_WIDTH - 14.0);
                    let label_y = (y - 11.0).max(TOP + 9.0);
                    rsx! {
                        circle {
                            class: "chart-current",
                            cx: "{x}",
                            cy: "{y}",
                            r: "6.6",
                        }
                        text {
                            class: "chart-value",
                            x: "{label_x}",
                            y: "{label_y}",
                            text_anchor: "middle",
                            "{value:.1}"
                        }
                    }
                }
            } else if snapshots.iter().all(Option::is_none) {
                text {
                    class: "chart-empty",
                    x: "{LEFT + PLOT_WIDTH / 2.0}",
                    y: "{TOP + PLOT_HEIGHT / 2.0}",
                    text_anchor: "middle",
                    "暂无分析"
                }
            }
        }
    }
}

#[component]
pub fn AnalysisPanel(
    open: bool,
    enabled: bool,
    perspective: Stone,
    mode: AnalysisMode,
    error: Option<String>,
    chart: Vec<Option<AnalysisSnapshot>>,
    current_index: usize,
    moves: Vec<Move>,
    on_toggle: EventHandler<()>,
    on_perspective: EventHandler<()>,
    on_mode: EventHandler<()>,
    on_replay: EventHandler<usize>,
) -> Element {
    rsx! {
        div { class: "analysis-area",
            div { class: "analysis-toolbar",
                button {
                    class: if open { "analysis-trigger open" } else { "analysis-trigger" },
                    r#type: "button",
                    disabled: !enabled,
                    aria_expanded: open,
                    onclick: move |_| on_toggle.call(()),
                    span { "分析" }
                    span { class: "chevron" }
                }
                div {
                    class: if open { "analysis-controls open" } else { "analysis-controls" },
                    PerspectiveToggle {
                        perspective,
                        on_click: move |_| on_perspective.call(()),
                    }
                    ModeToggle {
                        mode,
                        perspective,
                        on_click: move |_| on_mode.call(()),
                    }
                }
            }
            section { class: if open { "analysis-panel open" } else { "analysis-panel" },
                    AnalysisChart {
                        snapshots: chart,
                        perspective,
                        mode,
                        current_index,
                        moves,
                        on_replay,
                    }
                    if let Some(error) = error {
                        span { class: "analysis-error", "{error}" }
                    }
                }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{chart_x, is_pass, LEFT, PLOT_WIDTH};
    use lazygo::game::Move;

    #[test]
    fn chart_x_uses_one_slot_larger_than_max_ten() {
        assert_eq!(chart_x(0, 0), LEFT);
        assert_eq!(chart_x(9, 9), LEFT + PLOT_WIDTH * 9.0 / 11.0);
        assert_eq!(chart_x(19, 19), LEFT + PLOT_WIDTH * 19.0 / 20.0);
        assert_eq!(chart_x(5, 20), LEFT + PLOT_WIDTH * 5.0 / 21.0);
    }

    #[test]
    fn pass_marker_only_applies_to_pass_moves() {
        let moves = vec![Move::Play(4, 4), Move::Pass];
        assert!(!is_pass(&moves, 0));
        assert!(!is_pass(&moves, 1));
        assert!(is_pass(&moves, 2));
    }
}
