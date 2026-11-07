pub const BOARD_SIZES: [usize; 3] = [9, 13, 19];
pub const DEFAULT_SIZE: usize = 9;

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
pub struct Game {
    size: usize,
    stones: Vec<Option<Stone>>,
    turn: Stone,
    last_move: Option<(usize, usize)>,
    history: Vec<Vec<Option<Stone>>>,
}

impl Game {
    pub fn new(size: usize) -> Self {
        let stones = vec![None; size * size];

        Self {
            size,
            stones: stones.clone(),
            turn: Stone::Black,
            last_move: None,
            history: vec![stones],
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn turn(&self) -> Stone {
        self.turn
    }

    pub fn last_move(&self) -> Option<(usize, usize)> {
        self.last_move
    }

    pub fn stone_at(&self, x: usize, y: usize) -> Option<Stone> {
        if x >= self.size || y >= self.size {
            return None;
        }

        self.stones[y * self.size + x]
    }

    pub fn is_initial(&self) -> bool {
        self.history.len() == 1
            && self.turn == Stone::Black
            && self.last_move.is_none()
            && self.stones.iter().all(Option::is_none)
    }

    pub fn play(&self, x: usize, y: usize) -> Option<Self> {
        if x >= self.size || y >= self.size {
            return None;
        }

        let index = y * self.size + x;

        if self.stones[index].is_some() {
            return None;
        }

        let mut next = self.clone();
        next.stones[index] = Some(self.turn);
        let opponent = self.turn.other();
        let mut visited = vec![false; self.size * self.size];

        for neighbor in neighbors(index, self.size) {
            if visited[neighbor] {
                continue;
            }

            if next.stones[neighbor] == Some(opponent) {
                let (group, liberties) = group_and_liberties(&next.stones, self.size, neighbor);

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

        let (_, liberties) = group_and_liberties(&next.stones, self.size, index);

        if liberties == 0 || next.history.iter().any(|position| position == &next.stones) {
            return None;
        }

        next.history.push(next.stones.clone());
        next.last_move = Some((x, y));
        next.turn = opponent;

        Some(next)
    }

    pub fn pass(&self) -> Self {
        let mut next = self.clone();
        next.turn = self.turn.other();
        next.last_move = None;
        next
    }
}

pub fn star_points(size: usize) -> Vec<(usize, usize)> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn position(
        size: usize,
        black: &[(usize, usize)],
        white: &[(usize, usize)],
        turn: Stone,
    ) -> Game {
        let mut stones = vec![None; size * size];

        for &(x, y) in black {
            stones[y * size + x] = Some(Stone::Black);
        }

        for &(x, y) in white {
            stones[y * size + x] = Some(Stone::White);
        }

        Game {
            size,
            stones: stones.clone(),
            turn,
            last_move: None,
            history: vec![stones],
        }
    }

    #[test]
    fn new_game_is_empty_and_black_to_play() {
        let game = Game::new(9);

        assert_eq!(game.size(), 9);
        assert_eq!(game.turn(), Stone::Black);
        assert_eq!(game.last_move(), None);
        assert!(game.is_initial());

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
    fn rejects_suicide() {
        let game = position(3, &[(0, 1), (2, 1), (1, 0), (1, 2)], &[], Stone::White);

        assert!(game.play(1, 1).is_none());
    }

    #[test]
    fn rejects_positional_superko() {
        let mut repeated = vec![None; 9];
        repeated[4] = Some(Stone::Black);

        let game = Game {
            size: 3,
            stones: vec![None; 9],
            turn: Stone::Black,
            last_move: None,
            history: vec![repeated],
        };

        assert!(game.play(1, 1).is_none());
    }

    #[test]
    fn pass_switches_turn_and_clears_last_move() {
        let game = Game::new(9).play(4, 4).unwrap();
        let game = game.pass();

        assert_eq!(game.turn(), Stone::Black);
        assert_eq!(game.last_move(), None);
        assert_eq!(game.stone_at(4, 4), Some(Stone::Black));
        assert!(!game.is_initial());
    }
}
