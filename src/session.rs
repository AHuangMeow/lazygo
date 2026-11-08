use crate::analysis::engine::AnalysisQuery;
use crate::game::{Game, Rules, DEFAULT_SIZE};

fn default_komi(rules: Rules) -> &'static str {
    match rules {
        Rules::Japanese => "6.5",
        Rules::Chinese | Rules::Ing => "7.5",
    }
}

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
    size_open: bool,
    rule_open: bool,
    reset_armed: bool,
    reset_rotation: i32,
    komi: String,
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
            size_open: false,
            rule_open: false,
            reset_armed: false,
            reset_rotation: 0,
            komi: default_komi(Rules::Chinese).to_string(),
        }
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    pub fn analysis_query(&self, max_visits: u32) -> AnalysisQuery {
        AnalysisQuery {
            size: self.game.size(),
            rules: self.game.rules(),
            komi: self
                .komi
                .trim()
                .parse()
                .unwrap_or_else(|_| default_komi(self.game.rules()).parse().unwrap()),
            moves: self.game.moves().to_vec(),
            max_visits,
        }
    }

    pub fn pending(&self) -> Option<PendingMove> {
        self.pending
    }

    pub fn size_open(&self) -> bool {
        self.size_open
    }

    pub fn rule_open(&self) -> bool {
        self.rule_open
    }

    pub fn reset_armed(&self) -> bool {
        self.reset_armed
    }

    pub fn reset_rotation(&self) -> i32 {
        self.reset_rotation
    }

    pub fn komi(&self) -> &str {
        &self.komi
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

    pub fn select(&mut self, x: usize, y: usize) {
        if x >= self.game.size() || y >= self.game.size() {
            return;
        }

        if self
            .pending
            .is_some_and(|pending| pending.x == x && pending.y == y)
        {
            self.pending = None;
        } else if self.game.stone_at(x, y).is_none() {
            let legal = self.game.play(x, y).is_some();
            self.pending = Some(PendingMove { x, y, legal });
        }
    }

    pub fn confirm(&mut self) {
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
        if self.pass_armed {
            self.pass_armed = false;
            self.pass();
        } else {
            self.pass_armed = true;
            self.size_open = false;
            self.rule_open = false;
            self.reset_armed = false;
        }
    }

    pub fn cancel_pass(&mut self) {
        self.pass_armed = false;
    }

    pub fn pass(&mut self) {
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

    pub fn toggle_reset(&mut self) {
        self.reset_rotation += 360;
        self.size_open = false;
        self.rule_open = false;
        self.pass_armed = false;

        if self.reset_armed {
            self.game = Game::with_rules(self.game.size(), self.game.rules());
            self.started = false;
            self.pending = None;
            self.reset_armed = false;
        } else {
            self.reset_armed = true;
        }
    }

    pub fn cancel_reset(&mut self) {
        self.reset_armed = false;
        self.reset_rotation -= 360;
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
            self.komi = default_komi(rules).to_string();
        }

        self.dismiss_overlays();
    }

    pub fn toggle_size_menu(&mut self) {
        if self.settings_locked() {
            return;
        }

        self.rule_open = false;
        self.pass_armed = false;
        self.size_open = !self.size_open;
    }

    pub fn toggle_rule_menu(&mut self) {
        if self.settings_locked() {
            return;
        }

        self.size_open = false;
        self.pass_armed = false;
        self.rule_open = !self.rule_open;
    }

    pub fn set_komi(&mut self, komi: String) {
        if self.settings_locked() {
            return;
        }

        self.komi = komi;
    }

    fn dismiss_overlays(&mut self) {
        self.size_open = false;
        self.rule_open = false;
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
        assert_eq!(session.komi(), "6.5");
    }

    #[test]
    fn settings_cannot_change_after_the_first_move() {
        let mut session = Session::new();

        session.select(4, 4);
        session.confirm();
        session.set_rules(Rules::Ing);
        session.set_size(19);
        session.set_komi("7.5".to_string());

        assert_eq!(session.game().rules(), Rules::Chinese);
        assert_eq!(session.game().size(), 9);
        assert_eq!(session.komi(), "7.5");
    }

    #[test]
    fn starting_a_new_rule_selection_clears_pending() {
        let mut session = Session::new();

        session.select(4, 4);
        session.set_rules(Rules::Ing);

        assert!(session.pending().is_none());
        assert!(session.game().is_initial());
        assert_eq!(session.game().rules(), Rules::Ing);
        assert_eq!(session.komi(), "7.5");
    }

    #[test]
    fn rules_set_their_default_komi() {
        let mut session = Session::new();

        assert_eq!(session.komi(), "7.5");

        session.set_rules(Rules::Japanese);
        assert_eq!(session.komi(), "6.5");

        session.set_rules(Rules::Ing);
        assert_eq!(session.komi(), "7.5");

        session.set_rules(Rules::Chinese);
        assert_eq!(session.komi(), "7.5");
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
    fn pass_locks_settings() {
        let mut session = Session::new();

        session.pass();

        assert!(session.settings_locked());
    }
}
