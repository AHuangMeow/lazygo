use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

use katago_analysis::engine::{Engine, LaunchOptions};
use katago_analysis::{
    AnalysisRequest, AnalysisResult, Analyzer, Coord, Move as KataMove, Player, Rules as KataRules,
};

use crate::game::{Move, Rules, Stone};

#[derive(Clone, Debug, PartialEq)]
pub struct AnalysisQuery {
    pub size: usize,
    pub rules: Rules,
    pub komi: f64,
    pub moves: Vec<Move>,
    pub max_visits: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnalysisSnapshot {
    pub black_winrate: f32,
    pub black_score_lead: f32,
    pub ownership: Vec<f32>,
    pub visits: u32,
    pub current_player: Stone,
    pub best_move: Option<(usize, usize)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AnalysisEvent {
    Update(AnalysisSnapshot),
    Finished(AnalysisSnapshot),
    Error(String),
}

#[derive(Clone)]
pub struct AnalysisService {
    jobs: tokio::sync::mpsc::Sender<Job>,
}

struct Job {
    query: AnalysisQuery,
    events: Sender<AnalysisEvent>,
}

impl AnalysisService {
    pub fn spawn(engine: PathBuf, config: PathBuf, model: PathBuf) -> Result<Self, String> {
        let (jobs, receiver) = tokio::sync::mpsc::channel(4);

        std::thread::Builder::new()
            .name("katago-analysis".into())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        drain_with_error(receiver, error.to_string());
                        return;
                    }
                };

                runtime.block_on(run(receiver, engine, config, model));
            })
            .map_err(|error| format!("无法启动分析线程: {error}"))?;

        Ok(Self { jobs })
    }

    pub fn analyze(&self, query: AnalysisQuery) -> Result<Receiver<AnalysisEvent>, String> {
        let (events, receiver) = mpsc::channel();
        self.jobs
            .try_send(Job { query, events })
            .map_err(|error| match error {
                tokio::sync::mpsc::error::TrySendError::Full(_) => {
                    "分析队列已满，请稍后再试".to_string()
                }
                tokio::sync::mpsc::error::TrySendError::Closed(_) => "分析引擎已停止".to_string(),
            })?;
        Ok(receiver)
    }
}

async fn run(
    mut jobs: tokio::sync::mpsc::Receiver<Job>,
    engine: PathBuf,
    config: PathBuf,
    model: PathBuf,
) {
    let options = LaunchOptions::new(
        engine.to_string_lossy().into_owned(),
        config.to_string_lossy().into_owned(),
        model.to_string_lossy().into_owned(),
    );

    let mut analyzer: Option<Analyzer> = match Engine::launch(&options) {
        Ok(engine) => Some(engine.into()),
        Err(error) => {
            drain_with_error(jobs, format!("无法启动 KataGo: {error}"));
            return;
        }
    };

    while let Some(job) = jobs.recv().await {
        let Some(analyzer) = analyzer.as_mut() else {
            let _ = job
                .events
                .send(AnalysisEvent::Error("分析引擎不可用".into()));
            continue;
        };

        if let Err(error) = analyze_job(analyzer, job.query, &job.events).await {
            let _ = job.events.send(AnalysisEvent::Error(error));
        }
    }
}

async fn analyze_job(
    analyzer: &mut Analyzer,
    query: AnalysisQuery,
    events: &Sender<AnalysisEvent>,
) -> Result<(), String> {
    let request = build_request(&query);
    let mut progress = analyzer
        .start_analyze(request)
        .await
        .map_err(|error| error.to_string())?;

    loop {
        match progress.poll().await {
            ControlFlow::Continue(next) => {
                progress = next;
                let result = progress.read().await;
                if let Ok(Some(result)) = result.as_ref() {
                    let _ = events.send(AnalysisEvent::Update(snapshot(result)));
                }
            }
            ControlFlow::Break(result) => {
                return match result {
                    Ok(Some(result)) => {
                        let _ = events.send(AnalysisEvent::Finished(snapshot(&result)));
                        Ok(())
                    }
                    Ok(None) => Err("分析未返回结果".into()),
                    Err(error) => Err(error.to_string()),
                };
            }
        }
    }
}

