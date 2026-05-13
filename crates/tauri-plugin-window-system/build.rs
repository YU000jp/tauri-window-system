const COMMANDS: &[&str] = &[
    "open_window",
    "close_window",
    "list_windows",
    "emit_to_window",
];

fn main() {
    use tauri_plugin::Builder;

    Builder::new(COMMANDS).build();
}
