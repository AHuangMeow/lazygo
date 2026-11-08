use base64::{engine::general_purpose::STANDARD, Engine as _};
use dioxus::prelude::*;
use lazygo::analysis::{
    active_model, import_model, models, remove_model, select_model, validate_model, ModelInfo,
};

const PICK_MODEL_SCRIPT: &str = r#"
const start = () => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".gz,.bin.gz,application/gzip,application/octet-stream";
    input.style.position = "fixed";
    input.style.left = "-9999px";
    input.style.top = "-9999px";
    document.body.appendChild(input);

    let settled = false;
    const done = (payload) => {
        if (settled) return;
        settled = true;
        dioxus.send(JSON.stringify(payload));
        input.remove();
    };

    input.onchange = () => {
        const file = input.files && input.files[0];
        if (!file) {
            done({ error: "未选择文件" });
            return;
        }
        const reader = new FileReader();
        reader.onload = () => {
            const bytes = new Uint8Array(reader.result);
            let binary = "";
            const chunk = 0x4000;
            for (let i = 0; i < bytes.length; i += chunk) {
                binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
            }
            done({ name: file.name, data: btoa(binary) });
        };
        reader.onerror = () => done({ error: "读取文件失败" });
        reader.readAsArrayBuffer(file);
    };
    input.oncancel = () => done({ error: "已取消" });
    input.click();

    setTimeout(() => {
        if (!settled) done({ error: "选择超时" });
    }, 120000);
};
start();
"#;

#[derive(serde::Deserialize)]
struct PickedModel {
    name: Option<String>,
    data: Option<String>,
    error: Option<String>,
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / 1024.0 / 1024.0)
    } else {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    }
}