fn drain_with_error(mut jobs: tokio::sync::mpsc::Receiver<Job>, error: String) {
    while let Ok(job) = jobs.try_recv() {
        let _ = job.events.send(AnalysisEvent::Error(error.clone()));
    }
}

fn build_request(query: &AnalysisQuery) -> AnalysisRequest {
    let moves = query
        .moves
        .iter()
        .enumerate()
        .map(|(index, mv)| {
            let player = if index % 2 == 0 {
                Player::Black
            } else {
                Player::White
            };
            let mv = match *mv {
                Move::Play(x, y) => KataMove::Move(Coord(x as u8, y as u8)),
                Move::Pass => KataMove::Pass,
            };
            (player, mv)
        })
        .collect();

    AnalysisRequest::new(
        kata_rules(query.rules),
        query.size as u8,
        query.size as u8,
        moves,
    )
    .with_komi(query.komi)
    .with_max_visits(query.max_visits.clamp(1, 5000))
    .with_ownership()
    .with_report_during_search_every(0.25)
}

fn kata_rules(rules: Rules) -> KataRules {
    match rules {
        Rules::Chinese => KataRules::chinese_ogs(),
        Rules::Japanese => KataRules::japanese(),
        Rules::Ing => KataRules::ing(),
    }
}

fn snapshot(result: &AnalysisResult) -> AnalysisSnapshot {
    let current_player = match result.root_info.current_player {
        Player::Black => Stone::Black,
        Player::White => Stone::White,
    };
    let (black_winrate, black_score_lead) = black_perspective(
        current_player,
        result.root_info.winrate,
        result.root_info.score_lead,
    );

    let ownership = result
        .ownership
        .as_ref()
        .map(|matrix| matrix.raw.iter().map(|value| *value as f32).collect())
        .unwrap_or_default();
    let ownership = black_perspective_ownership(current_player, ownership);

    let best_move = result.move_infos.first().and_then(|info| match info.mv {
        KataMove::Move(Coord(x, y)) => Some((x as usize, y as usize)),
        KataMove::Pass => None,
    });

    AnalysisSnapshot {
        black_winrate,
        black_score_lead,
        ownership,
        visits: result.root_info.visits,
        current_player,
        best_move,
    }
}

fn black_perspective(current_player: Stone, winrate: f64, score_lead: f64) -> (f32, f32) {
    match current_player {
        Stone::Black => (winrate as f32, score_lead as f32),
        Stone::White => ((1.0 - winrate) as f32, -score_lead as f32),
    }
}

fn black_perspective_ownership(current_player: Stone, mut ownership: Vec<f32>) -> Vec<f32> {
    if current_player == Stone::White {
        for value in &mut ownership {
            *value = -*value;
        }
    }
    ownership
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_our_rules_to_katago_rules() {
        assert_eq!(kata_rules(Rules::Chinese), KataRules::chinese_ogs());
        assert_eq!(kata_rules(Rules::Japanese), KataRules::japanese());
        assert_eq!(kata_rules(Rules::Ing), KataRules::ing());
    }

    #[test]
    fn converts_to_black_perspective() {
        assert_eq!(black_perspective(Stone::Black, 0.6, 3.0), (0.6, 3.0));
        let (winrate, score) = black_perspective(Stone::White, 0.6, 3.0);
        assert!((winrate - 0.4).abs() < f32::EPSILON);
        assert!((score + 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn normalizes_ownership_to_black_perspective() {
        assert_eq!(
            black_perspective_ownership(Stone::Black, vec![0.5, -0.25]),
            vec![0.5, -0.25]
        );
        assert_eq!(
            black_perspective_ownership(Stone::White, vec![0.5, -0.25]),
            vec![-0.5, 0.25]
        );
    }

    #[test]
    fn builds_moves_with_alternating_players() {
        let query = AnalysisQuery {
            size: 9,
            rules: Rules::Chinese,
            komi: 7.5,
            moves: vec![Move::Play(4, 4), Move::Pass],
            max_visits: 10,
        };
        let request = build_request(&query);
        assert_eq!(request.board_x_size, 9);
        assert_eq!(request.komi, Some(7.5));
        assert_eq!(request.max_visits, Some(10));
        assert!(request.include_ownership);
    }
}
