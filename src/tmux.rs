use std::process::Command;

pub fn capture_pane(target: &str) -> Option<String> {
    let output = Command::new("tmux")
        .args(["capture-pane", "-t", target, "-p"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

pub fn send_keys(target: &str, keys: &str) {
    let _ = Command::new("tmux")
        .args(["send-keys", "-t", target, keys])
        .output();
}
