# Bauer Browser

Lightweight, controlled, AI-integrated browser for BauerOS and resource-constrained machines.  
**Multi-platform:** Windows · macOS · Linux — uses the native OS WebView (no bundled engine).

---

## How it works

```
Tao window
├── Chrome WebView  ← toolbar UI (HTML/CSS/JS, local)  — top 80px
└── Content WebView ← web pages (external URLs)        — fills rest
```

- **Windows:** WebView2 (Microsoft Edge Chromium) — pre-installed on Win 10/11
- **macOS:** WKWebView (WebKit/Safari) — native
- **Linux:** WebKitGTK — install via apt/dnf

---

## Requirements

### Windows
- Windows 10 build 1803+ or Windows 11
- WebView2 Runtime — already installed via Windows Update on most machines  
  If missing: download from https://developer.microsoft.com/en-us/microsoft-edge/webview2/
- [Rust](https://rustup.rs) — install once, used to build the binary

### macOS
- macOS 10.15+
- [Rust](https://rustup.rs)

### Linux (Ubuntu/Debian)
```bash
# Ubuntu 24.04+
sudo apt install libwebkitgtk-6.0-dev libgtk-3-dev libssl-dev pkg-config

# Ubuntu 22.04
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libssl-dev pkg-config
```
Plus [Rust](https://rustup.rs).

---

## Build & Run

### Windows
```bat
build.bat run          :: debug build + run
build.bat release run  :: optimized build + run
```

### Linux / macOS
```bash
chmod +x build.sh
./build.sh run          # debug build + run
./build.sh release run  # optimized build + run
```

### Manual
```bash
cargo run              # debug
cargo run --release    # optimized
```

> **First build:** downloads all crates (~100–200 MB) and may take 3–5 minutes.  
> Subsequent builds are incremental and fast.

---

## Configuration

Edit `config/settings.toml`:

```toml
[browser]
max_tabs     = 5
default_mode = "normal"   # "lite" | "normal" | "full"
home_url     = "https://duckduckgo.com"
ram_alert_mb = 300        # MB threshold for RAM warning

[blocklists]
block_trackers = true     # blocks navigation to tracker domains
block_ads      = true

[agent]
enabled         = true
base_url        = "http://localhost:8742"
timeout_seconds = 10

[logging]
enabled = true            # logs to ~/.bauer-browser/navigation.log
```

---

## Modes

| | 🌿 Lite | ⚡ Normal | 🔥 Full |
|---|---|---|---|
| Tracker navigation blocked | ✅ | ✅ | ✅ |
| Autoplay blocked (JS) | ✅ | ✅ | ❌ |
| 3rd-party script injection blocked | ✅ | ❌ | ❌ |
| WebGL, Media | ❌ | ✅ | ✅ |
| Best for | Articles, reading | Daily browsing | Banks, Gmail, Teams |

---

## Project structure

```
bauer-browser/
├── Cargo.toml
├── src/
│   ├── main.rs        ← event loop, WebViews, IPC routing
│   ├── config.rs      ← TOML config loader
│   ├── mode.rs        ← Lite/Normal/Full JS scripts
│   ├── filter.rs      ← URL blocklist
│   ├── resource.rs    ← RAM monitor (sysinfo)
│   ├── agent.rs       ← Bauer Agent HTTP client
│   └── logger.rs      ← navigation log
├── chrome/
│   └── index.html     ← browser toolbar UI
├── assets/
│   └── blocklists/
│       └── tracker_domains.txt
├── config/
│   ├── defaults.toml
│   └── settings.toml  ← user config (gitignored)
├── build.bat          ← Windows build script
├── build.sh           ← Linux/macOS build script
└── _prototype_linux/  ← original Python+WebKitGTK prototype (reference)
```

---

## Bauer Agent integration

The browser connects to `http://localhost:8742` (configurable).

Expected API:
```
GET  /health    → 200 OK
POST /summarize → { "url": "...", "content": "...", "mode": "summary" }
               ← { "summary": "..." }
```

Click **🤖 Resumir** to send the current page to the agent.  
If unavailable, a message is shown in the result panel.

---

## Navigation log

`~/.bauer-browser/navigation.log`:
```
2026-06-03T14:22:01 | NAV | mode=normal | url=https://example.com | ram=87.0MB
2026-06-03T14:22:15 | MODE | normal->lite
2026-06-03T14:22:30 | AGENT_REQ | url=https://example.com
```

---

## Keyboard shortcuts

| Key | Action |
|---|---|
| Enter (in URL bar) | Navigate |
| Alt + ← | Back |
| Alt + → | Forward |
| F5 / Ctrl+R | Reload |
| Ctrl+L | Focus URL bar |
| Ctrl+T | New Tab |

---

## Pending — V2

- [ ] True multi-tab (multiple content WebViews, tab bar)
- [ ] Sub-resource blocking (ads/trackers inline — requires platform-specific API)
- [ ] Hard RAM limit per tab (cgroups on Linux, Job Objects on Windows)
- [ ] Auto-detect heavy pages → suggest Lite Mode
- [ ] Navigation history & bookmarks
- [ ] Download manager
- [ ] Robust reader mode (Readability.js port)
- [ ] Trusted/untrusted site profiles
- [ ] macOS app bundle (.app) and code signing
- [ ] Windows installer (.msi)
- [ ] WPE WebKit build for BauerOS kiosk/embedded

---

## Legacy

`_prototype_linux/` contains the original Python + WebKitGTK prototype  
(Linux-only, not maintained). Useful as reference for business logic.
