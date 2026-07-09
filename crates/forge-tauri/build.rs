const COMMANDS: &[&str] = &[
    "request",
    "widget_open",
    "widget_send_text",
    "widget_send_binary",
    "widget_close",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
