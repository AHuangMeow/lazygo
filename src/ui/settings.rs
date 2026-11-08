use dioxus::prelude::*;
use lazygo::game::{Rules, BOARD_SIZES};

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
pub fn KomiField(value: String, locked: bool, on_input: EventHandler<String>) -> Element {
    rsx! {
        label { class: if locked { "komi-field locked" } else { "komi-field" },
            span { class: "komi-label", "贴目" }
            input {
                class: "komi-input",
                r#type: "number",
                inputmode: "decimal",
                step: "0.5",
                min: "0",
                max: "100",
                value: "{value}",
                aria_label: "Komi",
                disabled: locked,
                oninput: move |event| on_input.call(event.value()),
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
