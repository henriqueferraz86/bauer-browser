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
mod bookmarks;
mod config;
mod filter;
mod history;
mod logger;
mod mode;
mod resource;

use std::path::PathBuf;
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

// Injected into every content WebView: lets Rust extract page text for the Bauer Agent.
// Title changes are handled natively via with_document_title_changed_handler (not IPC).
const CONTENT_IPC_JS: &str = r#"(function(){})();"#;

// ── IPC commands (chrome → Rust) ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
enum Cmd {
    Navigate        { url: String },
    Home,
    Back,
    Forward,
    Reload,
    SetMode         { mode: String },
    ReaderMode,
    AgentSummarize,
    SaveBookmark,
    RemoveBookmark  { url: String },
    ShowBookmarks,
    NewTab,
    NextTab,
    PrevTab,
    SwitchTab       { index: usize },
    CloseTab        { index: usize },
}

// ── IPC messages (content WebViews → Rust) ───────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
enum ContentMsg {
    PageContent { content: String, url: String },
}

// ── Custom events ─────────────────────────────────────────────────────────────

#[derive(Debug)]
enum AppEvent {
    Command(Cmd),
    TabUrlChanged    { tab: usize, url: String },
    TabTitleChanged  { tab: usize, title: String },
    RamUpdate(f64),
    AgentResult(String),
    PageContent      { tab: usize, url: String, content: String },
    DownloadStarted  { filename: String },
    DownloadFinished { filename: String, success: bool },
}

// ── Tab metadata ──────────────────────────────────────────────────────────────

#[derive(Clone)]
struct TabMeta {
    url:         String,
    title:       String,
    mode:        String,
    open:        bool,
    reader_mode: bool,
}

