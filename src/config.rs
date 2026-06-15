//! Configuration — reads config/settings.toml, falls back to defaults.

use std::path::Path;

#[derive(Debug, Clone)]
pub struct Config {
    pub max_tabs:        usize,
    pub default_mode:    String,
    pub home_url:        String,
    pub ram_alert_mb:    f64,
    pub block_trackers:  bool,
    pub block_ads:       bool,
    pub agent_enabled:   bool,
    pub agent_base_url:  String,
    pub agent_timeout:   u64,
    pub log_enabled:     bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_tabs:       5,
            default_mode:   "normal".into(),
            home_url:       "https://www.google.com".into(),
            ram_alert_mb:   300.0,
            block_trackers: true,
            block_ads:      true,
            agent_enabled:  true,
            agent_base_url: "http://localhost:8742".into(),
            agent_timeout:  10,
            log_enabled:    true,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let mut cfg = Self::default();
        let path = Path::new("config/settings.toml");
        if !path.exists() {
            return cfg;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            eprintln!("[config] Cannot read settings.toml");
            return cfg;
        };
        let Ok(table) = text.parse::<toml::Value>() else {
            eprintln!("[config] Cannot parse settings.toml");
            return cfg;
        };

        macro_rules! str_val {
            ($section:literal, $key:literal, $field:ident) => {
                if let Some(v) = table
                    .get($section)
                    .and_then(|s| s.get($key))
                    .and_then(|v| v.as_str())
                {
                    cfg.$field = v.to_string();
                }
            };
        }
        macro_rules! int_val {
            ($section:literal, $key:literal, $field:ident, $t:ty) => {
                if let Some(v) = table
                    .get($section)
                    .and_then(|s| s.get($key))
                    .and_then(|v| v.as_integer())
                {
                    cfg.$field = v as $t;
                }
            };
        }
        macro_rules! float_val {
            ($section:literal, $key:literal, $field:ident) => {
                if let Some(v) = table
                    .get($section)
                    .and_then(|s| s.get($key))
                    .and_then(|v| v.as_float())
                {
                    cfg.$field = v;
                }
            };
        }
        macro_rules! bool_val {
            ($section:literal, $key:literal, $field:ident) => {
                if let Some(v) = table
                    .get($section)
                    .and_then(|s| s.get($key))
                    .and_then(|v| v.as_bool())
                {
                    cfg.$field = v;
                }
            };
        }

        int_val!("browser", "max_tabs",     max_tabs,    usize);
        str_val!("browser", "default_mode", default_mode);
        str_val!("browser", "home_url",     home_url);
        float_val!("browser", "ram_alert_mb", ram_alert_mb);
        bool_val!("blocklists", "block_trackers", block_trackers);
        bool_val!("blocklists", "block_ads",      block_ads);
        bool_val!("agent", "enabled",        agent_enabled);
        str_val!("agent", "base_url",        agent_base_url);
        int_val!("agent", "timeout_seconds", agent_timeout, u64);
        bool_val!("logging", "enabled", log_enabled);

        cfg
    }
}
