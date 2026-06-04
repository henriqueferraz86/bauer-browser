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
mod history;
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
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use wry::{Rect, WebViewBuilder};

use config::Config;
use filter::BlockList;
use mode::{READER_MODE_JS, autoplay_script_for_mode, extra_script_for_mode};
use resource::RamMonitor;

// ── Layout constants ──────────────────────────────────────────────────────────

const CHROME_H: u32 = 92;
const MAX_TABS: usize = 5;

// ── Assets ────────────────────────────────────────────────────────────────────

const CHROME_HTML: &str = include_str!("../chrome/index.html");
const ADBLOCK_JS:  &str = include_str!("../chrome/adblock.js");

// Injected into every content WebView: watches document.title and lets Rust
// extract page text for the Bauer Agent.
const CONTENT_IPC_JS: &str = r#"(function(){
    function _reportTitle(){
        try{window.ipc.postMessage(JSON.stringify({action:'titleChanged',title:document.title}));}catch(_){}
    }
    document.addEventListener('DOMContentLoaded',_reportTitle);
    window.addEventListener('load',_reportTitle);
    new MutationObserver(_reportTitle)
        .observe(document.documentElement,{subtree:true,childList:true,characterData:true});
})();"#;

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

// ── IPC messages (content WebViews → Rust) ───────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
enum ContentMsg {
    TitleChanged { title: String },
    PageContent  { content: String, url: String },
}

// ── Custom events ─────────────────────────────────────────────────────────────

#[derive(Debug)]
enum AppEvent {
    Command(Cmd),
    TabUrlChanged   { tab: usize, url: String },
    TabTitleChanged { tab: usize, title: String },
    RamUpdate(f64),
    AgentResult(String),
    PageContent     { tab: usize, url: String, content: String },
}

// ── Tab metadata ──────────────────────────────────────────────────────────────

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

fn content_rect(win_w: u32, win_h: u32) -> Rect {
    Rect { x: 0, y: CHROME_H as i32, width: win_w, height: win_h.saturating_sub(CHROME_H) }
}

fn hidden_rect() -> Rect {
    Rect { x: -9999, y: -9999, width: 1, height: 1 }
}

fn apply_tab_bounds(tabs: &[wry::WebView], active: usize, win_w: u32, win_h: u32) {
    for (i, wv) in tabs.iter().enumerate() {
        let r = if i == active { content_rect(win_w, win_h) } else { hidden_rect() };
        let _ = wv.set_bounds(r);
    }
}

