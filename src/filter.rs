//! URL/navigation blocking — top-level navigation filter.

use std::collections::HashSet;

const TRACKER_LIST: &str = include_str!("../assets/blocklists/tracker_domains.txt");

pub struct BlockList {
    domains: HashSet<String>,
}

impl BlockList {
    pub fn load() -> Self {
        let domains = TRACKER_LIST
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| l.to_lowercase())
            .collect();
        Self { domains }
    }

    /// Returns true if the URL should be BLOCKED.
    pub fn is_blocked(&self, url: &str) -> bool {
        let url_lower = url.to_lowercase();
        self.domains.iter().any(|d| url_lower.contains(d.as_str()))
    }

    /// Serialise domain list as a JS array literal for injection into pages.
    pub fn domains_as_js_array(&self) -> String {
        let mut items: Vec<&String> = self.domains.iter().collect();
        items.sort();
        let inner = items
            .iter()
            .map(|d| format!("\"{}\"", d.replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join(",");
        format!("[{inner}]")
    }
}
