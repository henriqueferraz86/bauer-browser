//! Bauer Browser — Multi-platform browser using Wry 0.37 + Tao 0.25
//!
//! Architecture:
//!   Tao window
//!   ├── Chrome WebView  (index.html — tab bar + toolbar, top CHROME_H px)
//!   └── Content WebViews[0..MAX_TABS]
//!       ├── Active tab  → bounds = content area
//!       └── Hidden tabs → bounds = (-9999, -9999, 1, 1)

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod agent;
mod config;
mod filter;
mod logger;
mod mode;
mod resource;

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wry::{Rect, WebViewBuilder};

use config::Config;
use filter::BlockList;
use mode::{READER_MODE_JS, autoplay_script_for_mode, extra_script_for_mode};
use resource::RamMonitor;

// ── Layout constants ──────────────────────────────────────────────────────────

/// Height of the chrome WebView (tab bar + toolbar) in logical pixels.
const CHROME_H: u32 = 92;
/// Maximum number of tabs (pre-allocated).
const MAX_TABS: usize = 5;

// ── Assets ────────────────────────────────────────────────────────────────────

const CHROME_HTML: &str = include_str!("../chrome/index.html");
const ADBLOCK_JS:  &str = include_str!("../chrome/adblock.js");

// ── IPC commands (chrome → Rust) ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
enum Cmd {
    Navigate     { url: String },
    Back,
    Forward,
    Reload,
    SetMode      { mode: String },
    ReaderMode,
    AgentSummarize,
    NewTab,
    SwitchTab    { index: usize },
    CloseTab     { index: usize },
}

// ── Custom events ─────────────────────────────────────────────────────────────

#[derive(Debug)]
enum AppEvent {
    Command(Cmd),
    TabUrlChanged   { tab: usize, url: String },
    TabTitleChanged { tab: usize, title: String },
    RamUpdate(f64),
    AgentResult(String),
}

// ── Tab metadata (kept in Rust, mirrored to chrome HTML) ─────────────────────

#[derive(Clone)]
struct TabMeta {
    url:   String,
    title: String,
    mode:  String,
    open:  bool,
}

