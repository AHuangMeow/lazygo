use lazygo::analysis::cache::{AnalysisController, AnalysisKey, AnalysisUpdate};
use lazygo::analysis::engine::AnalysisSnapshot;
use lazygo::analysis::{active_model, load_settings, save_settings, AnalysisSettings};
use lazygo::game::Stone;
use lazygo::session::Session;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnalysisMode {
    Winrate,
    Score,
}

#[derive(Default, Debug, PartialEq, Eq)]
struct BackfillCursor {
    cursor: usize,
    limit: Option<usize>,
}

impl BackfillCursor {
    fn reset(&mut self) {
        self.cursor = 0;
        self.limit = None;
    }

    fn sync(&mut self, limit: Option<usize>) {
        if self.limit != limit {
            self.limit = limit;
            self.cursor = 0;
        }
    }

    fn index(&self) -> Option<usize> {
        let limit = self.limit?;
        (self.cursor <= limit).then_some(self.cursor)
    }

    fn advance(&mut self) {
        self.cursor += 1;
    }

    fn complete(&self) -> bool {
        self.limit.is_some_and(|limit| self.cursor > limit)
    }
}

pub struct AnalysisUiState {
    controller: AnalysisController,
    enabled: bool,
    error: Option<String>,
    current: Option<AnalysisSnapshot>,
    current_key: Option<AnalysisKey>,
    panel_open: bool,
    perspective: Stone,
    mode: AnalysisMode,
    max_visits: u32,
    candidate_count: u32,
    show_candidates: bool,
    backfill_target: Option<usize>,
    backfill: BackfillCursor,
}

impl Default for AnalysisUiState {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisUiState {
    pub fn new() -> Self {
        let settings = load_settings();
        let mut state = Self {
            controller: AnalysisController::default(),
            enabled: false,
            error: None,
            current: None,
            current_key: None,
            panel_open: false,
            perspective: Stone::Black,
            mode: AnalysisMode::Winrate,
            max_visits: settings.max_visits,
            candidate_count: settings.candidate_count,
            show_candidates: false,
            backfill_target: None,
            backfill: BackfillCursor::default(),
        };
        state.reload();
        state
    }

    pub fn reload(&mut self) {
        self.current = None;
        self.current_key = None;
        self.error = None;
        self.enabled = false;
        self.backfill.reset();

        if active_model().is_none() {
            self.panel_open = false;
            return;
        }

        match self.controller.configure() {
            Ok(()) => self.enabled = true,
            Err(error) => {
                self.panel_open = false;
                self.error = Some(error);
            }
        }
    }

    pub fn start_new_game(&mut self, total_moves: usize) {
        self.current = None;
        self.current_key = None;
        self.error = None;
        self.controller.clear_cache();
        self.backfill_target = Some(total_moves);
        self.backfill.reset();
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn panel_open(&self) -> bool {
        self.panel_open
    }

    pub fn perspective(&self) -> Stone {
        self.perspective
    }

    pub fn mode(&self) -> AnalysisMode {
        self.mode
    }

    pub fn max_visits(&self) -> u32 {
        self.max_visits
    }

    pub fn candidate_count(&self) -> u32 {
        self.candidate_count
    }

    pub fn show_candidates(&self) -> bool {
        self.show_candidates
    }

    pub fn toggle_candidates(&mut self) {
        if self.enabled {
            self.show_candidates = !self.show_candidates;
        }
    }

    pub fn set_candidate_count(&mut self, candidate_count: u32) {
        let candidate_count = candidate_count.clamp(1, 10);
        if candidate_count == self.candidate_count {
            return;
        }
        self.candidate_count = candidate_count;
        if let Err(error) = save_settings(&AnalysisSettings {
            max_visits: self.max_visits,
            candidate_count,
        }) {
            self.error = Some(error);
        }
    }

    pub fn current(&self) -> Option<&AnalysisSnapshot> {
        self.current.as_ref()
    }

    fn current_key(&self, session: &Session, model_id: &str) -> AnalysisKey {
        AnalysisKey::from_parts(
            session.game().node_id(),
            session.komi().value(),
            self.max_visits,
            model_id,
        )
    }

    fn key_at(&self, session: &Session, turn: usize, model_id: &str) -> Option<AnalysisKey> {
        let node_id = session.game().node_id_at(turn)?;
        Some(AnalysisKey::from_parts(
            node_id,
            session.komi().value(),
            self.max_visits,
            model_id,
        ))
    }

    pub fn current_matches(&self, session: &Session) -> bool {
        let Some(model_id) = self.controller.model_id() else {
            return false;
        };
        self.current_key.as_ref() == Some(&self.current_key(session, model_id))
    }

    pub fn toggle_panel(&mut self) {
        if self.enabled {
            self.panel_open = !self.panel_open;
        }
    }

    pub fn toggle_perspective(&mut self) {
        self.perspective = self.perspective.other();
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            AnalysisMode::Winrate => AnalysisMode::Score,
            AnalysisMode::Score => AnalysisMode::Winrate,
        };
    }