impl TabMeta {
    fn new(url: &str, default_mode: &str) -> Self {
        Self { url: url.into(), title: "New Tab".into(), mode: default_mode.into(), open: true, reader_mode: false }
    }
    fn blank(default_mode: &str) -> Self {
        Self { url: "about:blank".into(), title: "".into(), mode: default_mode.into(), open: false, reader_mode: false }
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

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn generate_bookmarks_html(list: &[bookmarks::Bookmark]) -> String {
    let items = if list.is_empty() {
        "<p class=\"empty\">Nenhum favorito ainda.<br>Clique ☆ na toolbar para adicionar.</p>".to_string()
    } else {
        let rows: String = list.iter().map(|b| format!(
            "<li><a href=\"{href}\">{title}</a><span class=\"url\">{url}</span></li>",
            href  = html_escape(&b.url),
            title = html_escape(if b.title.is_empty() { &b.url } else { &b.title }),
            url   = html_escape(&b.url),
        )).collect();
        format!("<ul>{rows}</ul>")
    };
    format!(r#"<!DOCTYPE html><html><head><meta charset="utf-8">
<title>Favoritos — Bauer Browser</title>
<style>
*{{box-sizing:border-box;margin:0;padding:0}}
body{{font-family:system-ui,-apple-system,sans-serif;background:#1e2030;color:#c0caf5;
      padding:48px 24px;max-width:720px;margin:0 auto}}
h1{{color:#7aa2f7;font-size:22px;margin-bottom:24px}}
ul{{list-style:none}}
li{{padding:14px 0;border-bottom:1px solid #2a2d3e}}
a{{color:#7dcfff;font-size:15px;text-decoration:none;display:block;margin-bottom:4px}}
a:hover{{color:#c0caf5;text-decoration:underline}}
.url{{display:block;font-size:11px;color:#565f89}}
p.empty{{color:#565f89;text-align:center;margin-top:60px;line-height:2}}
</style></head><body>
<h1>☆ Favoritos</h1>
{items}
</body></html>"#)
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

    let history_urls    = history::load_urls();
    let mut bm_list     = bookmarks::load();

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

            let proxy_title = proxy.clone();
            let proxy_dls   = proxy.clone();
            let proxy_dlc   = proxy.clone();
            WebViewBuilder::new_as_child(&window)
                .with_bounds(rect)
                .with_url(url)
                .with_initialization_script(&is)
                .with_navigation_handler(move |url: String| {
                    if block_enabled && bl_t.is_blocked(&url) { return false; }
                    let _ = proxy_nav.send_event(AppEvent::TabUrlChanged { tab: i, url });
                    true
                })
                // Native title handler — fires whenever document.title changes (B-01)
                .with_document_title_changed_handler(move |title: String| {
                    let _ = proxy_title.send_event(AppEvent::TabTitleChanged { tab: i, title });
                })
                // Download started: redirect destination to ~/Downloads/ (F-04)
                .with_download_started_handler(move |_url: String, path: &mut PathBuf| {
                    let filename = path
                        .file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_else(|| "download".to_string());
                    let dl_dir = dirs::download_dir()
                        .unwrap_or_else(|| PathBuf::from("."));
                    *path = dl_dir.join(&filename);
                    let _ = proxy_dls.send_event(AppEvent::DownloadStarted { filename });
                    true
                })
                // Download completed: notify chrome UI (F-04)
                .with_download_completed_handler(move |_url: String, path: Option<PathBuf>, success: bool| {
                    let filename = path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_else(|| "download".to_string());
                    let _ = proxy_dlc.send_event(AppEvent::DownloadFinished { filename, success });
                })
                // IPC handler for page content extraction (used by Bauer Agent, B-02)
                .with_ipc_handler(move |msg: String| {
                    match serde_json::from_str::<ContentMsg>(&msg) {
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

    // Send initial bookmark star state to chrome (F-03)
    let is_bm = bm_list.iter().any(|b| b.url == home);
    let _ = chrome.evaluate_script(&format!(
        "if(typeof setBookmarkState==='function')setBookmarkState({})", is_bm
    ));

    // ── Event loop ─────────────────────────────────────────────────────────────
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(AppEvent::Command(cmd)) => match cmd {

                Cmd::Navigate { url } => {
                    let url = normalize_url(&url);
                    tab_metas[active_tab].url = url.clone();
                    tab_metas[active_tab].reader_mode = false;
                    let _ = chrome.evaluate_script(
                        "if(typeof setReaderMode==='function')setReaderMode(false)"
                    );
                    content_views[active_tab].load_url(&url);
                }
                Cmd::Home => {
                    let url = cfg.home_url.clone();
                    tab_metas[active_tab].url = url.clone();
                    tab_metas[active_tab].reader_mode = false;
                    let _ = chrome.evaluate_script(
                        "if(typeof setReaderMode==='function')setReaderMode(false)"
                    );
                    content_views[active_tab].load_url(&url);
                }
                Cmd::Back    => {
                    tab_metas[active_tab].reader_mode = false;
                    let _ = chrome.evaluate_script(
                        "if(typeof setReaderMode==='function')setReaderMode(false)"
                    );
                    let _ = content_views[active_tab].evaluate_script("history.back()");
                }
                Cmd::Forward => {
                    tab_metas[active_tab].reader_mode = false;
                    let _ = chrome.evaluate_script(
                        "if(typeof setReaderMode==='function')setReaderMode(false)"
                    );
                    let _ = content_views[active_tab].evaluate_script("history.forward()");
                }
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
                    if tab_metas[active_tab].reader_mode {
                        // Deactivate: reload restores the original page
                        tab_metas[active_tab].reader_mode = false;
                        let _ = content_views[active_tab].evaluate_script("location.reload()");
                        let _ = chrome.evaluate_script(
                            "if(typeof setReaderMode==='function')setReaderMode(false)"
                        );
                    } else {
                        tab_metas[active_tab].reader_mode = true;
                        let _ = content_views[active_tab].evaluate_script(READER_MODE_JS);
                        let _ = chrome.evaluate_script(
                            "if(typeof setReaderMode==='function')setReaderMode(true)"
                        );
                    }
                }

                // Step 1: request page content via content IPC; agent fires in PageContent handler
                Cmd::ShowBookmarks => {
                    let html = generate_bookmarks_html(&bm_list);
                    let escaped = serde_json::to_string(&html).unwrap_or_else(|_| "''".to_string());
                    let _ = content_views[active_tab].evaluate_script(&format!(
                        "document.open();document.write({escaped});document.close();"
                    ));
                    tab_metas[active_tab].reader_mode = false;
                    let _ = chrome.evaluate_script(
                        "if(typeof setReaderMode==='function')setReaderMode(false)"
                    );
                }

                Cmd::SaveBookmark => {
                    let url   = tab_metas[active_tab].url.clone();
                    let title = tab_metas[active_tab].title.clone();
                    bookmarks::add(&url, &title, &mut bm_list);
                    let _ = chrome.evaluate_script(
                        "if(typeof setBookmarkState==='function')setBookmarkState(true)"
                    );
                }

                Cmd::RemoveBookmark { url } => {
                    let active_url = tab_metas[active_tab].url.clone();
                    bookmarks::remove(&url, &mut bm_list);
                    if url == active_url {
                        let _ = chrome.evaluate_script(
                            "if(typeof setBookmarkState==='function')setBookmarkState(false)"
                        );
                    }
                }

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

                Cmd::NextTab => {
                    if open_count > 1 {
                        active_tab = (active_tab + 1) % open_count;
                        cur_mode = tab_metas[active_tab].mode.clone();
                        apply_tab_bounds(&content_views, active_tab, cur_w, cur_h);
                        sync_tabs(&chrome, &tab_metas, open_count, active_tab, max_tabs);
                        let url = tab_metas[active_tab].url.clone();
                        let _ = chrome.evaluate_script(&format!(
                            "if(typeof setUrlBar==='function')setUrlBar('{}')", js_escape(&url)
                        ));
                        let is_bm = bm_list.iter().any(|b| b.url == url);
                        let _ = chrome.evaluate_script(&format!(
                            "if(typeof setBookmarkState==='function')setBookmarkState({})", is_bm
                        ));
                        let rm = tab_metas[active_tab].reader_mode;
                        let _ = chrome.evaluate_script(&format!(
                            "if(typeof setReaderMode==='function')setReaderMode({})", rm
                        ));
                        let title = tab_metas[active_tab].title.clone();
                        window.set_title(&format!("{} — Bauer Browser",
                            if title.is_empty() { url } else { title }
                        ));
                    }
                }

                Cmd::PrevTab => {
                    if open_count > 1 {
                        active_tab = if active_tab == 0 { open_count - 1 } else { active_tab - 1 };
                        cur_mode = tab_metas[active_tab].mode.clone();
                        apply_tab_bounds(&content_views, active_tab, cur_w, cur_h);
                        sync_tabs(&chrome, &tab_metas, open_count, active_tab, max_tabs);
                        let url = tab_metas[active_tab].url.clone();
                        let _ = chrome.evaluate_script(&format!(
                            "if(typeof setUrlBar==='function')setUrlBar('{}')", js_escape(&url)
                        ));
                        let is_bm = bm_list.iter().any(|b| b.url == url);
                        let _ = chrome.evaluate_script(&format!(
                            "if(typeof setBookmarkState==='function')setBookmarkState({})", is_bm
                        ));
                        let rm = tab_metas[active_tab].reader_mode;
                        let _ = chrome.evaluate_script(&format!(
                            "if(typeof setReaderMode==='function')setReaderMode({})", rm
                        ));
                        let title = tab_metas[active_tab].title.clone();
                        window.set_title(&format!("{} — Bauer Browser",
                            if title.is_empty() { url } else { title }
                        ));
                    }
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
                        // Update bookmark star for the newly active tab (F-03)
                        let is_bm = bm_list.iter().any(|b| b.url == url);
                        let _ = chrome.evaluate_script(&format!(
                            "if(typeof setBookmarkState==='function')setBookmarkState({})", is_bm
                        ));
                        // Sync reader mode button state (fix toggle)
                        let rm = tab_metas[active_tab].reader_mode;
                        let _ = chrome.evaluate_script(&format!(
                            "if(typeof setReaderMode==='function')setReaderMode({})", rm
                        ));
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
                    // Update bookmark star state (F-03)
                    let is_bm = bm_list.iter().any(|b| b.url == url);
                    let _ = chrome.evaluate_script(&format!(
                        "if(typeof setBookmarkState==='function')setBookmarkState({})", is_bm
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

            // ── Download started (F-04) ───────────────────────────────────────
            Event::UserEvent(AppEvent::DownloadStarted { filename }) => {
                let js = format!(
                    "if(typeof showDownload==='function')showDownload('{}','started')",
                    js_escape(&filename)
                );
                let _ = chrome.evaluate_script(&js);
            }

            // ── Download finished (F-04) ──────────────────────────────────────
            Event::UserEvent(AppEvent::DownloadFinished { filename, success }) => {
                let status = if success { "done" } else { "error" };
                let js = format!(
                    "if(typeof showDownload==='function')showDownload('{}','{}')",
                    js_escape(&filename), status
                );
                let _ = chrome.evaluate_script(&js);
            }

            // ── DPI / monitor change ──────────────────────────────────────────
            // Fires when the window moves to a different monitor or the OS
            // display scale changes. Recalculate logical bounds so WebViews
            // cover the full window at the new scale factor.
            Event::WindowEvent {
                event: WindowEvent::ScaleFactorChanged { new_inner_size, .. }, ..
            } => {
                let scale = window.scale_factor();
                cur_w = (new_inner_size.width  as f64 / scale) as u32;
                cur_h = (new_inner_size.height as f64 / scale) as u32;
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
