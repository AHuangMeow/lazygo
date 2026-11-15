use dioxus::prelude::*;
use lazygo::game::{Komi, Rules, BOARD_SIZES};

use super::Menu;

#[component]
pub fn RuleMenu(
    value: Rules,
    locked: bool,
    open: bool,
    on_toggle: EventHandler<()>,
    on_select: EventHandler<Rules>,
) -> Element {
    rsx! {
        Menu {
            label: value.label().to_string(),
            trigger_class: "rule-trigger",
            options_class: "rule-options",
            aria_label: "Rules",
            open,
            locked,
            on_toggle,
            for rule in Rules::ALL {
                button {
                    class: if value == rule { "menu-option selected" } else { "menu-option" },
                    r#type: "button",
                    onclick: move |_| on_select.call(rule),
                    "{rule.label()}"
                }
            }
        }
    }
}

#[component]
pub fn KomiField(value: Komi, locked: bool, on_input: EventHandler<Komi>) -> Element {
    let mut draft = use_signal(|| value.to_string());
    let mut synced = use_signal(|| value);

    use_effect(move || {
        if synced() != value {
            synced.set(value);
            draft.set(value.to_string());
        }
    });

    let draft_value = draft();
    let invalid = Komi::parse(&draft_value).is_err();
    let class = if locked {
        "komi-field locked"
    } else if invalid {
        "komi-field invalid"
    } else {
        "komi-field"
    };

    rsx! {
        label { class: "{class}",
            span { class: "komi-label", "贴目" }
            input {
                class: "komi-input",
                r#type: "number",
                inputmode: "decimal",
                step: "0.5",
                min: "0",
                max: "100",
                value: "{draft_value}",
                aria_label: "Komi",
                aria_invalid: if invalid { "true" } else { "false" },
                disabled: locked,
                oninput: move |event| {
                    let text = event.value();
                    draft.set(text.clone());
                    if let Ok(komi) = Komi::parse(&text) {
                        on_input.call(komi);
                    }
                },
                onchange: move |_| {
                    if Komi::parse(&draft()).is_err() {
                        draft.set(value.to_string());
                    }
                },
            }
        }
    }
}

#[component]
pub fn SizeMenu(
    size: usize,
    locked: bool,
    open: bool,
    on_toggle: EventHandler<()>,
    on_select: EventHandler<usize>,
) -> Element {
    rsx! {
        Menu {
            label: format!("{size}x{size}"),
            trigger_class: "size-trigger",
            options_class: "size-options",
            aria_label: "Board size",
            open,
            locked,
            on_toggle,
            for option in BOARD_SIZES {
                button {
                    class: if size == option { "menu-option selected" } else { "menu-option" },
                    r#type: "button",
                    onclick: move |_| on_select.call(option),
                    "{option}x{option}"
                }
            }
        }
    }
}
