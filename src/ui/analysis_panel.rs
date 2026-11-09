use dioxus::prelude::*;
use lazygo::analysis::engine::AnalysisSnapshot;
use lazygo::game::{Move, Stone};

use super::{AnalysisMode, ModeToggle, PerspectiveToggle};

const DEFAULT_WIDTH: f64 = 320.0;
const DEFAULT_HEIGHT: f64 = 168.0;
const LEFT: f64 = 20.0;
const RIGHT: f64 = 6.0;
const TOP: f64 = 14.0;
const BOTTOM: f64 = 14.0;

const CHART_SIZE_SCRIPT: &str = r#"
const setupChartSize = () => {
    const svg = document.querySelector(".analysis-chart-svg");
    if (!svg) {
        requestAnimationFrame(setupChartSize);
        return;
    }
    if (svg.__lazygoSizeReady) return;
    svg.__lazygoSizeReady = true;

    let last = "";
    const send = () => {
        const rect = svg.getBoundingClientRect();
        const width = Math.round(rect.width * 10) / 10;
        const height = Math.round(rect.height * 10) / 10;
        if (width < 80 || height < 60) return;
        const key = `${width}x${height}`;
        if (key === last) return;
        last = key;
        dioxus.send(key);
    };
    requestAnimationFrame(send);
    new ResizeObserver(send).observe(svg);
};
setupChartSize();
"#;

const RING_ANIMATION_SCRIPT: &str = r#"
const setupRing = () => {
    const el = document.getElementById("chart-current-marker");
    if (!el) {
        requestAnimationFrame(setupRing);
        return;
    }
    if (el.__lazygoRingReady) return;
    el.__lazygoRingReady = true;

    const parseTarget = () => {
        const match = /translate\(\s*([-\d.]+)[ ,]+([-\d.]+)\s*\)/.exec(
            el.getAttribute("transform") || ""
        );
        return match ? { x: parseFloat(match[1]), y: parseFloat(match[2]) } : null;
    };
    const apply = (x, y) => {
        el.style.transform = `translate(${x}px, ${y}px)`;
    };
    const isVisible = () => !el.classList.contains("hidden");
    const state = { x: 0, y: 0, visualX: 0, visualY: 0, raf: 0, hasPosition: false };
    const target = parseTarget();

    if (target) {
        state.x = target.x;
        state.y = target.y;
        state.visualX = target.x;
        state.visualY = target.y;
        if (isVisible()) {
            state.hasPosition = true;
            apply(state.x, state.y);
            el.style.opacity = "1";
        } else {
            el.style.opacity = "0";
        }
    }

    const animate = () => {
        const next = parseTarget();
        const visible = isVisible();
        if (!next || !visible) {
            el.style.opacity = "0";
            return;
        }

        if (!state.hasPosition) {
            state.x = next.x;
            state.y = next.y;
            state.visualX = next.x;
            state.visualY = next.y;
            state.hasPosition = true;
            apply(state.x, state.y);
            el.style.opacity = "1";
            return;
        }

        el.style.opacity = "1";
        if (Math.abs(state.x - next.x) < 0.01 && Math.abs(state.y - next.y) < 0.01) {
            return;
        }

        const fromX = state.visualX;
        const fromY = state.visualY;
        const toX = next.x;
        const toY = next.y;
        state.x = toX;
        state.y = toY;
        if (state.raf) cancelAnimationFrame(state.raf);
        const start = performance.now();
        const duration = 300;
        const step = (now) => {
            const t = Math.min(1, (now - start) / duration);
            const eased = 1 - Math.pow(1 - t, 3);
            const x = fromX + (toX - fromX) * eased;
            const y = fromY + (toY - fromY) * eased;
            state.visualX = x;
            state.visualY = y;
            apply(x, y);
            if (t < 1) {
                state.raf = requestAnimationFrame(step);
            } else {
                state.visualX = toX;
                state.visualY = toY;
                state.raf = 0;
            }
        };
        state.raf = requestAnimationFrame(step);
    };

    new MutationObserver(animate).observe(el, {
        attributes: true,
        attributeFilter: ["transform", "class"],
    });
    animate();
};
setupRing();
"#;

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

fn chart_x(index: usize, total_moves: usize, width: f64) -> f64 {
    let plot_width = (width - LEFT - RIGHT).max(1.0);
    LEFT + index as f64 * plot_width / (total_moves.max(10) + 1) as f64
}

