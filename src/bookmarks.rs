//! Bookmarks — persisted to ~/.bauer-browser/bookmarks.json

use std::fs;
use std::path::PathBuf;

use chrono::Local;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub url:       String,
    pub title:     String,
    pub timestamp: String,
}

fn bookmarks_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".bauer-browser")
        .join("bookmarks.json")
}

pub fn load() -> Vec<Bookmark> {
    let path = bookmarks_path();
    let Ok(text) = fs::read_to_string(&path) else { return vec![]; };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save(bookmarks: &[Bookmark]) {
    let path = bookmarks_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(bookmarks) {
        let _ = fs::write(&path, json);
    }
}

/// Add bookmark for current page. No-op if URL already exists. Returns updated list.
pub fn add(url: &str, title: &str, list: &mut Vec<Bookmark>) {
    if url.is_empty() || url.starts_with("about:") { return; }
    if list.iter().any(|b| b.url == url) { return; }
    list.insert(0, Bookmark {
        url:       url.to_string(),
        title:     if title.is_empty() { url.to_string() } else { title.to_string() },
        timestamp: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
    });
    save(list);
}

/// Remove bookmark by URL. Returns true if something was removed.
pub fn remove(url: &str, list: &mut Vec<Bookmark>) -> bool {
    let before = list.len();
    list.retain(|b| b.url != url);
    let removed = list.len() < before;
    if removed { save(list); }
    removed
}