fn sync_tabs(chrome: &wry::WebView, metas: &[TabMeta], open_count: usize, active: usize, max_tabs: usize) {
    let tabs_json: String = metas[..open_count]
        .iter()
        .map(|m| format!(
            "{{\"title\":\"{}\",\"url\":\"{}\"}}",
            js_escape(&m.title), js_escape(&m.url)
        ))
        .collect::<Vec<_>>()
        .join(",");
    let js = format!("if(typeof updateTabs==='function')updateTabs([{tabs_json}],{active},{max_tabs})");
    let _ = chrome.evaluate_script(&js);
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> wry::Result<()> {
    let cfg          = Config::load();
    let home         = cfg.home_url.clone();
    let default_mode = cfg.default_mode.clone();
    let max_tabs     = cfg.max_tabs.min(MAX_TABS);
    let block_enabled = cfg.block_trackers || cfg.block_ads;
    let blocklist    = Arc::new(BlockList::load());

    logger::init(cfg.log_enabled);

    let history_urls = history::load_urls();

    let init_script = format!(
        "var __BAUER_BLOCKED__={};\n{}\n{}",
        blocklist.domains_as_js_array(),
        ADBLOCK_JS,
        CONTENT_IPC_JS,
    );

    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();
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

    // ── Chrome WebView ────────────────────────────────────────────────────────
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

    // ── Content WebViews ──────────────────────────────────────────────────────
    let mut tab_metas: Vec<TabMeta> = vec![TabMeta::new(&home, &default_mode)];
    for _ in 1..MAX_TABS {
        tab_metas.push(TabMeta::blank(&default_mode));
    }
    let mut open_count: usize = 1;
    let mut active_tab: usize = 0;

    let content_views: Vec<wry::WebView> = (0..MAX_TABS)
        .map(|i| {
            let proxy_nav   = proxy.clone();
            let proxy_ipc   = proxy.clone();
            let bl_t        = blocklist.clone();
            let is          = init_script.clone();
            let url         = if i == 0 { home.as_str() } else { "about:blank" };
            let rect        = if i == 0 { content_rect(win_w, win_h) } else { hidden_rect() };

            WebViewBuilder::new_as_child(&window)
                .with_bounds(rect)
                .with_url(url)
                .with_initialization_script(&is)
                .with_navigation_handler(move |url: String| {
                    if block_enabled && bl_t.is_blocked(&url) { return false; }
                    let _ = proxy_nav.send_event(AppEvent::TabUrlChanged { tab: i, url });
                    true
                })
                // Receives titleChanged and pageContent messages from content pages (B-01, B-02)
                .with_ipc_handler(move |msg: String| {
                    match serde_json::from_str::<ContentMsg>(&msg) {
                        Ok(ContentMsg::TitleChanged { title }) => {
                            let _ = proxy_ipc.send_event(AppEvent::TabTitleChanged { tab: i, title });
                        }
                        Ok(ContentMsg::PageContent { content, url }) => {
                            let _ = proxy_ipc.send_event(AppEvent::PageContent { tab: i, url, content });
                        }
                        Err(e) => eprintln!("[content-ipc:{i}] {e} | {msg}"),
                    }
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

    let agent_cfg = (cfg.agent_base_url.clone(), cfg.agent_timeout, cfg.agent_enabled);

    let mut cur_w    = win_w;
    let mut cur_h    = win_h;
    let mut cur_mode = default_mode.clone();

    sync_tabs(&chrome, &tab_metas, open_count, active_tab, max_tabs);

    // Send history to chrome datalist for URL autocomplete (F-02)
    let history_json = serde_json::to_string(&history_urls).unwrap_or_else(|_| "[]".to_string());
    let _ = chrome.evaluate_script(&format!(
        "if(typeof setHistory==='function')setHistory({})", history_json
    ));

    // ── Event loop ─────────────────────────────────────────────────────────────
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
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

                // Step 1: request page content via content IPC; agent fires in PageContent handler
                Cmd::AgentSummarize => {
                    let url = tab_metas[active_tab].url.clone();
                    logger::log_agent_request(&url);
                    let _ = content_views[active_tab].evaluate_script(
                        "try{window.ipc.postMessage(JSON.stringify({\
                            action:'pageContent',\
                            content:document.body.innerText.substring(0,8000),\
                            url:window.location.href\
                        }));}catch(_){}"
                    );
                }

                Cmd::NewTab => {
                    if open_count < max_tabs {
                        let idx = open_count;
                        open_count += 1;
                        tab_metas[idx] = TabMeta::new(&cfg.home_url, &default_mode);
                        content_views[idx].load_url(&cfg.home_url);
                        active_tab = idx;
                        apply_tab_bounds(&content_views, active_tab, cur_w, cur_h);
                        sync_tabs(&chrome, &tab_metas, open_count, active_tab, max_tabs);
                        window.set_title("New Tab — Bauer Browser");
                    }
                }

                Cmd::SwitchTab { index } => {
                    if index < open_count {
                        active_tab = index;
                        cur_mode   = tab_metas[active_tab].mode.clone();
                        apply_tab_bounds(&content_views, active_tab, cur_w, cur_h);
                        sync_tabs(&chrome, &tab_metas, open_count, active_tab, max_tabs);
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
                    let index = if index == usize::MAX { active_tab } else { index };

                    if open_count == 1 {
                        // B-06: last tab — reset to home instead of exiting
                        tab_metas[0] = TabMeta::new(&cfg.home_url, &default_mode);
                        content_views[0].load_url(&cfg.home_url);
                        active_tab = 0;
                        sync_tabs(&chrome, &tab_metas, 1, 0, max_tabs);
                    } else if index < open_count {
                        // Shift tabs down to fill the gap
                        for i in index..open_count - 1 {
                            tab_metas[i] = tab_metas[i + 1].clone();
                            let url = tab_metas[i].url.clone();
                            content_views[i].load_url(&url);
                        }
                        tab_metas[open_count - 1] = TabMeta::blank(&default_mode);
                        content_views[open_count - 1].load_url("about:blank");
                        open_count -= 1;

                        // B-07: select adjacent tab correctly
                        active_tab = if index < active_tab {
                            active_tab - 1           // closed tab was before active
                        } else {
                            index.min(open_count - 1) // closed active or after; prefer same index
                        };

                        apply_tab_bounds(&content_views, active_tab, cur_w, cur_h);
                        sync_tabs(&chrome, &tab_metas, open_count, active_tab, max_tabs);

                        let url = tab_metas[active_tab].url.clone();
                        let js = format!(
                            "if(typeof setUrlBar==='function')setUrlBar('{}')",
                            js_escape(&url)
                        );
                        let _ = chrome.evaluate_script(&js);
                        let title = tab_metas[active_tab].title.clone();
                        window.set_title(&format!("{} — Bauer Browser",
                            if title.is_empty() { url } else { title }
                        ));
                    }
                }
            },

            // ── Content URL changed ───────────────────────────────────────────
            Event::UserEvent(AppEvent::TabUrlChanged { tab, url }) => {
                if tab < MAX_TABS {
                    tab_metas[tab].url = url.clone();
                }
                if tab == active_tab {
                    logger::log_navigation(&url, &cur_mode, 0.0);
                    // Persist to history and push new URL to chrome autocomplete (F-02)
                    history::append(&url, &tab_metas[tab].title);
                    let _ = chrome.evaluate_script(&format!(
                        "if(typeof addToHistory==='function')addToHistory('{}')",
                        js_escape(&url)
                    ));
                    window.set_title(&format!("{url} — Bauer Browser"));
                    let js = format!(
                        "if(typeof setUrlBar==='function')setUrlBar('{}')",
                        js_escape(&url)
                    );
                    let _ = chrome.evaluate_script(&js);
                    sync_tabs(&chrome, &tab_metas, open_count, active_tab, max_tabs);
                }
            }

            // ── Title changed (B-01 fix) ──────────────────────────────────────
            Event::UserEvent(AppEvent::TabTitleChanged { tab, title }) => {
                if tab < MAX_TABS && !title.is_empty() {
                    tab_metas[tab].title = title.clone();
                }
                if tab == active_tab && !title.is_empty() {
                    window.set_title(&format!("{title} — Bauer Browser"));
                    sync_tabs(&chrome, &tab_metas, open_count, active_tab, max_tabs);
                }
            }

            // ── Page content received → spawn agent (B-02 fix) ───────────────
            Event::UserEvent(AppEvent::PageContent { tab, url, content }) => {
                if tab == active_tab {
                    let proxy_ag = proxy.clone();
                    let (base, timeout, enabled) = agent_cfg.clone();
                    thread::spawn(move || {
                        let client = agent::AgentClient::new(base.clone(), timeout, enabled);
                        let result = if client.is_available() {
                            client.summarize(&url, &content)
                                .unwrap_or_else(|e| format!("Erro: {e}"))
                        } else {
                            format!("Bauer Agent não disponível.\nInicie em: {base}")
                        };
                        let _ = proxy_ag.send_event(AppEvent::AgentResult(result));
                    });
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
