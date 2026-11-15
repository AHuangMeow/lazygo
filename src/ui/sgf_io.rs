pub const PICK_SGF_SCRIPT: &str = r#"
const start = () => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".sgf,application/x-go-sgf,text/plain";
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
        if (file.size > 4 * 1024 * 1024) {
            done({ error: "SGF 文件过大" });
            return;
        }
        const reader = new FileReader();
        reader.onload = () => done({ name: file.name, text: String(reader.result) });
        reader.onerror = () => done({ error: "读取 SGF 失败" });
        reader.readAsText(file, "utf-8");
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
pub struct PickedSgf {
    pub text: Option<String>,
    pub error: Option<String>,
    pub cancelled: Option<bool>,
}
