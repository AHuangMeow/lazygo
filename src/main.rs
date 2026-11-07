use dioxus::prelude::*;

const MAIN_CSS: Asset = asset!("/assets/main.css");
const APP_ICON: Asset = asset!("/assets/icon.png");
const APP_FONT: Asset = asset!("/assets/fonts/maiyuan-ii-regular.ttf");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let font_face = format!(
        r#"@font-face {{ font-family: "Maiyuan II"; src: url("{}") format("truetype"); font-style: normal; font-weight: 400; font-display: swap; }}"#,
        APP_FONT
    );

    rsx! {
        document::Style { "{font_face}" }
        document::Stylesheet { href: MAIN_CSS }
        main { class: "page",
            header { class: "topbar",
                div { class: "brand",
                    img {
                        class: "brand-icon",
                        src: APP_ICON,
                        alt: "LazyGo",
                    }
                    span { class: "brand-name", "LazyGo" }
                }
                button {
                    class: "menu-button",
                    r#type: "button",
                    aria_label: "More",
                    svg {
                        class: "menu-icon",
                        view_box: "0 0 24 24",
                        path {
                            d: "M4 18h16q.425 0 .713-.288Q21 17.425 21 17t-.287-.712Q20.425 16 20 16H4q-.425 0-.712.288Q3 16.575 3 17t.288.712Q3.575 18 4 18Zm0-5h16q.425 0 .713-.288Q21 12.425 21 12t-.287-.712Q20.425 11 20 11H4q-.425 0-.712.288Q3 11.575 3 12t.288.712Q3.575 13 4 13Zm0-5h16q.425 0 .713-.288Q21 7.425 21 7t-.287-.712Q20.425 6 20 6H4q-.425 0-.712.288Q3 6.575 3 7t.288.712Q3.575 8 4 8Z"
                        }
                    }
                }
            }
            div { class: "content" }
        }
    }
}
