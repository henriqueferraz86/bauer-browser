//! Browsing history — JSONL file at ~/.bauer-browser/history.jsonl

use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use chrono::Local;
use serde::{Deserialize, Serialize};

const MAX_ENTRIES: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub url:       String,
    pub title:     String,
    pub timestamp: String,
}

fn history_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".bauer-browser")
        .join("history.jsonl")
}

/// Append one entry. Skips about: URLs and blank entries.
pub fn append(url: &str, title: &str) {
    if url.is_empty() || url.starts_with("about:") { return; }
    let path = history_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let entry = HistoryEntry {
        url:       url.to_string(),
        title:     title.to_string(),
        timestamp: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
    };
    if let Ok(line) = serde_json::to_string(&entry) {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(file, "{line}");
        }
    }
}

/// Load history, newest first, capped at MAX_ENTRIES. Returns only URLs (deduped).
pub fn load_urls() -> Vec<String> {
    let path = history_path();
    let Ok(file) = fs::File::open(&path) else { return vec![]; };
    let reader = io::BufReader::new(file);

    let mut urls: Vec<String> = reader
        .lines()
        .filter_map(|l| l.ok())
        .filter_map(|l| serde_json::from_str::<HistoryEntry>(&l).ok())
        .map(|e| e.url)
        .collect();

    urls.reverse();

    // Deduplicate keeping first occurrence (most recent)
    let mut seen = std::collections::HashSet::new();
    urls.retain(|u| seen.insert(u.clone()));
    urls.truncate(MAX_ENTRIES);
    urls
}