impl TabMeta {
    fn new(url: &str, default_mode: &str) -> Self {
        Self { url: url.into(), title: "New Tab".into(), mode: default_mode.into(), open: true }
    }
    fn blank(default_mode: &str) -> Self {
        Self { url: "about:blank".into(), title: "".into(), mode: default_mode.into(), open: false }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn normalize_url(url: &str) -> String {
    let url = url.trim();
    if url.is_empty() { return "https://duckduckgo.com".into(); }
    if url.starts_with("http://") || url.starts_with("https://") { return url.into(); }
    if url.contains('.') && !url.contains(' ') { format!("https://{url}") }
    else { format!("https://duckduckgo.com/?q={}", url.replace(' ', "+")) }
}

fn js_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'").replace('\n', " ").replace('\r', "")
}

/// Rect covering the content area below the chrome toolbar.
fn content_rect(win_w: u32, win_h: u32) -> Rect {
    Rect { x: 0, y: CHROME_H as i32, width: win_w, height: win_h.saturating_sub(CHROME_H) }
}

/// Off-screen rect used to hide inactive tabs.
fn hidden_rect() -> Rect {
    Rect { x: -9999, y: -9999, width: 1, height: 1 }
}

/// Update WebView bounds: show active tab, hide all others.
fn apply_tab_bounds(tabs: &[wry::WebView], active: usize, win_w: u32, win_h: u32) {
    for (i, wv) in tabs.iter().enumerate() {
        let r = if i == active { content_rect(win_w, win_h) } else { hidden_rect() };
        let _ = wv.set_bounds(r);
    }
}

/// Push tab state to the chrome WebView so the tab bar re-renders.
fn sync_tabs(chrome: &wry::WebView, metas: &[TabMeta], open_count: usize, active: usize) {
    let tabs_json: String = metas[..open_count]
        .iter()
        .map(|m| format!(
            "{{\"title\":\"{}\",\"url\":\"{}\"}}",
            js_escape(&m.title), js_escape(&m.url)
        ))
        .collect::<Vec<_>>()
        .join(",");
    let js = format!("if(typeof updateTabs==='function')updateTabs([{tabs_json}],{active})");
    let _ = chrome.evaluate_script(&js);
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> wry::Result<()> {
    let cfg          = Config::load();
    let home         = cfg.home_url.clone();
    let default_mode = cfg.default_mode.clone();
    let blocklist    = Arc::new(BlockList::load());

    // Init script for content WebViews: domain list + adblock JS only
    // (toolbar is in the separate chrome WebView, not injected into pages)
    let init_script = format!(
        "var __BAUER_BLOCKED__={};\n{}",
        blocklist.domains_as_js_array(),
        ADBLOCK_JS,
    );

    let event_loop = EventLoop::<AppEvent>::new();
    let proxy      = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title("Bauer Browser")
        .with_inner_size(LogicalSize::new(1280_u32, 800_u32))
        .build(&event_loop)
        .expect("Window creation failed");

    let psize  = window.inner_size();
    let scale  = window.scale_factor();
    let win_w  = (psize.width  as f64 / scale) as u32;
    let win_h  = (psize.height as f64 / scale) as u32;

    // ── Chrome WebView (tab bar + toolbar) ────────────────────────────────────
    let proxy_c = proxy.clone();
    let chrome = WebViewBuilder::new_as_child(&window)
        .with_bounds(Rect { x: 0, y: 0, width: win_w, height: CHROME_H })
        .with_html(CHROME_HTML)
        .with_ipc_handler(move |msg: String| {
            match serde_json::from_str::<Cmd>(&msg) {
                Ok(cmd) => { let _ = proxy_c.send_event(AppEvent::Command(cmd)); }
                Err(e)  => eprintln!("[ipc] {e} | {msg}"),
            }
        })
        .build()?;

    // ── Content WebViews (pre-allocated, one per tab slot) ────────────────────
    let mut tab_metas: Vec<TabMeta> = vec![TabMeta::new(&home, &default_mode)];
    for _ in 1..MAX_TABS {
        tab_metas.push(TabMeta::blank(&default_mode));
    }
    let mut open_count: usize = 1;
    let mut active_tab: usize = 0;

    let content_views: Vec<wry::WebView> = (0..MAX_TABS)
        .map(|i| {
            let proxy_t = proxy.clone();
            let bl_t    = blocklist.clone();
            let is      = init_script.clone();
            let url     = if i == 0 { home.as_str() } else { "about:blank" };
            let rect    = if i == 0 { content_rect(win_w, win_h) } else { hidden_rect() };

            WebViewBuilder::new_as_child(&window)
                .with_bounds(rect)
                .with_url(url)
                .with_initialization_script(&is)
                .with_navigation_handler(move |url: String| {
                    if bl_t.is_blocked(&url) { return false; }
                    let _ = proxy_t.send_event(AppEvent::TabUrlChanged { tab: i, url });
                    true
                })
                .build()
        })
        .collect::<Result<_, _>>()?;

    // ── RAM monitor thread ─────────────────────────────────────────────────────
    let proxy_ram = proxy.clone();
    thread::spawn(move || {
        let mut mon = RamMonitor::new();
        loop {
            let _ = proxy_ram.send_event(AppEvent::RamUpdate(mon.total_mb()));
            thread::sleep(Duration::from_secs(2));
        }
    });

    // ── Agent config (clone-friendly tuple) ───────────────────────────────────
    let agent_cfg = (cfg.agent_base_url.clone(), cfg.agent_timeout, cfg.agent_enabled);

    // Mutable state in event loop
    let mut cur_w      = win_w;
    let mut cur_h      = win_h;
    let mut cur_mode   = default_mode.clone();

    // Initial tab bar render
    sync_tabs(&chrome, &tab_metas, open_count, active_tab);

    // ── Event loop ─────────────────────────────────────────────────────────────
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            // ── Commands from chrome toolbar / tab bar ────────────────────────
            Event::UserEvent(AppEvent::Command(cmd)) => match cmd {

                Cmd::Navigate { url } => {
                    let url = normalize_url(&url);
                    tab_metas[active_tab].url = url.clone();
                    content_views[active_tab].load_url(&url);
                }
                Cmd::Back    => { let _ = content_views[active_tab].evaluate_script("history.back()"); }
                Cmd::Forward => { let _ = content_views[active_tab].evaluate_script("history.forward()"); }
                Cmd::Reload  => { let _ = content_views[active_tab].evaluate_script("location.reload()"); }

                Cmd::SetMode { mode } => {
                    logger::log_mode_change(&cur_mode, &mode);
                    cur_mode = mode.clone();
                    tab_metas[active_tab].mode = mode.clone();
                    if let Some(js) = autoplay_script_for_mode(&mode) {
                        let _ = content_views[active_tab].evaluate_script(js);
                    }
                    if let Some(js) = extra_script_for_mode(&mode) {
                        let _ = content_views[active_tab].evaluate_script(js);
                    }
                }

                Cmd::ReaderMode => {
                    let _ = content_views[active_tab].evaluate_script(READER_MODE_JS);
                }

                Cmd::AgentSummarize => {
                    let proxy_ag = proxy.clone();
                    let (base, timeout, enabled) = agent_cfg.clone();
                    thread::spawn(move || {
                        let client = agent::AgentClient::new(base.clone(), timeout, enabled);
                        let result = if client.is_available() {
                            client.summarize("", "").unwrap_or_else(|e| format!("Erro: {e}"))
                        } else {
                            format!("Bauer Agent não disponível.\nInicie em: {base}")
                        };
                        let _ = proxy_ag.send_event(AppEvent::AgentResult(result));
                    });
                }

                // ── Tab management ────────────────────────────────────────────
                Cmd::NewTab => {
                    if open_count < MAX_TABS {
                        let idx = open_count;
                        open_count += 1;
                        tab_metas[idx] = TabMeta::new(&cfg.home_url, &default_mode);
                        content_views[idx].load_url(&cfg.home_url);
                        active_tab = idx;
                        apply_tab_bounds(&content_views, active_tab, cur_w, cur_h);
                        sync_tabs(&chrome, &tab_metas, open_count, active_tab);
                        window.set_title("New Tab — Bauer Browser");
                    }
                }

                Cmd::SwitchTab { index } => {
                    if index < open_count {
                        active_tab = index;
                        cur_mode   = tab_metas[active_tab].mode.clone();
                        apply_tab_bounds(&content_views, active_tab, cur_w, cur_h);
                        sync_tabs(&chrome, &tab_metas, open_count, active_tab);
                        // Update URL bar on chrome
                        let url = tab_metas[active_tab].url.clone();
                        let js  = format!(
                            "if(typeof setUrlBar==='function')setUrlBar('{}')",
                            js_escape(&url)
                        );
                        let _ = chrome.evaluate_script(&js);
                        let title = tab_metas[active_tab].title.clone();
                        window.set_title(&format!("{title} — Bauer Browser"));
                    }
                }

                Cmd::CloseTab { index } => {
                    // index == usize::MAX means close active tab (sent as -1 from JS)
                    let index = if index == usize::MAX { active_tab } else { index };
                    if open_count > 1 && index < open_count {
                        // Shift tabs down to fill the gap
                        for i in index..open_count - 1 {
                            tab_metas[i] = tab_metas[i + 1].clone();
                            // Move WebView content by navigating — swap URLs
                            let url = tab_metas[i].url.clone();
                            content_views[i].load_url(&url);
                        }
                        // Blank out last slot
                        tab_metas[open_count - 1] = TabMeta::blank(&default_mode);
                        content_views[open_count - 1].load_url("about:blank");
                        open_count -= 1;

                        if active_tab >= open_count { active_tab = open_count - 1; }
                        apply_tab_bounds(&content_views, active_tab, cur_w, cur_h);
                        sync_tabs(&chrome, &tab_metas, open_count, active_tab);
                    }
                }
            },

            // ── Content URL changed → update tab meta + chrome UI ─────────────
            Event::UserEvent(AppEvent::TabUrlChanged { tab, url }) => {
                if tab < MAX_TABS {
                    tab_metas[tab].url = url.clone();
                }
                if tab == active_tab {
                    logger::log_navigation(&url, &cur_mode, 0.0);
                    window.set_title(&format!("{url} — Bauer Browser"));
                    let js = format!(
                        "if(typeof setUrlBar==='function')setUrlBar('{}')",
                        js_escape(&url)
                    );
                    let _ = chrome.evaluate_script(&js);
                    sync_tabs(&chrome, &tab_metas, open_count, active_tab);
                }
            }

            // ── Title changed → update tab label ─────────────────────────────
            Event::UserEvent(AppEvent::TabTitleChanged { tab, title }) => {
                if tab < MAX_TABS {
                    tab_metas[tab].title = title.clone();
                }
                if tab == active_tab {
                    window.set_title(&format!("{title} — Bauer Browser"));
                    sync_tabs(&chrome, &tab_metas, open_count, active_tab);
                }
            }

            // ── RAM update ────────────────────────────────────────────────────
            Event::UserEvent(AppEvent::RamUpdate(mb)) => {
                let alert = mb > cfg.ram_alert_mb;
                let js = format!(
                    "if(typeof setRam==='function')setRam({mb:.0},{alert})"
                );
                let _ = chrome.evaluate_script(&js);
            }

            // ── Agent result ──────────────────────────────────────────────────
            Event::UserEvent(AppEvent::AgentResult(text)) => {
                let js = format!(
                    "if(typeof showAgent==='function')showAgent('{}')",
                    js_escape(&text)
                );
                let _ = chrome.evaluate_script(&js);
            }

            // ── Window resize ─────────────────────────────────────────────────
            Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                let scale = window.scale_factor();
                cur_w = (size.width  as f64 / scale) as u32;
                cur_h = (size.height as f64 / scale) as u32;
                let _ = chrome.set_bounds(Rect { x: 0, y: 0, width: cur_w, height: CHROME_H });
                apply_tab_bounds(&content_views, active_tab, cur_w, cur_h);
            }

            // ── Close ─────────────────────────────────────────────────────────
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }

            _ => {}
        }
    });

    Ok(())
}
