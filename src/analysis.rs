pub mod cache;
pub mod engine;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const MODEL_DIR: &str = "models";
const ACTIVE_MODEL_FILE: &str = "active";
const SETTINGS_FILE: &str = "settings.json";
const MODEL_MAGIC: [u8; 2] = [0x1f, 0x8b];
const DISABLED_MODEL: &str = "__disabled__";

pub fn native_engine_path() -> Option<PathBuf> {
    let maps = fs::read_to_string("/proc/self/maps").ok()?;
    engine_path_from_maps(&maps)
}

fn engine_path_from_maps(maps: &str) -> Option<PathBuf> {
    let libmain = maps.lines().find_map(|line| {
        let path = line.split_whitespace().last()?;
        path.ends_with("/libmain.so").then_some(path)
    })?;

    Some(PathBuf::from(libmain).parent()?.join("libkatago.so"))
}

pub fn app_files_dir() -> Option<PathBuf> {
    let cmdline = fs::read("/proc/self/cmdline").ok()?;
    let package = cmdline.split(|byte| *byte == 0).next()?;
    let package = String::from_utf8_lossy(package);
    let files = PathBuf::from(format!("/data/user/0/{package}/files"));
    fs::create_dir_all(&files).ok()?;
    Some(files)
}

fn models_dir() -> Option<PathBuf> {
    let dir = app_files_dir()?.join(MODEL_DIR);
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
}

impl ModelInfo {
    fn from_path(path: PathBuf) -> Option<Self> {
        let name = path.file_name()?.to_string_lossy().into_owned();
        let size = fs::metadata(&path).ok()?.len();
        Some(Self {
            id: name.clone(),
            name,
            path,
            size,
        })
    }
}

pub fn models() -> Vec<ModelInfo> {
    let Some(dir) = models_dir() else {
        return Vec::new();
    };

    let mut models: Vec<_> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() || !is_model_name(&path) {
                return None;
            }
            ModelInfo::from_path(path)
        })
        .collect();

    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn active_model() -> Option<ModelInfo> {
    let models = models();
    if models.is_empty() {
        return None;
    }

    let active = app_files_dir()
        .and_then(|dir| fs::read_to_string(dir.join(MODEL_DIR).join(ACTIVE_MODEL_FILE)).ok());
    let active = active.as_deref().map(str::trim).filter(|id| !id.is_empty());

    if active.is_some_and(is_disabled_model) {
        return None;
    }

    active
        .and_then(|id| models.iter().find(|model| model.id == id).cloned())
        .or_else(|| models.first().cloned())
}

pub fn model_path() -> Option<PathBuf> {
    active_model().map(|model| model.path)
}

pub fn import_model(name: &str, bytes: &[u8]) -> Result<ModelInfo, String> {
    if bytes.len() < 1024 {
        return Err("模型文件太小，可能不是有效的 KataGo 模型".into());
    }
    if bytes.len() > 512 * 1024 * 1024 {
        return Err("模型文件过大".into());
    }
    if !bytes.starts_with(&MODEL_MAGIC) {
        return Err("模型必须是 gzip 压缩的 KataGo .bin.gz 文件".into());
    }

    let dir = models_dir().ok_or_else(|| "无法访问应用存储目录".to_string())?;
    let name = unique_model_name(&dir, &sanitize_model_name(name));
    let path = dir.join(&name);

    let partial = dir.join(format!(".{name}.part"));
    fs::write(&partial, bytes).map_err(|error| format!("写入模型失败: {error}"))?;
    fs::rename(&partial, &path).map_err(|error| format!("保存模型失败: {error}"))?;
    set_active_model(&name)?;

    ModelInfo::from_path(path).ok_or_else(|| "模型保存后无法读取".to_string())
}

pub fn disable_model() -> Result<(), String> {
    let dir = models_dir().ok_or_else(|| "无法访问应用存储目录".to_string())?;
    fs::write(dir.join(ACTIVE_MODEL_FILE), DISABLED_MODEL)
        .map_err(|error| format!("保存模型选择失败: {error}"))
}

fn is_disabled_model(active: &str) -> bool {
    active == DISABLED_MODEL
}

pub fn select_model(id: &str) -> Result<(), String> {
    if !models().iter().any(|model| model.id == id) {
        return Err("模型不存在".into());
    }
    set_active_model(id)
}

pub fn remove_model(id: &str) -> Result<(), String> {
    let dir = models_dir().ok_or_else(|| "无法访问应用存储目录".to_string())?;
    let path = dir.join(id);
    let was_active = fs::read_to_string(dir.join(ACTIVE_MODEL_FILE))
        .map(|active| active.trim() == id)
        .unwrap_or(false);
    if path.exists() {
        fs::remove_file(&path).map_err(|error| format!("删除模型失败: {error}"))?;
    }

    if was_active {
        let _ = fs::remove_file(dir.join(ACTIVE_MODEL_FILE));
    }

    Ok(())
}

fn set_active_model(id: &str) -> Result<(), String> {
    let dir = models_dir().ok_or_else(|| "无法访问应用存储目录".to_string())?;
    fs::write(dir.join(ACTIVE_MODEL_FILE), id).map_err(|error| format!("保存模型选择失败: {error}"))
}

fn is_model_name(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".bin.gz"))
}

