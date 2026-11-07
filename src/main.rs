use dioxus::prelude::*;

const MAIN_CSS: Asset = asset!("/assets/main.css");
const APP_ICON: Asset = asset!("/assets/icon.png");
const APP_FONT: Asset = asset!("/assets/fonts/maiyuan-ii-regular.ttf");

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stone {
    Black,
    White,
}

impl Stone {
    fn other(self) -> Self {
        match self {
            Self::Black => Self::White,
            Self::White => Self::Black,
        }
    }
}

#[derive(Clone, PartialEq)]
struct Game {
    size: usize,
    stones: Vec<Option<Stone>>,
    turn: Stone,
    last_move: Option<(usize, usize)>,
    history: Vec<Vec<Option<Stone>>>,
}

fn new_game(size: usize) -> Game {
    let stones = vec![None; size * size];
    Game {
        size,
        stones: stones.clone(),
        turn: Stone::Black,
        last_move: None,
        history: vec![stones],
    }
}

fn is_initial(game: &Game) -> bool {
    game.history.len() == 1
        && game.turn == Stone::Black
        && game.last_move.is_none()
        && game.stones.iter().all(Option::is_none)
}

fn neighbors(index: usize, size: usize) -> Vec<usize> {
    let x = index % size;
    let y = index / size;
    let mut result = Vec::with_capacity(4);

    if x > 0 {
        result.push(index - 1);
    }
    if x + 1 < size {
        result.push(index + 1);
    }
    if y > 0 {
        result.push(index - size);
    }
    if y + 1 < size {
        result.push(index + size);
    }

    result
}

fn group_and_liberties(stones: &[Option<Stone>], size: usize, start: usize) -> (Vec<usize>, usize) {
    let color = stones[start];
    let mut group = Vec::new();
    let mut stack = vec![start];
    let mut seen = vec![false; stones.len()];
    let mut liberty_seen = vec![false; stones.len()];
    let mut liberties = 0;

    while let Some(index) = stack.pop() {
        if seen[index] {
            continue;
        }

        seen[index] = true;
        group.push(index);

        for neighbor in neighbors(index, size) {
            match stones[neighbor] {
                None => {
                    if !liberty_seen[neighbor] {
                        liberty_seen[neighbor] = true;
                        liberties += 1;
                    }
                }
                Some(stone) if Some(stone) == color => {
                    if !seen[neighbor] {
                        stack.push(neighbor);
                    }
                }
                _ => {}
            }
        }
    }

    (group, liberties)
}

fn play(game: &Game, x: usize, y: usize) -> Option<Game> {
    let size = game.size;

    if x >= size || y >= size {
        return None;
    }

    let index = y * size + x;

    if game.stones[index].is_some() {
        return None;
    }

    let mut next = game.clone();
    next.stones[index] = Some(game.turn);
    let opponent = game.turn.other();
    let mut visited = vec![false; size * size];

    for neighbor in neighbors(index, size) {
        if visited[neighbor] {
            continue;
        }

        if next.stones[neighbor] == Some(opponent) {
            let (group, liberties) = group_and_liberties(&next.stones, size, neighbor);

            for member in &group {
                visited[*member] = true;
            }

            if liberties == 0 {
                for member in group {
                    next.stones[member] = None;
                }
            }
        }
    }

    let (_, liberties) = group_and_liberties(&next.stones, size, index);

    if liberties == 0 {
        return None;
    }

    if next.history.iter().any(|position| position == &next.stones) {
        return None;
    }

    next.history.push(next.stones.clone());
    next.last_move = Some((x, y));
    next.turn = opponent;

    Some(next)
}

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
    let mut game = use_signal(|| new_game(9));
    let mut menu_open = use_signal(|| false);
    let mut reset_armed = use_signal(|| false);
    let mut reset_rotation = use_signal(|| 0i32);
    let current = game();
    let size = current.size;
    let max = size - 1;
    let board_margin = format!("--board-margin: {:.6}vw;", 100.0 / (size as f64 + 2.0));
    let initial = is_initial(&current);
    let star_radius = 40.0 / 304.0;
    let hit_radius = 0.36;
    let stars = star_points(size);
    let (back, front) = match current.turn {
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
                                if let Some(stone) = current.stones[y * size + x] {
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
                                    if current.last_move == Some((x, y)) {
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

                                        if let Some(next) = play(&position, x, y) {
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
                                    let mut next = game();
                                    next.turn = next.turn.other();
                                    next.last_move = None;
                                    game.set(next);
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
                                        game.set(new_game(size));
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
                                    for option in [9usize, 13usize, 19usize] {
                                        button {
                                            class: if size == option { "size-option selected" } else { "size-option" },
                                            r#type: "button",
                                            onclick: move |_| {
                                                game.set(new_game(option));
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
