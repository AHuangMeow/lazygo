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
        };
        state.reload();
        state
    }

    pub fn reload(&mut self) {
        self.current = None;
        self.current_key = None;
        self.error = None;
        self.enabled = false;

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

    pub fn current(&self) -> Option<&AnalysisSnapshot> {
        self.current.as_ref()
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

        if let Err(error) = save_settings(&AnalysisSettings { max_visits }) {
            self.error = Some(error);
        }
    }

    pub fn needs_tick(&self, session: &Session) -> bool {
        if !self.enabled {
            return false;
        }
        let Some(model_id) = self.controller.model_id() else {
            return false;
        };
        let query = session.analysis_query(self.max_visits);
        let key = AnalysisKey::new(&query, model_id);

        if self.current_key.as_ref() != Some(&key) {
            return true;
        }
        if self.controller.has_pending() {
            return true;
        }
        if self.current.is_none() && self.controller.cached(&query).is_some() {
            return true;
        }
        if self.panel_open && self.controller.pending_count() < 2 {
            let total = session.game().all_moves().len();
            if self.controller.cache().len() > total {
                return false;
            }
            return (0..=total).any(|index| {
                let query = session.analysis_query_at(index, self.max_visits);
                self.controller.cached(&query).is_none()
            });
        }
        false
    }

    pub fn tick(&mut self, session: &Session) {
        if !self.enabled {
            return;
        }
        let Some(model_id) = self.controller.model_id() else {
            return;
        };
        let query = session.analysis_query(self.max_visits);
        let key = AnalysisKey::new(&query, model_id);

        if self.current_key.as_ref() != Some(&key) {
            self.current_key = Some(key.clone());
            self.current = None;
            match self.controller.ensure(query.clone()) {
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
            if let Some(snapshot) = self.controller.cached(&query) {
                self.current = Some(snapshot);
            }
        }

        if self.panel_open && self.controller.pending_count() < 2 {
            let total = session.game().all_moves().len();
            if self.controller.cache().len() < total + 1 {
                for index in 0..=total {
                    let query = session.analysis_query_at(index, self.max_visits);
                    if self.controller.cached(&query).is_none() {
                        let _ = self.controller.ensure(query);
                        break;
                    }
                }
            }
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
                let query = session.analysis_query_at(index, self.max_visits);
                let key = AnalysisKey::new(&query, model_id);
                self.controller.cache().get(&key).cloned()
            })
            .collect()
    }
}
