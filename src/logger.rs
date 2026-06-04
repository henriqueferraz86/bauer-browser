//! Navigation and resource logger — appends lines to ~/.bauer-browser/navigation.log

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::Local;

static LOG_ENABLED: AtomicBool = AtomicBool::new(true);

pub fn init(enabled: bool) {
    LOG_ENABLED.store(enabled, Ordering::Relaxed);
}

fn log_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".bauer-browser")
        .join("navigation.log")
}

fn write_line(parts: &[&str]) {
    if !LOG_ENABLED.load(Ordering::Relaxed) { return; }
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        let now = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        let line = format!("{} | {}\n", now, parts.join(" | "));
        let _ = file.write_all(line.as_bytes());
    }
}

pub fn log_navigation(url: &str, mode: &str, ram_mb: f64) {
    write_line(&[
        "NAV",
        &format!("mode={mode}"),
        &format!("url={url}"),
        &format!("ram={ram_mb:.1}MB"),
    ]);
}

pub fn log_mode_change(old: &str, new: &str) {
    write_line(&["MODE", &format!("{old}->{new}")]);
}

pub fn log_agent_request(url: &str) {
    write_line(&["AGENT_REQ", &format!("url={url}")]);
}
