use dioxus::prelude::*;

#[component]
pub fn Menu(
    label: String,
    trigger_class: &'static str,
    options_class: &'static str,
    aria_label: &'static str,
    open: bool,
    locked: bool,
    on_toggle: EventHandler<()>,
    children: Element,
) -> Element {
    let expanded = open && !locked;

    rsx! {
        div { class: "menu",
            button {
                class: if expanded { "menu-trigger {trigger_class} open" } else { "menu-trigger {trigger_class}" },
                r#type: "button",
                aria_label: "{aria_label}",
                aria_expanded: if expanded { "true" } else { "false" },
                disabled: locked,
                onclick: move |_| on_toggle.call(()),
                span { "{label}" }
                span { class: "chevron" }
            }
            div {
                class: if expanded { "menu-backdrop open" } else { "menu-backdrop" },
                onclick: move |_| on_toggle.call(()),
            }
            div {
                class: if expanded {
                    "menu-options {options_class} open"
                } else {
                    "menu-options {options_class}"
                },
                {children}
            }
        }
    }
}
