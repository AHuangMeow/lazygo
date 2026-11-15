use crate::analysis::engine::AnalysisQuery;
use crate::game::{Game, Komi, Move, Rules, DEFAULT_SIZE};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PendingMove {
    pub x: usize,
    pub y: usize,
    pub legal: bool,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Session {
    game: Game,
    started: bool,
    pass_armed: bool,
    pending: Option<PendingMove>,
    reset_armed: bool,
    komi: Komi,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub fn new() -> Self {
        Self {
            game: Game::new(DEFAULT_SIZE),
            started: false,
            pass_armed: false,
            pending: None,
            reset_armed: false,
            komi: Rules::Chinese.default_komi(),
        }
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    pub fn analysis_query(&self, max_visits: u32) -> AnalysisQuery {
        AnalysisQuery {
            node_id: self.game.node_id(),
            size: self.game.size(),
            rules: self.game.rules(),
            komi: self.komi.value(),
            moves: self.game.moves().to_vec(),
            max_visits,
        }
    }

    pub fn pending(&self) -> Option<PendingMove> {
        self.pending
    }

    pub fn reset_armed(&self) -> bool {
        self.reset_armed
    }

    pub fn komi(&self) -> Komi {
        self.komi
    }

    pub fn settings_locked(&self) -> bool {
        self.started
    }

    pub fn pass_armed(&self) -> bool {
        self.pass_armed
    }

    pub fn can_confirm(&self) -> bool {
        self.pending.is_some_and(|pending| pending.legal)
    }

    pub fn cancel_pending(&mut self) {
        self.pending = None;
    }

    pub fn select(&mut self, x: usize, y: usize) {
        if self.game.is_replaying() || x >= self.game.size() || y >= self.game.size() {
            return;
        }

        if let Some(pending) = self.pending {
            if pending.x == x && pending.y == y {
                self.pending = None;
                return;
            }
        }
        if self.game.stone_at(x, y).is_some() {
            return;
        }

        let legal = self.game.play(x, y).is_some();
        self.pending = Some(PendingMove { x, y, legal });
    }

    pub fn confirm(&mut self) {
        if self.game.is_replaying() {
            return;
        }
        let Some(pending) = self.pending else {
            return;
        };

        if !pending.legal {
            return;
        }

        self.pending = None;

        if let Some(next) = self.game.play(pending.x, pending.y) {
            self.game = next;
            self.started = true;
        }

        self.dismiss_overlays();
    }

    pub fn toggle_pass(&mut self) {
        if self.game.is_replaying() {
            return;
        }
        if self.pass_armed {
            self.pass_armed = false;
            self.pass();
        } else {
            self.pending = None;
            self.pass_armed = true;
            self.reset_armed = false;
        }
    }

    pub fn cancel_pass(&mut self) {
        self.pass_armed = false;
    }

    pub fn pass(&mut self) {
        if self.game.is_replaying() {
            return;
        }
        self.game = self.game.pass();
        self.started = true;
        self.pending = None;
        self.dismiss_overlays();
    }

    pub fn undo(&mut self) {
        if let Some(next) = self.game.undo() {
            self.game = next;
            self.pending = None;
            self.dismiss_overlays();
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.game.redo() {
            self.game = next;
            self.pending = None;
            self.dismiss_overlays();
        }
    }

    pub fn is_replaying(&self) -> bool {
        self.game.is_replaying()
    }

    pub fn replay_to(&mut self, turn: usize) {
        if let Some(next) = self.game.replay_to(turn) {
            self.game = next;
            self.pending = None;
            self.pass_armed = false;
            self.dismiss_overlays();
        }
    }

    pub fn analysis_query_at(&self, turn: usize, max_visits: u32) -> AnalysisQuery {
        let mut moves = self.game.all_moves().to_vec();
        let turn = turn.min(moves.len());
        moves.truncate(turn);
        let node_id = self
            .game
            .node_id_at(turn)
            .unwrap_or_else(|| self.game.node_id());

        AnalysisQuery {
            node_id,
            size: self.game.size(),
            rules: self.game.rules(),
            komi: self.komi.value(),
            moves,
            max_visits,
        }
    }

    pub fn toggle_reset(&mut self) {
        self.pass_armed = false;
        self.pending = None;

        if self.reset_armed {
            self.game = Game::with_rules(self.game.size(), self.game.rules());
            self.started = false;
            self.reset_armed = false;
        } else {
            self.reset_armed = true;
        }
    }

    pub fn cancel_reset(&mut self) {
        self.reset_armed = false;
    }

    pub fn set_size(&mut self, size: usize) {
        if self.settings_locked() {
            return;
        }

        if size != self.game.size() {
            self.game = Game::with_rules(size, self.game.rules());
            self.started = false;
            self.pending = None;
        }

        self.dismiss_overlays();
    }

    pub fn set_rules(&mut self, rules: Rules) {
        if self.settings_locked() {
            return;
        }

        if rules != self.game.rules() {
            self.game = Game::with_rules(self.game.size(), rules);
            self.started = false;
            self.pending = None;
            self.komi = rules.default_komi();
        }

        self.dismiss_overlays();
    }

    pub fn set_komi(&mut self, komi: Komi) {
        if self.settings_locked() {
            return;
        }

        self.komi = komi;
    }

    pub fn import_sgf(&mut self, text: &str) -> Result<(), String> {
        let record = crate::sgf::parse_sgf(text)?;
        let mut game = Game::with_rules(record.size, record.rules);

        for (index, mv) in record.moves.iter().enumerate() {
            let next = match *mv {
                Move::Play(x, y) => game.play(x, y),
                Move::Pass => Some(game.pass()),
            };
            game = next.ok_or_else(|| format!("第 {} 手非法落子", index + 1))?;
        }

        self.game = game;
        self.komi = record.komi;
        self.started = true;
        self.pending = None;
        self.dismiss_overlays();
        Ok(())
    }

    pub fn sgf_text(&self) -> String {
        crate::sgf::write_sgf(&crate::sgf::SgfRecord {
            size: self.game.size(),
            rules: self.game.rules(),
            komi: self.komi,
            moves: self.game.all_moves().to_vec(),
        })
    }

    fn dismiss_overlays(&mut self) {
        self.reset_armed = false;
        self.pass_armed = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Stone;

    #[test]
    fn staged_move_confirms_and_locks_settings() {
        let mut session = Session::new();

        session.select(4, 4);
        assert!(session.pending().is_some());
        assert!(session.can_confirm());
        assert!(!session.settings_locked());

        session.confirm();

        assert_eq!(session.game().stone_at(4, 4), Some(Stone::Black));
        assert!(session.pending().is_none());
        assert!(session.settings_locked());
    }

    #[test]
    fn ignores_out_of_bounds_points() {
        let mut session = Session::new();

        session.select(9, 9);

        assert!(session.pending().is_none());
    }

    #[test]
    fn tapping_the_same_point_cancels() {
        let mut session = Session::new();

        session.select(4, 4);
        session.select(4, 4);

        assert!(session.pending().is_none());
        assert!(!session.can_confirm());
    }

    #[test]
    fn tapping_another_point_moves_pending() {
        let mut session = Session::new();

        session.select(4, 4);
        session.select(3, 3);

        assert_eq!(
            session.pending(),
            Some(PendingMove {
                x: 3,
                y: 3,
                legal: true
            })
        );
        assert!(session.can_confirm());
    }

    #[test]
    fn tapping_an_occupied_point_keeps_pending() {
        let mut session = Session::new();

        session.select(4, 4);
        session.confirm();
        session.select(3, 3);
        session.select(4, 4);

        assert_eq!(
            session.pending(),
            Some(PendingMove {
                x: 3,
                y: 3,
                legal: true
            })
        );
    }

    #[test]
    fn imports_sgf_and_enters_started_state() {
        let mut session = Session::new();

        session
            .import_sgf("(;GM[1]FF[4]SZ[13]KM[6.5]RU[Japanese];B[dd];W[ee];B[])")
            .unwrap();

        assert!(session.settings_locked());
        assert!(session.pending().is_none());
        assert_eq!(session.game().size(), 13);
        assert_eq!(session.game().rules(), Rules::Japanese);
        assert_eq!(session.komi().value(), 6.5);
        assert_eq!(session.game().all_moves().len(), 3);
        assert_eq!(session.game().cursor(), 3);
    }

    #[test]
    fn sgf_import_is_atomic_on_invalid_moves() {
        let mut session = Session::new();
        session.select(4, 4);
        session.confirm();
        let before = session.game().clone();

        let error = session.import_sgf("(;SZ[9];B[aa];B[bb])").unwrap_err();

        assert!(error.contains("颜色"));
        assert_eq!(session.game(), &before);
    }

    #[test]
    fn sgf_round_trips_current_game() {
        let mut session = Session::new();
        session.select(4, 4);
        session.confirm();
        session.select(3, 3);
        session.confirm();
        session.pass();

        let text = session.sgf_text();
        let record = crate::sgf::parse_sgf(&text).unwrap();

        assert_eq!(record.size, 9);
        assert_eq!(record.moves, session.game().all_moves());
        assert_eq!(record.komi.value(), 7.5);
    }

    #[test]
    fn rules_and_size_are_preserved_until_reset() {
        let mut session = Session::new();

        session.set_rules(Rules::Japanese);
        session.set_size(13);

        assert_eq!(session.game().rules(), Rules::Japanese);
        assert_eq!(session.game().size(), 13);

        session.select(6, 6);
        session.confirm();
        session.toggle_reset();
        session.toggle_reset();

        assert!(session.game().is_initial());
        assert!(!session.settings_locked());
        assert_eq!(session.game().rules(), Rules::Japanese);
        assert_eq!(session.game().size(), 13);
        assert_eq!(session.komi().value(), 6.5);
    }

    #[test]
    fn settings_cannot_change_after_the_first_move() {
        let mut session = Session::new();

        session.select(4, 4);
        session.confirm();
        session.set_rules(Rules::Ing);
        session.set_size(19);
        session.set_komi(Komi::parse("7.5").unwrap());

        assert_eq!(session.game().rules(), Rules::Chinese);
        assert_eq!(session.game().size(), 9);
        assert_eq!(session.komi().value(), 7.5);
    }

    #[test]
    fn starting_a_new_rule_selection_clears_pending() {
        let mut session = Session::new();

        session.select(4, 4);
        session.set_rules(Rules::Ing);

        assert!(session.pending().is_none());
        assert!(session.game().is_initial());
        assert_eq!(session.game().rules(), Rules::Ing);
        assert_eq!(session.komi().value(), 7.5);
    }

    #[test]
    fn rules_set_their_default_komi() {
        let mut session = Session::new();

        assert_eq!(session.komi().value(), 7.5);

        session.set_rules(Rules::Japanese);
        assert_eq!(session.komi().value(), 6.5);

        session.set_rules(Rules::Ing);
        assert_eq!(session.komi().value(), 7.5);

        session.set_rules(Rules::Chinese);
        assert_eq!(session.komi().value(), 7.5);
    }

    #[test]
    fn undo_to_initial_keeps_settings_locked() {
        let mut session = Session::new();

        session.select(4, 4);
        session.confirm();
        session.undo();

        assert!(session.game().is_initial());
        assert!(session.settings_locked());

        session.set_rules(Rules::Ing);
        assert_eq!(session.game().rules(), Rules::Chinese);

        session.toggle_reset();
        session.toggle_reset();

        assert!(!session.settings_locked());
    }

    #[test]
    fn pass_indicator_arms_then_passes() {
        let mut session = Session::new();

        session.toggle_pass();

        assert!(session.pass_armed());
        assert!(session.game().is_initial());
        assert!(!session.settings_locked());

        session.toggle_pass();

        assert!(!session.pass_armed());
        assert_eq!(session.game().turn(), Stone::White);
        assert!(session.settings_locked());
    }

    #[test]
    fn canceling_pass_leaves_the_game_untouched() {
        let mut session = Session::new();

        session.toggle_pass();
        session.cancel_pass();

        assert!(!session.pass_armed());
        assert!(session.game().is_initial());
        assert_eq!(session.game().turn(), Stone::Black);
    }

    #[test]
    fn arming_pass_cancels_reset() {
        let mut session = Session::new();

        session.toggle_reset();
        assert!(session.reset_armed());

        session.toggle_pass();

        assert!(session.pass_armed());
        assert!(!session.reset_armed());
    }

    #[test]
    fn replaying_blocks_new_moves_and_passes() {
        let mut session = Session::new();
        session.select(4, 4);
        session.confirm();
        session.select(3, 3);
        session.confirm();
        session.undo();

        assert!(session.is_replaying());
        session.select(2, 2);
        assert!(session.pending().is_none());
        session.toggle_pass();
        assert!(!session.pass_armed());
        session.pass();
        assert_eq!(session.game().moves().len(), 1);

        session.replay_to(2);
        assert!(!session.is_replaying());
        assert_eq!(session.game().moves().len(), 2);
    }

    #[test]
    fn analysis_queries_can_target_a_turn() {
        let mut session = Session::new();
        session.select(4, 4);
        session.confirm();
        session.select(3, 3);
        session.confirm();
        session.undo();

        assert_eq!(session.analysis_query_at(1, 100).moves.len(), 1);
        assert_eq!(session.analysis_query_at(2, 100).moves.len(), 2);
        assert_eq!(session.analysis_query_at(99, 100).moves.len(), 2);
    }

    #[test]
    fn arming_an_overlay_clears_the_pending_move() {
        let mut session = Session::new();
        session.select(4, 4);
        assert!(session.pending().is_some());

        session.toggle_pass();

        assert!(session.pass_armed());
        assert!(session.pending().is_none());
        assert!(!session.can_confirm());
    }

    #[test]
    fn arming_reset_clears_the_pending_move() {
        let mut session = Session::new();
        session.select(4, 4);
        assert!(session.pending().is_some());

        session.toggle_reset();

        assert!(session.reset_armed());
        assert!(session.pending().is_none());
        assert!(!session.can_confirm());
    }

    #[test]
    fn pass_locks_settings() {
        let mut session = Session::new();

        session.pass();

        assert!(session.settings_locked());
    }
}