    pub fn set_max_visits(&mut self, max_visits: u32) {
        let max_visits = max_visits.clamp(1, 5000);
        if max_visits == self.max_visits {
            return;
        }

        self.max_visits = max_visits;
        self.current = None;
        self.current_key = None;
        self.error = None;
        self.controller.clear_cache();
        self.backfill.reset();

        if let Err(error) = save_settings(&AnalysisSettings {
            max_visits,
            candidate_count: self.candidate_count,
        }) {
            self.error = Some(error);
        }
    }

    fn backfill_limit(&self, session: &Session) -> Option<usize> {
        let total = session.game().all_moves().len();
        if self.panel_open {
            Some(total)
        } else {
            self.backfill_target.map(|target| target.min(total))
        }
    }

    pub fn needs_tick(&self, session: &Session) -> bool {
        if !self.enabled {
            return false;
        }
        let Some(model_id) = self.controller.model_id() else {
            return false;
        };
        let key = self.current_key(session, model_id);

        if self.current_key.as_ref() != Some(&key) {
            return true;
        }
        if self.controller.has_pending() {
            return true;
        }
        if self.current.is_none() && self.controller.cache().get(&key).is_some() {
            return true;
        }
        if self.controller.pending_count() >= 2 {
            return false;
        }
        let Some(limit) = self.backfill_limit(session) else {
            return false;
        };
        self.backfill.limit != Some(limit) || self.backfill.cursor <= limit
    }

    pub fn tick(&mut self, session: &Session) {
        if !self.enabled {
            return;
        }
        let Some(model_id) = self.controller.model_id().map(str::to_owned) else {
            return;
        };
        let key = self.current_key(session, &model_id);

        if self.current_key.as_ref() != Some(&key) {
            let query = session.analysis_query(self.max_visits);
            self.current_key = Some(key.clone());
            self.current = None;
            match self.controller.ensure(query) {
                Ok(Some(snapshot)) => self.current = Some(snapshot),
                Ok(None) => {}
                Err(error) => self.error = Some(error),
            }
        }

        for update in self.controller.poll() {
            match update {
                AnalysisUpdate::Progress(key, snapshot)
                | AnalysisUpdate::Finished(key, snapshot)
                    if self.current_key.as_ref() == Some(&key) =>
                {
                    self.current = Some(snapshot);
                    self.error = None;
                }
                AnalysisUpdate::Error(key, error) if self.current_key.as_ref() == Some(&key) => {
                    self.error = Some(error);
                }
                _ => {}
            }
        }

        if self.current.is_none() {
            if let Some(snapshot) = self.controller.cache().get(&key).cloned() {
                self.current = Some(snapshot);
            }
        }

        let Some(limit) = self.backfill_limit(session) else {
            self.backfill.reset();
            return;
        };
        self.backfill.sync(Some(limit));
        if self.controller.pending_count() >= 2 {
            return;
        }

        while let Some(index) = self.backfill.index() {
            let Some(key) = self.key_at(session, index, &model_id) else {
                break;
            };
            if self.controller.cache().get(&key).is_some() {
                self.backfill.advance();
                continue;
            }
            if self.controller.is_pending(&key) {
                break;
            }

            let query = session.analysis_query_at(index, self.max_visits);
            match self.controller.ensure(query) {
                Ok(Some(snapshot)) => {
                    if self.current_key.as_ref() == Some(&key) {
                        self.current = Some(snapshot);
                    }
                    self.backfill.advance();
                }
                Ok(None) => break,
                Err(error) => {
                    self.error = Some(error);
                    break;
                }
            }
        }

        if self.backfill.complete() {
            self.backfill_target = None;
        }
    }

    pub fn chart(&self, session: &Session) -> Vec<Option<AnalysisSnapshot>> {
        if !self.enabled {
            return Vec::new();
        }
        let Some(model_id) = self.controller.model_id() else {
            return Vec::new();
        };

        let total = session.game().all_moves().len();
        (0..=total)
            .map(|index| {
                let key = self.key_at(session, index, model_id)?;
                self.controller.cache().get(&key).cloned()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::BackfillCursor;

    #[test]
    fn backfill_cursor_advances_through_the_limit() {
        let mut cursor = BackfillCursor::default();
        cursor.sync(Some(2));

        assert_eq!(cursor.index(), Some(0));
        cursor.advance();
        assert_eq!(cursor.index(), Some(1));
        cursor.advance();
        assert_eq!(cursor.index(), Some(2));
        assert!(!cursor.complete());

        cursor.advance();

        assert_eq!(cursor.index(), None);
        assert!(cursor.complete());
    }

    #[test]
    fn changing_the_limit_restarts_the_scan() {
        let mut cursor = BackfillCursor::default();
        cursor.sync(Some(5));
        cursor.advance();
        cursor.advance();

        cursor.sync(Some(5));
        assert_eq!(cursor.index(), Some(2));

        cursor.sync(Some(4));
        assert_eq!(cursor.index(), Some(0));

        cursor.sync(Some(6));
        assert_eq!(cursor.index(), Some(0));
    }

    #[test]
    fn reset_clears_the_limit_and_cursor() {
        let mut cursor = BackfillCursor::default();
        cursor.sync(Some(3));
        cursor.advance();
        cursor.reset();

        assert_eq!(cursor.index(), None);
        assert!(!cursor.complete());
    }
}
