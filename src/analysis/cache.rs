use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Receiver;

use crate::analysis::engine::{AnalysisEvent, AnalysisQuery, AnalysisService, AnalysisSnapshot};
use crate::game::{Move, Rules};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct AnalysisKey {
    pub size: usize,
    pub rules: Rules,
    pub komi_bits: u64,
    pub moves: Vec<Move>,
    pub max_visits: u32,
    pub model_id: String,
}

impl AnalysisKey {
    pub fn new(query: &AnalysisQuery, model_id: &str) -> Self {
        Self {
            size: query.size,
            rules: query.rules,
            komi_bits: query.komi.to_bits(),
            moves: query.moves.clone(),
            max_visits: query.max_visits,
            model_id: model_id.to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AnalysisUpdate {
    Progress(AnalysisKey, AnalysisSnapshot),
    Finished(AnalysisKey, AnalysisSnapshot),
    Error(AnalysisKey, String),
}

#[derive(Default)]
pub struct AnalysisCache {
    entries: HashMap<AnalysisKey, AnalysisSnapshot>,
}

impl AnalysisCache {
    pub fn get(&self, key: &AnalysisKey) -> Option<&AnalysisSnapshot> {
        self.entries.get(key)
    }

    pub fn insert(&mut self, key: AnalysisKey, snapshot: AnalysisSnapshot) {
        self.entries.insert(key, snapshot);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Default)]
pub struct AnalysisController {
    service: Option<AnalysisService>,
    cache: AnalysisCache,
    pending: HashMap<AnalysisKey, Receiver<AnalysisEvent>>,
    model_id: Option<String>,
}

impl AnalysisController {
    pub fn configure(&mut self) -> Result<(), String> {
        let engine = crate::analysis::native_engine_path().ok_or("KataGo 引擎不可用")?;
        let config = crate::analysis::analysis_config_path().ok_or("无法创建分析配置")?;
        let model = crate::analysis::active_model().ok_or("尚未导入模型")?;

        if self.model_id.as_deref() == Some(model.id.as_str()) && self.service.is_some() {
            return Ok(());
        }

        let service = AnalysisService::spawn(engine, config, model.path.clone())?;
        self.service = Some(service);
        self.model_id = Some(model.id);
        self.cache.clear();
        self.pending.clear();
        Ok(())
    }

    pub fn is_ready(&self) -> bool {
        self.service.is_some() && self.model_id.is_some()
    }

    pub fn model_id(&self) -> Option<&str> {
        self.model_id.as_deref()
    }

    pub fn cache(&self) -> &AnalysisCache {
        &self.cache
    }

    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn cached(&self, query: &AnalysisQuery) -> Option<AnalysisSnapshot> {
        let model_id = self.model_id.as_deref()?;
        let key = AnalysisKey::new(query, model_id);
        self.cache.get(&key).cloned()
    }

    pub fn ensure(&mut self, query: AnalysisQuery) -> Result<Option<AnalysisSnapshot>, String> {
        let model_id = self.model_id.as_deref().ok_or("尚未导入模型")?;
        let key = AnalysisKey::new(&query, model_id);

        if let Some(snapshot) = self.cache.get(&key) {
            return Ok(Some(snapshot.clone()));
        }
        if self.pending.contains_key(&key) {
            return Ok(None);
        }

        let service = self.service.as_ref().ok_or("分析引擎尚未启动")?;
        let receiver = service.analyze(query)?;
        self.pending.insert(key, receiver);
        Ok(None)
    }

    pub fn poll(&mut self) -> Vec<AnalysisUpdate> {
        let mut updates = Vec::new();
        let mut finished = Vec::new();
        let mut failed = HashSet::new();

        for (key, receiver) in &self.pending {
            loop {
                match receiver.try_recv() {
                    Ok(AnalysisEvent::Update(snapshot)) => {
                        updates.push(AnalysisUpdate::Progress(key.clone(), snapshot));
                    }
                    Ok(AnalysisEvent::Finished(snapshot)) => {
                        updates.push(AnalysisUpdate::Finished(key.clone(), snapshot.clone()));
                        finished.push((key.clone(), snapshot));
                        break;
                    }
                    Ok(AnalysisEvent::Error(error)) => {
                        updates.push(AnalysisUpdate::Error(key.clone(), error));
                        failed.insert(key.clone());
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        updates.push(AnalysisUpdate::Error(
                            key.clone(),
                            "分析进程意外结束".into(),
                        ));
                        failed.insert(key.clone());
                        break;
                    }
                }
            }
        }

        for (key, snapshot) in finished {
            self.cache.insert(key.clone(), snapshot);
            self.pending.remove(&key);
        }
        for key in failed {
            self.pending.remove(&key);
        }

        updates
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query() -> AnalysisQuery {
        AnalysisQuery {
            size: 9,
            rules: Rules::Chinese,
            komi: 7.5,
            moves: vec![Move::Play(4, 4)],
            max_visits: 100,
        }
    }

    #[test]
    fn key_changes_with_depth_and_model() {
        let query = query();
        let a = AnalysisKey::new(&query, "m1");
        let b = AnalysisKey::new(&query, "m2");
        let mut deeper = query.clone();
        deeper.max_visits = 200;
        let c = AnalysisKey::new(&deeper, "m1");

        assert_ne!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn cache_round_trips() {
        let query = query();
        let key = AnalysisKey::new(&query, "m1");
        let snapshot = AnalysisSnapshot {
            black_winrate: 0.5,
            black_score_lead: 0.0,
            ownership: vec![0.0; 81],
            visits: 100,
            current_player: crate::game::Stone::White,
            best_move: Some((4, 5)),
            candidates: Vec::new(),
        };
        let mut cache = AnalysisCache::default();
        cache.insert(key.clone(), snapshot.clone());
        assert_eq!(cache.get(&key), Some(&snapshot));
    }
}
