pub const BOARD_SIZES: [usize; 3] = [9, 13, 19];
pub const DEFAULT_SIZE: usize = 9;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Rules {
    #[default]
    Chinese,
    Japanese,
    Ing,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum KoRule {
    Simple,
    Positional,
}

impl Rules {
    pub const ALL: [Self; 3] = [Self::Chinese, Self::Japanese, Self::Ing];

    pub fn label(self) -> &'static str {
        match self {
            Self::Chinese => "中国",
            Self::Japanese => "日韩",
            Self::Ing => "应氏",
        }
    }

    fn allows_suicide(self) -> bool {
        self == Self::Ing
    }

    fn ko_rule(self) -> KoRule {
        match self {
            Self::Japanese => KoRule::Simple,
            Self::Chinese | Self::Ing => KoRule::Positional,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Move {
    Play(usize, usize),
    Pass,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stone {
    Black,
    White,
}

impl Stone {
    pub fn other(self) -> Self {
        match self {
            Self::Black => Self::White,
            Self::White => Self::Black,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
struct Board {
    size: usize,
    stones: Vec<Option<Stone>>,
}

impl Board {
    fn new(size: usize) -> Self {
        Self {
            size,
            stones: vec![None; size * size],
        }
    }

    fn size(&self) -> usize {
        self.size
    }

    fn len(&self) -> usize {
        self.stones.len()
    }

    fn index(&self, x: usize, y: usize) -> Option<usize> {
        (x < self.size && y < self.size).then_some(y * self.size + x)
    }

    fn get(&self, x: usize, y: usize) -> Option<Stone> {
        self.index(x, y).and_then(|index| self.stones[index])
    }

    fn get_index(&self, index: usize) -> Option<Stone> {
        self.stones[index]
    }

    fn set(&mut self, index: usize, stone: Option<Stone>) {
        self.stones[index] = stone;
    }

    fn remove(&mut self, group: &[usize]) {
        for &index in group {
            self.stones[index] = None;
        }
    }

    fn neighbors(&self, index: usize) -> Vec<usize> {
        let x = index % self.size;
        let y = index / self.size;
        let mut result = Vec::with_capacity(4);

        if x > 0 {
            result.push(index - 1);
        }
        if x + 1 < self.size {
            result.push(index + 1);
        }
        if y > 0 {
            result.push(index - self.size);
        }
        if y + 1 < self.size {
            result.push(index + self.size);
        }

        result
    }

    fn group_and_liberties(&self, start: usize) -> (Vec<usize>, usize) {
        let color = self.stones[start];
        let mut group = Vec::new();
        let mut stack = vec![start];
        let mut seen = vec![false; self.len()];
        let mut liberty_seen = vec![false; self.len()];
        let mut liberties = 0;

        while let Some(index) = stack.pop() {
            if seen[index] {
                continue;
            }

            seen[index] = true;
            group.push(index);

            for neighbor in self.neighbors(index) {
                match self.stones[neighbor] {
                    None => {
                        if !liberty_seen[neighbor] {
                            liberty_seen[neighbor] = true;
                            liberties += 1;
                        }
                    }
                    Some(stone) if Some(stone) == color && !seen[neighbor] => {
                        stack.push(neighbor);
                    }
                    _ => {}
                }
            }
        }

        (group, liberties)
    }
}

#[derive(Clone, PartialEq, Debug)]
struct State {
    board: Board,
    turn: Stone,
    last_move: Option<(usize, usize)>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Game {
    rules: Rules,
    states: Vec<State>,
    moves: Vec<Move>,
    cursor: usize,
}

impl Game {
    pub fn new(size: usize) -> Self {
        Self::with_rules(size, Rules::default())
    }

    pub fn with_rules(size: usize, rules: Rules) -> Self {
        Self {
            rules,
            states: vec![State {
                board: Board::new(size),
                turn: Stone::Black,
                last_move: None,
            }],
            moves: Vec::new(),
            cursor: 0,
        }
    }

    fn current(&self) -> &State {
        &self.states[self.cursor]
    }

    pub fn size(&self) -> usize {
        self.current().board.size()
    }

    pub fn rules(&self) -> Rules {
        self.rules
    }

    pub fn turn(&self) -> Stone {
        self.current().turn
    }

    pub fn last_move(&self) -> Option<(usize, usize)> {
        self.current().last_move
    }

    pub fn moves(&self) -> &[Move] {
        &self.moves[..self.cursor.min(self.moves.len())]
    }

    pub fn stone_at(&self, x: usize, y: usize) -> Option<Stone> {
        self.current().board.get(x, y)
    }

    pub fn is_initial(&self) -> bool {
        self.cursor == 0
    }

    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    pub fn can_redo(&self) -> bool {
        self.cursor + 1 < self.states.len()
    }

    pub fn undo(&self) -> Option<Self> {
        if !self.can_undo() {
            return None;
        }

        let mut next = self.clone();
        next.cursor -= 1;
        Some(next)
    }

    pub fn redo(&self) -> Option<Self> {
        if !self.can_redo() {
            return None;
        }

        let mut next = self.clone();
        next.cursor += 1;
        Some(next)
    }

    pub fn play(&self, x: usize, y: usize) -> Option<Self> {
        let current = self.current();
        let index = current.board.index(x, y)?;

        if current.board.get_index(index).is_some() {
            return None;
        }

        let color = current.turn;
        let opponent = color.other();
        let mut board = current.board.clone();
        board.set(index, Some(color));
        let mut visited = vec![false; board.len()];

        for neighbor in board.neighbors(index) {
            if visited[neighbor] {
                continue;
            }

            if board.get_index(neighbor) == Some(opponent) {
                let (group, liberties) = board.group_and_liberties(neighbor);

                for &member in &group {
                    visited[member] = true;
                }

                if liberties == 0 {
                    board.remove(&group);
                }
            }
        }

        let (group, liberties) = board.group_and_liberties(index);
        let self_capture = liberties == 0;

        if self_capture && !self.rules.allows_suicide() {
            return None;
        }

        let pass_like = self_capture && self.rules == Rules::Ing && group.len() == 1;

        if self_capture {
            board.remove(&group);
        }

        let repeats = if pass_like {
            false
        } else {
            match self.rules.ko_rule() {
                KoRule::Simple => self.cursor > 0 && self.states[self.cursor - 1].board == board,
                KoRule::Positional => self.states[..=self.cursor]
                    .iter()
                    .any(|state| state.board == board),
            }
        };

        if repeats {
            return None;
        }

        let mut next = self.clone();
        next.states.truncate(self.cursor + 1);
        next.moves.truncate(self.cursor);
        next.moves.push(Move::Play(x, y));
        next.states.push(State {
            board,
            turn: opponent,
            last_move: if self_capture { None } else { Some((x, y)) },
        });
        next.cursor += 1;
        Some(next)
    }

    pub fn pass(&self) -> Self {
        let current = self.current();
        let mut next = self.clone();
        next.states.truncate(self.cursor + 1);
        next.moves.truncate(self.cursor);
        next.moves.push(Move::Pass);
        next.states.push(State {
            board: current.board.clone(),
            turn: current.turn.other(),
            last_move: None,
        });
        next.cursor += 1;
        next
    }
}

const STAR_POINTS_9: [(usize, usize); 5] = [(2, 2), (2, 6), (4, 4), (6, 2), (6, 6)];
const STAR_POINTS_13: [(usize, usize); 5] = [(3, 3), (3, 9), (6, 6), (9, 3), (9, 9)];
const STAR_POINTS_19: [(usize, usize); 9] = [
    (3, 3),
    (3, 9),
    (3, 15),
    (9, 3),
    (9, 9),
    (9, 15),
    (15, 3),
    (15, 9),
    (15, 15),
];

pub fn star_points(size: usize) -> &'static [(usize, usize)] {
    match size {
        9 => &STAR_POINTS_9,
        13 => &STAR_POINTS_13,
        19 => &STAR_POINTS_19,
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board(size: usize, black: &[(usize, usize)], white: &[(usize, usize)]) -> Board {
        let mut board = Board::new(size);

        for &(x, y) in black {
            board.set(board.index(x, y).unwrap(), Some(Stone::Black));
        }

        for &(x, y) in white {
            board.set(board.index(x, y).unwrap(), Some(Stone::White));
        }

        board
    }

    fn state(
        size: usize,
        black: &[(usize, usize)],
        white: &[(usize, usize)],
        turn: Stone,
    ) -> State {
        State {
            board: board(size, black, white),
            turn,
            last_move: None,
        }
    }

    fn position(
        size: usize,
        black: &[(usize, usize)],
        white: &[(usize, usize)],
        turn: Stone,
    ) -> Game {
        position_with_rules(size, black, white, turn, Rules::default())
    }

    fn position_with_rules(
        size: usize,
        black: &[(usize, usize)],
        white: &[(usize, usize)],
        turn: Stone,
        rules: Rules,
    ) -> Game {
        Game {
            rules,
            states: vec![state(size, black, white, turn)],
            moves: Vec::new(),
            cursor: 0,
        }
    }

    fn ko_game(rules: Rules) -> Game {
        Game {
            rules,
            states: vec![
                state(
                    5,
                    &[(0, 1), (1, 0), (1, 2)],
                    &[(2, 0), (1, 1), (2, 2), (3, 1)],
                    Stone::Black,
                ),
                state(
                    5,
                    &[(0, 1), (1, 0), (1, 2), (2, 1)],
                    &[(2, 0), (2, 2), (3, 1)],
                    Stone::White,
                ),
            ],
            moves: vec![Move::Play(2, 1)],
            cursor: 1,
        }
    }

    fn ko_game_after_passes(rules: Rules) -> Game {
        let mut game = ko_game(rules);
        let captured = game.states[1].clone();
        game.states.push(State {
            board: captured.board.clone(),
            turn: Stone::Black,
            last_move: None,
        });
        game.states.push(State {
            board: captured.board,
            turn: Stone::White,
            last_move: None,
        });
        game.moves = vec![Move::Play(2, 1), Move::Pass, Move::Pass];
        game.cursor = 3;
        game
    }

    #[test]
    fn new_game_is_empty_and_black_to_play() {
        let game = Game::new(9);

        assert_eq!(game.size(), 9);
        assert_eq!(game.rules(), Rules::Chinese);
        assert_eq!(game.turn(), Stone::Black);
        assert_eq!(game.last_move(), None);
        assert!(game.is_initial());
        assert!(!game.can_undo());
        assert!(!game.can_redo());

        for y in 0..9 {
            for x in 0..9 {
                assert_eq!(game.stone_at(x, y), None);
            }
        }
    }

    #[test]
    fn play_places_stone_and_switches_turn() {
        let game = Game::new(9).play(4, 4).unwrap();

        assert_eq!(game.stone_at(4, 4), Some(Stone::Black));
        assert_eq!(game.turn(), Stone::White);
        assert_eq!(game.last_move(), Some((4, 4)));
        assert!(!game.is_initial());
        assert!(game.can_undo());
        assert!(!game.can_redo());
    }

    #[test]
    fn rejects_occupied_and_out_of_bounds_points() {
        let game = Game::new(9).play(4, 4).unwrap();

        assert_eq!(game.stone_at(9, 0), None);
        assert_eq!(game.stone_at(0, 9), None);
        assert!(game.play(4, 4).is_none());
        assert!(game.play(9, 0).is_none());
        assert!(game.play(0, 9).is_none());
    }

    #[test]
    fn captures_surrounded_stone() {
        let game = position(3, &[(0, 1), (2, 1), (1, 0)], &[(1, 1)], Stone::Black);

        let game = game.play(1, 2).unwrap();

        assert_eq!(game.stone_at(1, 1), None);
        assert_eq!(game.stone_at(1, 2), Some(Stone::Black));
        assert_eq!(game.turn(), Stone::White);
    }

    #[test]
    fn rejects_suicide_in_chinese_and_japanese_rules() {
        for rules in [Rules::Chinese, Rules::Japanese] {
            let game = position_with_rules(
                3,
                &[(0, 1), (2, 1), (1, 0), (1, 2)],
                &[],
                Stone::White,
                rules,
            );

            assert!(game.play(1, 1).is_none());
        }
    }

    #[test]
    fn ing_allows_single_stone_suicide_as_pass() {
        let game = position_with_rules(
            3,
            &[(0, 1), (2, 1), (1, 0), (1, 2)],
            &[],
            Stone::White,
            Rules::Ing,
        );
        let game = game.play(1, 1).unwrap();

        assert_eq!(game.stone_at(1, 1), None);
        assert_eq!(game.turn(), Stone::Black);
        assert_eq!(game.last_move(), None);
        assert!(!game.is_initial());
    }

    #[test]
    fn ing_allows_multi_stone_suicide() {
        let game = position_with_rules(
            3,
            &[(0, 0), (0, 1), (2, 0), (2, 1), (1, 2)],
            &[(1, 1)],
            Stone::White,
            Rules::Ing,
        );
        let game = game.play(1, 0).unwrap();

        assert_eq!(game.stone_at(1, 0), None);
        assert_eq!(game.stone_at(1, 1), None);
        assert_eq!(game.stone_at(0, 0), Some(Stone::Black));
        assert_eq!(game.turn(), Stone::Black);
        assert_eq!(game.last_move(), None);
    }

    #[test]
    fn rejects_positional_superko() {
        let game = Game {
            rules: Rules::Chinese,
            states: vec![
                State {
                    board: board(3, &[(1, 1)], &[]),
                    turn: Stone::White,
                    last_move: None,
                },
                State {
                    board: Board::new(3),
                    turn: Stone::Black,
                    last_move: None,
                },
            ],
            moves: Vec::new(),
            cursor: 1,
        };

        assert!(game.play(1, 1).is_none());
    }

    #[test]
    fn ko_recapture_is_illegal_in_every_ruleset() {
        for rules in Rules::ALL {
            assert!(ko_game(rules).play(1, 1).is_none());
        }
    }

    #[test]
    fn japanese_simple_ko_lifts_after_passes() {
        let game = ko_game_after_passes(Rules::Japanese).play(1, 1).unwrap();

        assert_eq!(game.stone_at(1, 1), Some(Stone::White));
        assert_eq!(game.stone_at(2, 1), None);
        assert_eq!(game.turn(), Stone::Black);
    }

    #[test]
    fn chinese_and_ing_reject_ko_repetition_after_passes() {
        for rules in [Rules::Chinese, Rules::Ing] {
            assert!(ko_game_after_passes(rules).play(1, 1).is_none());
        }
    }

    #[test]
    fn undo_and_redo_restore_state() {
        let game = Game::new(9).play(4, 4).unwrap();
        let undone = game.undo().unwrap();

        assert_eq!(undone.stone_at(4, 4), None);
        assert_eq!(undone.turn(), Stone::Black);
        assert_eq!(undone.last_move(), None);
        assert!(undone.is_initial());
        assert!(undone.can_redo());

        let redone = undone.redo().unwrap();

        assert_eq!(redone.stone_at(4, 4), Some(Stone::Black));
        assert_eq!(redone.turn(), Stone::White);
        assert_eq!(redone.last_move(), Some((4, 4)));
        assert!(!redone.can_redo());
    }

    #[test]
    fn new_move_after_undo_discards_redo() {
        let game = Game::new(9).play(4, 4).unwrap();
        let game = game.undo().unwrap().play(3, 3).unwrap();

        assert_eq!(game.stone_at(3, 3), Some(Stone::Black));
        assert_eq!(game.stone_at(4, 4), None);
        assert!(!game.can_redo());
    }

    #[test]
    fn exposes_rule_labels_and_preserves_selected_rules() {
        assert_eq!(Rules::default(), Rules::Chinese);
        assert_eq!(
            Rules::ALL.map(|rules| rules.label()),
            ["中国", "日韩", "应氏"]
        );
        assert_eq!(Game::with_rules(9, Rules::Ing).rules(), Rules::Ing);
    }

    #[test]
    fn records_play_and_pass_history() {
        let game = Game::new(9).play(4, 4).unwrap();
        assert_eq!(game.moves(), &[Move::Play(4, 4)]);

        let game = game.pass();
        assert_eq!(game.moves(), &[Move::Play(4, 4), Move::Pass]);

        let undone = game.undo().unwrap();
        assert_eq!(undone.moves(), &[Move::Play(4, 4)]);

        let branched = undone.play(3, 3).unwrap();
        assert_eq!(branched.moves(), &[Move::Play(4, 4), Move::Play(3, 3)]);
    }

    #[test]
    fn pass_is_undoable() {
        let game = Game::new(9).pass();

        assert_eq!(game.turn(), Stone::White);
        assert_eq!(game.last_move(), None);
        assert!(!game.is_initial());

        let undone = game.undo().unwrap();

        assert_eq!(undone.turn(), Stone::Black);
        assert!(undone.is_initial());

        let redone = undone.redo().unwrap();

        assert_eq!(redone.turn(), Stone::White);
        assert_eq!(redone.last_move(), None);
    }
}