#[component]
pub fn SettingsPage(
    max_visits: u32,
    analysis_enabled: bool,
    analysis_error: Option<String>,
    on_back: EventHandler<()>,
    on_changed: EventHandler<()>,
    on_depth: EventHandler<u32>,
) -> Element {
    let mut model_list = use_signal(models);
    let mut active = use_signal(|| active_model().map(|model| model.id));
    let mut status = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);
    let mut delete_armed = use_signal(|| None::<String>);
    let mut depth = use_signal(|| max_visits);
    let current_depth = depth();
    let active_id = active();
    let model_items: Vec<ModelInfo> = model_list();

    rsx! {
        section { class: "settings-page",
            div { class: "settings-heading",
                button {
                    class: "settings-back",
                    r#type: "button",
                    aria_label: "返回",
                    onclick: move |_| on_back.call(()),
                    svg {
                        class: "settings-back-icon",
                        view_box: "0 0 24 24",
                        path { d: "M15.41 7.41 14 6l-6 6 6 6 1.41-1.41L10.83 12z" }
                    }
                }
                div {
                    h1 { class: "settings-title", "设置" }
                    p { class: "settings-subtitle", "模型与分析" }
                }
            }

            div { class: "settings-card",
                div { class: "settings-card-heading",
                    h2 { "分析引擎" }
                    span {
                        class: if analysis_enabled { "status-pill enabled" } else { "status-pill" },
                        if analysis_enabled { "已启用" } else { "未启用" }
                    }
                }
                p { class: "settings-note",
                    if analysis_enabled {
                        "已加载模型，落子后会自动分析并缓存。"
                    } else {
                        "导入有效的 KataGo .bin.gz 模型后，分析功能自动启用。"
                    }
                }
                if let Some(error) = analysis_error {
                    p { class: "settings-error", "{error}" }
                }
            }

            div { class: "settings-card",
                div { class: "settings-card-heading",
                    h2 { "分析深度" }
                    span { class: "settings-value", "{current_depth} visits" }
                }
                p { class: "settings-note", "深度越高，结果越稳定，也越耗电、耗时。" }
                div { class: "depth-presets",
                    for preset in [50_u32, 100, 200, 500] {
                        button {
                            class: if current_depth == preset { "depth-preset selected" } else { "depth-preset" },
                            r#type: "button",
                            onclick: move |_| {
                                depth.set(preset);
                                on_depth.call(preset);
                            },
                            "{preset}"
                        }
                    }
                }
                div { class: "depth-input-row",
                    input {
                        class: "depth-input",
                        r#type: "number",
                        min: "1",
                        max: "5000",
                        step: "10",
                        value: "{current_depth}",
                        onchange: move |event| {
                            if let Ok(value) = event.value().parse::<u32>() {
                                let value = value.clamp(1, 5000);
                                depth.set(value);
                                on_depth.call(value);
                            }
                        },
                    }
                    span { "visits" }
                }
            }

            div { class: "settings-card",
                div { class: "settings-card-heading",
                    h2 { "模型" }
                    button {
                        class: "primary-button",
                        r#type: "button",
                        disabled: busy(),
                        onclick: move |_| {
                            if busy() {
                                return;
                            }
                            busy.set(true);
                            status.set(Some("正在打开文件选择器…".into()));
                            spawn(async move {
                                let mut eval = document::eval(PICK_MODEL_SCRIPT);
                                let message = match eval.recv::<String>().await {
                                    Ok(message) => message,
                                    Err(error) => {
                                        busy.set(false);
                                        status.set(Some(format!("文件选择失败: {error}")));
                                        return;
                                    }
                                };

                                let picked: PickedModel = match serde_json::from_str(&message) {
                                    Ok(picked) => picked,
                                    Err(_) => {
                                        busy.set(false);
                                        status.set(Some("无法解析所选文件".into()));
                                        return;
                                    }
                                };

                                if let Some(error) = picked.error {
                                    busy.set(false);
                                    status.set(Some(error));
                                    return;
                                }

                                let Some(data) = picked.data else {
                                    busy.set(false);
                                    status.set(Some("没有收到文件内容".into()));
                                    return;
                                };
                                let bytes = match STANDARD.decode(data) {
                                    Ok(bytes) => bytes,
                                    Err(_) => {
                                        busy.set(false);
                                        status.set(Some("文件内容解码失败".into()));
                                        return;
                                    }
                                };
                                let name = picked
                                    .name
                                    .filter(|name| !name.is_empty())
                                    .unwrap_or_else(|| "model.bin.gz".into());

                                match import_model(&name, &bytes) {
                                    Ok(model) => {
                                        status.set(Some("正在验证模型…".into()));
                                        let path = model.path.clone();
                                        let (sender, receiver) = std::sync::mpsc::channel();
                                        std::thread::spawn(move || {
                                            let _ = sender.send(validate_model(&path));
                                        });
                                        let validation = loop {
                                            match receiver.try_recv() {
                                                Ok(result) => break result,
                                                Err(std::sync::mpsc::TryRecvError::Empty) => {
                                                    futures_timer::Delay::new(
                                                        std::time::Duration::from_millis(40),
                                                    )
                                                    .await;
                                                }
                                                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                                    break Err("模型校验线程异常退出".into());
                                                }
                                            }
                                        };

                                        if let Err(error) = validation {
                                            let _ = remove_model(&model.id);
                                            model_list.set(models());
                                            active.set(active_model().map(|model| model.id));
                                            status.set(Some(format!("模型无效: {error}")));
                                            on_changed.call(());
                                        } else {
                                            status.set(Some(format!("已导入 {}", model.name)));
                                            model_list.set(models());
                                            active.set(active_model().map(|model| model.id));
                                            on_changed.call(());
                                        }
                                    }
                                    Err(error) => status.set(Some(error)),
                                }
                                busy.set(false);
                            });
                        },
                        if busy() { "读取中…" } else { "导入模型" }
                    }
                }
                p { class: "settings-note",
                    "模型不内置在 APK 中。请从电脑或浏览器下载 KataGo 的 .bin.gz 模型，再在这里导入；App 不会联网下载。"
                }
                if model_items.is_empty() {
                    div { class: "empty-models",
                        span { "尚未导入模型" }
                        span { class: "empty-models-hint", "导入 b6c96 约 3.7MB，b10c128 约 11MB" }
                    }
                } else {
                    div { class: "model-list",
                        for model in model_items {
                            div { class: "model-row", key: "{model.id}",
                                div { class: "model-info",
                                    span { class: "model-name", "{model.name}" }
                                    span { class: "model-meta", "{format_size(model.size)}" }
                                }
                                if active_id.as_deref() == Some(model.id.as_str()) {
                                    span { class: "model-badge", "使用中" }
                                } else {
                                    button {
                                        class: "secondary-button",
                                        r#type: "button",
                                        disabled: busy(),
                                        onclick: {
                                            let id = model.id.clone();
                                            move |_| {
                                                match select_model(&id) {
                                                    Ok(()) => {
                                                        active.set(Some(id.clone()));
                                                        status.set(Some(format!("已切换到 {id}")));
                                                        on_changed.call(());
                                                    }
                                                    Err(error) => status.set(Some(error)),
                                                }
                                            }
                                        },
                                        "使用"
                                    }
                                }
                                button {
                                    class: if delete_armed() == Some(model.id.clone()) {
                                        "danger-button armed"
                                    } else {
                                        "danger-button"
                                    },
                                    r#type: "button",
                                    disabled: busy(),
                                    onclick: {
                                        let id = model.id.clone();
                                        move |_| {
                                            if delete_armed() != Some(id.clone()) {
                                                delete_armed.set(Some(id.clone()));
                                                return;
                                            }
                                            match remove_model(&id) {
                                                Ok(()) => {
                                                    delete_armed.set(None);
                                                    model_list.set(models());
                                                    active.set(active_model().map(|model| model.id));
                                                    status.set(Some("模型已删除".into()));
                                                    on_changed.call(());
                                                }
                                                Err(error) => status.set(Some(error)),
                                            }
                                        }
                                    },
                                    if delete_armed() == Some(model.id.clone()) {
                                        "确认删除"
                                    } else {
                                        "删除"
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(message) = status() {
                    p { class: "settings-status", "{message}" }
                }
            }

            div { class: "settings-card settings-about",
                h2 { "关于" }
                p { class: "settings-note", "LazyGo 离线分析由 KataGo CPU 引擎提供。" }
                p { class: "settings-note", "KataGo v1.18.2 · Eigen backend" }
            }
        }
    }
}