fn sanitize_model_name(name: &str) -> String {
    let file_name = Path::new(name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("model.bin.gz");
    let mut sanitized: String = file_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect();

    if !sanitized.ends_with(".bin.gz") {
        sanitized.push_str(".bin.gz");
    }
    if sanitized.trim_matches('.').is_empty() {
        sanitized = "model.bin.gz".into();
    }

    sanitized
}

fn unique_model_name(dir: &Path, name: &str) -> String {
    if !dir.join(name).exists() {
        return name.to_string();
    }

    let stem = name.strip_suffix(".bin.gz").unwrap_or(name);
    for index in 1.. {
        let candidate = format!("{stem}-{index}.bin.gz");
        if !dir.join(&candidate).exists() {
            return candidate;
        }
    }

    unreachable!()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisSettings {
    pub max_visits: u32,
}

impl Default for AnalysisSettings {
    fn default() -> Self {
        Self { max_visits: 200 }
    }
}

pub fn load_settings() -> AnalysisSettings {
    app_files_dir()
        .and_then(|dir| fs::read_to_string(dir.join(SETTINGS_FILE)).ok())
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .map(|settings: AnalysisSettings| AnalysisSettings {
            max_visits: settings.max_visits.clamp(1, 5000),
        })
        .unwrap_or_default()
}

pub fn save_settings(settings: &AnalysisSettings) -> Result<(), String> {
    let dir = app_files_dir().ok_or_else(|| "无法访问应用存储目录".to_string())?;
    let contents = serde_json::to_string_pretty(settings)
        .map_err(|error| format!("序列化设置失败: {error}"))?;
    fs::write(dir.join(SETTINGS_FILE), contents).map_err(|error| format!("保存设置失败: {error}"))
}

pub fn analysis_config_path() -> Option<PathBuf> {
    let files = app_files_dir()?;
    let logs = files.join("logs");
    fs::create_dir_all(&logs).ok()?;
    let config = files.join("analysis.cfg");
    let contents = format!(
        "logDir = {}\nnumAnalysisThreads = 1\nnumSearchThreadsPerAnalysisThread = 2\nmaxVisits = 200\nnnCacheSizePowerOfTwo = 16\nnnMutexPoolSizePowerOfTwo = 16\nnumEigenThreadsPerModel = 2\n",
        logs.display()
    );

    if fs::read_to_string(&config).ok().as_deref() != Some(&contents) {
        fs::write(&config, contents).ok()?;
    }

    Some(config)
}

pub fn analysis_enabled() -> bool {
    native_engine_path().is_some() && active_model().is_some() && analysis_config_path().is_some()
}

pub fn validate_model(path: &Path) -> Result<(), String> {
    let path = path.to_path_buf();
    std::thread::spawn(move || validate_model_inner(&path))
        .join()
        .map_err(|_| "模型校验线程异常退出".to_string())?
}

fn validate_model_inner(path: &Path) -> Result<(), String> {
    let engine = native_engine_path().ok_or("KataGo 引擎不可用")?;
    let config = analysis_config_path().ok_or("无法创建分析配置")?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("无法启动校验运行时: {error}"))?;

    runtime.block_on(async {
        let options = katago_analysis::engine::LaunchOptions::new(
            engine.to_string_lossy().into_owned(),
            config.to_string_lossy().into_owned(),
            path.to_string_lossy().into_owned(),
        );
        let mut analyzer: katago_analysis::Analyzer =
            katago_analysis::engine::Engine::launch(&options)
                .map_err(|error| format!("模型加载失败: {error}"))?
                .into();
        let request = katago_analysis::AnalysisRequest::new(
            katago_analysis::Rules::chinese_ogs(),
            9,
            9,
            Vec::new(),
        )
        .with_komi(7.5)
        .with_max_visits(1);

        match analyzer.analyze(request).await {
            Ok(Some(_)) => Ok(()),
            Ok(None) => Err("模型没有返回分析结果".into()),
            Err(error) => Err(format!("模型校验失败: {error}")),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn finds_engine_next_to_libmain() {
        let maps = "7f00-7f01 r-xp 00000000 00:00 0 /data/app/pkg/lib/x86_64/libmain.so\n";
        assert_eq!(
            engine_path_from_maps(maps),
            Some(PathBuf::from("/data/app/pkg/lib/x86_64/libkatago.so"))
        );
    }

    #[test]
    fn ignores_maps_without_libmain() {
        assert_eq!(engine_path_from_maps("no lib here"), None);
    }

    #[test]
    fn sanitizes_model_names() {
        assert_eq!(sanitize_model_name("b6c96.bin.gz"), "b6c96.bin.gz");
        assert_eq!(sanitize_model_name("../my model"), "my_model.bin.gz");
        assert_eq!(sanitize_model_name("model"), "model.bin.gz");
    }

    #[test]
    fn default_analysis_depth_is_200() {
        assert_eq!(AnalysisSettings::default().max_visits, 200);
    }

    #[test]
    fn recognizes_disabled_model_sentinel() {
        assert!(is_disabled_model(DISABLED_MODEL));
        assert!(!is_disabled_model("b6c96.bin.gz"));
    }

    #[test]
    fn recognizes_model_names() {
        assert!(is_model_name(Path::new("m.bin.gz")));
        assert!(!is_model_name(Path::new("active")));
    }
}