fn nearest_chart_turn(x: f64, total_moves: usize, width: f64) -> Option<usize> {
    if total_moves == 0 {
        return None;
    }
    let mut nearest = 1;
    let mut nearest_distance = f64::INFINITY;
    for index in 1..=total_moves {
        let distance = (chart_x(index, total_moves, width) - x).abs();
        if distance < nearest_distance {
            nearest_distance = distance;
            nearest = index;
        }
    }
    Some(nearest)
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
    let chart_size = use_signal(|| (DEFAULT_WIDTH, DEFAULT_HEIGHT));
    let mut chart_drag = use_signal(|| None::<i32>);
    let (width, height) = chart_size();
    let plot_height = (height - TOP - BOTTOM).max(1.0);
    let (min, max) = axis_bounds(mode);
    let y_for = |value: f64| TOP + (max - value) / (max - min) * plot_height;
    let zero_y = y_for(0.0);
    let total_moves = moves.len();
    let x_for = |index: usize| chart_x(index, total_moves, width);
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

    use_effect(move || {
        let _ = document::eval(RING_ANIMATION_SCRIPT);
    });

    use_effect(move || {
        let mut chart_size = chart_size;
        spawn(async move {
            let mut eval = document::eval(CHART_SIZE_SCRIPT);
            while let Ok(value) = eval.recv::<String>().await {
                let mut parts = value.split('x').filter_map(|part| part.parse::<f64>().ok());
                let (Some(width), Some(height)) = (parts.next(), parts.next()) else {
                    continue;
                };
                chart_size.set((width, height));
            }
        });
    });

    rsx! {
        svg {
            class: "analysis-chart-svg",
            view_box: "0 0 {width} {height}",
            role: "img",
            line {
                class: "chart-axis",
                x1: "{LEFT}",
                y1: "{TOP}",
                x2: "{LEFT}",
                y2: "{TOP + plot_height}",
            }
            line {
                class: "chart-axis",
                x1: "{LEFT}",
                y1: "{zero_y}",
                x2: "{width - RIGHT}",
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
                                    }
                                    if pass {
                                        text {
                                            class: "chart-pass",
                                            x: "{x}",
                                            y: "{y}",
                                            text_anchor: "middle",
                                            dominant_baseline: "middle",
                                            "p"
                                        }
                                    } else {
                                        circle {
                                            class: "chart-point",
                                            cx: "{x}",
                                            cy: "{y}",
                                            r: "3.2",
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            {
                let current_geometry = current.map(|snapshot| {
                    let value = point_value(snapshot, perspective, mode);
                    (x_for(current_index), y_for(value), value)
                });
                let (current_x, current_y, current_value) =
                    current_geometry.unwrap_or((0.0, 0.0, 0.0));
                let visible = current_geometry.is_some();
                rsx! {
                    g {
                        id: "chart-current-marker",
                        class: if visible {
                            "chart-current-marker"
                        } else {
                            "chart-current-marker hidden"
                        },
                        transform: "translate({current_x} {current_y})",
                        circle {
                            class: "chart-current",
                            cx: "0",
                            cy: "0",
                            r: "6.6",
                        }
                        text {
                            class: "chart-value",
                            x: "0",
                            y: "-11",
                            text_anchor: "middle",
                            "{current_value:.1}"
                        }
                    }
                }
            }
            if snapshots.iter().all(Option::is_none) {
                text {
                    class: "chart-empty",
                    x: "{LEFT + (width - LEFT - RIGHT) / 2.0}",
                    y: "{TOP + plot_height / 2.0}",
                    text_anchor: "middle",
                    "暂无分析"
                }
            }
            rect {
                class: "chart-interaction",
                x: "0",
                y: "0",
                width: "{width}",
                height: "{height}",
                onpointerdown: move |event| {
                    event.prevent_default();
                    chart_drag.set(Some(event.data().pointer_id()));
                    if let Some(turn) = nearest_chart_turn(
                        event.data().element_coordinates().x,
                        total_moves,
                        width,
                    ) {
                        on_replay.call(turn);
                    }
                },
                onpointermove: move |event| {
                    if chart_drag() != Some(event.data().pointer_id()) {
                        return;
                    }
                    event.prevent_default();
                    if let Some(turn) = nearest_chart_turn(
                        event.data().element_coordinates().x,
                        total_moves,
                        width,
                    ) {
                        on_replay.call(turn);
                    }
                },
                onpointerup: move |event| {
                    if chart_drag() == Some(event.data().pointer_id()) {
                        chart_drag.set(None);
                    }
                },
                onpointercancel: move |event| {
                    if chart_drag() == Some(event.data().pointer_id()) {
                        chart_drag.set(None);
                    }
                },
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
    use super::{chart_x, is_pass, nearest_chart_turn, LEFT};
    use lazygo::game::Move;

    #[test]
    fn chart_x_uses_one_slot_larger_than_max_ten() {
        let width = 320.0;
        let plot_width = width - LEFT - 6.0;
        assert_eq!(chart_x(0, 0, width), LEFT);
        assert_eq!(chart_x(9, 9, width), LEFT + plot_width * 9.0 / 11.0);
        assert_eq!(chart_x(19, 19, width), LEFT + plot_width * 19.0 / 20.0);
        assert_eq!(chart_x(5, 20, width), LEFT + plot_width * 5.0 / 21.0);
    }

    #[test]
    fn chart_turn_is_chosen_by_nearest_x() {
        let width = 320.0;
        let first = chart_x(1, 3, width);
        let second = chart_x(2, 3, width);
        let third = chart_x(3, 3, width);

        assert_eq!(nearest_chart_turn(first, 3, width), Some(1));
        assert_eq!(nearest_chart_turn(first + 1.0, 3, width), Some(1));
        assert_eq!(nearest_chart_turn(second, 3, width), Some(2));
        assert_eq!(nearest_chart_turn(third, 3, width), Some(3));
        assert_eq!(nearest_chart_turn(0.0, 3, width), Some(1));
        assert_eq!(nearest_chart_turn(1000.0, 3, width), Some(3));
        assert_eq!(nearest_chart_turn(100.0, 0, width), None);
    }

    #[test]
    fn pass_marker_only_applies_to_pass_moves() {
        let moves = vec![Move::Play(4, 4), Move::Pass];
        assert!(!is_pass(&moves, 0));
        assert!(!is_pass(&moves, 1));
        assert!(is_pass(&moves, 2));
    }
}
