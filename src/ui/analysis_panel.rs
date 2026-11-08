use dioxus::prelude::*;
use lazygo::analysis::engine::AnalysisSnapshot;
use lazygo::game::Stone;

use super::{AnalysisMode, ModeToggle, PerspectiveToggle};

const WIDTH: f64 = 320.0;
const HEIGHT: f64 = 168.0;
const LEFT: f64 = 31.0;
const RIGHT: f64 = 6.0;
const TOP: f64 = 12.0;
const BOTTOM: f64 = 12.0;
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

#[component]
fn AnalysisChart(
    snapshots: Vec<Option<AnalysisSnapshot>>,
    perspective: Stone,
    mode: AnalysisMode,
    current_index: usize,
) -> Element {
    let (min, max) = axis_bounds(mode);
    let y_for = |value: f64| TOP + (max - value) / (max - min) * PLOT_HEIGHT;
    let zero_y = y_for(0.0);
    let count = snapshots.len();
    let x_for = |index: usize| {
        if count <= 1 {
            LEFT + PLOT_WIDTH / 2.0
        } else {
            LEFT + index as f64 / (count - 1) as f64 * PLOT_WIDTH
        }
    };
    let labels: &[f64] = match mode {
        AnalysisMode::Winrate => &[100.0, 75.0, 50.0, 25.0, 0.0],
        AnalysisMode::Score => &[10.0, 5.0, 0.0, -5.0, -10.0],
    };
    let current = snapshots
        .get(current_index)
        .and_then(|snapshot| snapshot.as_ref());

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
                    x: "{LEFT - 5.0}",
                    y: "{y_for(*value)}",
                    text_anchor: "end",
                    dominant_baseline: "middle",
                    "{value:.0}"
                }
            }
            for (index, snapshot) in snapshots.iter().enumerate() {
                if let Some(snapshot) = snapshot {
                    {
                        let value = point_value(snapshot, perspective, mode);
                        rsx! {
                            circle {
                                class: "chart-point",
                                cx: "{x_for(index)}",
                                cy: "{y_for(value)}",
                                r: "2.4"
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
                    let label_x = x.clamp(LEFT + 12.0, LEFT + PLOT_WIDTH - 12.0);
                    let label_y = (y - 9.0).max(TOP + 8.0);
                    rsx! {
                        circle {
                            class: "chart-current",
                            cx: "{x}",
                            cy: "{y}",
                            r: "5.5"
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
    current: Option<AnalysisSnapshot>,
    error: Option<String>,
    chart: Vec<Option<AnalysisSnapshot>>,
    current_index: usize,
    max_visits: u32,
    on_toggle: EventHandler<()>,
    on_perspective: EventHandler<()>,
    on_mode: EventHandler<()>,
) -> Element {
    rsx! {
        section {
            class: if open { "analysis-panel open" } else { "analysis-panel" },
            div { class: "analysis-header",
                button {
                    class: "analysis-toggle",
                    r#type: "button",
                    disabled: !enabled,
                    aria_expanded: open,
                    onclick: move |_| on_toggle.call(()),
                    "分析"
                }
                if open {
                    span { class: "analysis-depth", "{max_visits} visits" }
                    div { class: "analysis-controls",
                        PerspectiveToggle {
                            perspective,
                            on_click: move |_| on_perspective.call(()),
                        }
                        ModeToggle {
                            mode,
                            on_click: move |_| on_mode.call(()),
                        }
                    }
                }
            }
            if open {
                div { class: "analysis-body",
                    AnalysisChart {
                        snapshots: chart,
                        perspective,
                        mode,
                        current_index,
                    }
                    if let Some(error) = error {
                        span { class: "analysis-error", "{error}" }
                    } else if current.is_none() {
                        span { class: "analysis-status", "分析中…" }
                    }
                }
            }
        }
    }
}
