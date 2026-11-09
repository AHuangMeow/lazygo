use base64::{engine::general_purpose::STANDARD, Engine as _};
use dioxus::prelude::*;
use lazygo::analysis::{
    active_model, disable_model, import_model, models, select_model, validate_model, ModelInfo,
};

use super::Menu;

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
            done({ cancelled: true });
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
    input.oncancel = () => done({ cancelled: true });
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
    cancelled: Option<bool>,
}

#[component]
pub fn SettingsPage(
    max_visits: u32,
    candidate_count: u32,
    on_changed: EventHandler<()>,
    on_depth: EventHandler<u32>,
    on_candidate_count: EventHandler<u32>,
) -> Element {
    let mut model_list = use_signal(models);
    let mut active = use_signal(|| active_model().map(|model| model.id));
    let mut menu_open = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut status = use_signal(|| None::<String>);
    let mut depth = use_signal(|| max_visits);

    let model_items: Vec<ModelInfo> = model_list();
    let active_id = active();
    let active_name = active_id
        .as_deref()
        .and_then(|id| model_items.iter().find(|model| model.id == id))
        .map(|model| model.name.as_str())
        .unwrap_or("禁用");
    let current_depth = depth();

    rsx! {
        section { class: "settings-page",
            h1 { class: "settings-page-title", "模型与分析" }
            div { class: "settings-divider" }

            div { class: "settings-row",
                span { class: "settings-label", "模型" }
                span { class: "settings-leader" }
                div { class: "settings-control",
                    Menu {
                        label: active_name.to_string(),
                        trigger_class: "model-select-trigger",
                        options_class: "model-options",
                        aria_label: "Model",
                        open: menu_open(),
                        locked: false,
                        on_toggle: move |_| menu_open.toggle(),
                        button {
                            class: if active_id.is_none() {
                                "menu-option muted selected"
                            } else {
                                "menu-option muted"
                            },
                            r#type: "button",
                            onclick: move |_| {
                                match disable_model() {
                                    Ok(()) => {
                                        active.set(None);
                                        menu_open.set(false);
                                        on_changed.call(());
                                    }
                                    Err(error) => status.set(Some(error)),
                                }
                            },
                            "禁用"
                        }
                        for model in model_items.iter() {
                            {
                                let id = model.id.clone();
                                let selected = active_id.as_deref() == Some(id.as_str());
                                rsx! {
                                    button {
                                        class: if selected { "menu-option selected" } else { "menu-option" },
                                        r#type: "button",
                                        onclick: move |_| {
                                            match select_model(&id) {
                                                Ok(()) => {
                                                    active.set(Some(id.clone()));
                                                    menu_open.set(false);
                                                    on_changed.call(());
                                                }
                                                Err(error) => status.set(Some(error)),
                                            }
                                        },
                                        "{model.name}"
                                    }
                                }
                            }
                        }
                    }
                    button {
                        class: "settings-import",
                        r#type: "button",
                        disabled: busy(),
                        onclick: move |_| {
                            if busy() {
                                return;
                            }
                            busy.set(true);
                            status.set(None);
                            spawn(async move {
                                let mut eval = document::eval(PICK_MODEL_SCRIPT);
                                let message = match eval.recv::<String>().await {
                                    Ok(message) => message,
                                    Err(_) => {
                                        busy.set(false);
                                        status.set(Some("文件选择失败".into()));
                                        return;
                                    }
                                };

                                let picked: PickedModel = match serde_json::from_str(&message) {
                                    Ok(picked) => picked,
                                    Err(_) => {
                                        busy.set(false);
                                        status.set(Some("无法读取所选文件".into()));
                                        return;
                                    }
                                };

                                if picked.cancelled.unwrap_or(false) {
                                    busy.set(false);
                                    status.set(None);
                                    return;
                                }
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
                                                    break Err("模型校验异常".into());
                                                }
                                            }
                                        };

                                        if let Err(error) = validation {
                                            let _ = lazygo::analysis::remove_model(&model.id);
                                            status.set(Some(error));
                                        } else {
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
                        if busy() { "验证中…" } else { "导入" }
                    }
                }
            }

            div { class: "settings-row",
                span { class: "settings-label", "分析深度" }
                span { class: "settings-leader" }
                div { class: "settings-control",
                    input {
                        class: "depth-input",
                        r#type: "number",
                        min: "1",
                        max: "5000",
                        step: "10",
                        value: "{current_depth}",
                        disabled: active_id.is_none(),
                        onchange: move |event| {
                            if let Ok(value) = event.value().parse::<u32>() {
                                let value = value.clamp(1, 5000);
                                depth.set(value);
                                on_depth.call(value);
                            }
                        },
                    }
                }
            }

            div { class: "settings-row",
                span { class: "settings-label", "推荐选点数" }
                span { class: "settings-leader" }
                div { class: "settings-control",
                    input {
                        class: "depth-input",
                        r#type: "number",
                        min: "1",
                        max: "10",
                        step: "1",
                        value: "{candidate_count}",
                        disabled: active_id.is_none(),
                        onchange: move |event| {
                            if let Ok(value) = event.value().parse::<u32>() {
                                on_candidate_count.call(value.clamp(1, 10));
                            }
                        },
                    }
                }
            }

            if let Some(message) = status() {
                p { class: "settings-error", "{message}" }
            }

            footer { class: "settings-footer",
                "KataGo v1.18.2 · Eigen CPU"
            }
        }
    }
}
