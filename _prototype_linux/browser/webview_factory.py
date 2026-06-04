"""WebView factory — creates and reconfigures WebKit WebViews per mode."""

from .compat import WebKit
from .mode_policy import apply_mode_settings

# ── Injected scripts ──────────────────────────────────────────────────────────

_BLOCK_AUTOPLAY_JS = """\
(function () {
    'use strict';
    function muteMedia(el) {
        el.autoplay = false;
        el.muted = true;
        try { el.pause(); } catch (_) {}
    }
    document.querySelectorAll('video, audio').forEach(muteMedia);
    new MutationObserver(function (mutations) {
        mutations.forEach(function (m) {
            m.addedNodes.forEach(function (n) {
                if (!n || n.nodeType !== 1) return;
                if (n.tagName === 'VIDEO' || n.tagName === 'AUDIO') muteMedia(n);
                n.querySelectorAll && n.querySelectorAll('video, audio').forEach(muteMedia);
            });
        });
    }).observe(document.documentElement, { childList: true, subtree: true });
})();
"""

_BLOCK_3RD_PARTY_SCRIPTS_JS = """\
(function () {
    'use strict';
    var host = window.location.hostname;
    var _orig = document.createElement.bind(document);
    document.createElement = function (tag) {
        var el = _orig(tag);
        if (tag.toLowerCase() !== 'script') return el;
        var _set = el.setAttribute.bind(el);
        el.setAttribute = function (name, val) {
            if (name === 'src') {
                try {
                    var u = new URL(val, window.location.href);
                    if (u.hostname && u.hostname !== host &&
                            !u.hostname.endsWith('.' + host)) {
                        console.log('[Bauer Lite] blocked 3rd-party script:', val);
                        return;
                    }
                } catch (_) {}
            }
            return _set(name, val);
        };
        return el;
    };
})();
"""


# ── Factory ───────────────────────────────────────────────────────────────────

def create_webview(mode: str, filter_manager=None) -> WebKit.WebView:
    """Return a new WebView configured for *mode*."""
    ucm = WebKit.UserContentManager.new()
    webview = WebKit.WebView.new_with_user_content_manager(ucm)
    apply_mode_settings(webview, mode)
    _inject_scripts(ucm, mode)
    if filter_manager:
        filter_manager.apply_to_webview(webview, mode)
    return webview


def reconfigure_webview(webview, mode: str, filter_manager=None) -> None:
    """Reconfigure an existing WebView for a new mode and reload."""
    ucm = webview.get_user_content_manager()
    ucm.remove_all_scripts()
    ucm.remove_all_filters()
    apply_mode_settings(webview, mode)
    _inject_scripts(ucm, mode)
    if filter_manager:
        filter_manager.apply_to_webview(webview, mode)


def _inject_scripts(ucm, mode: str) -> None:
    if mode in ('lite', 'normal'):
        _add_script(ucm, _BLOCK_AUTOPLAY_JS,
                    WebKit.UserContentInjectedFrames.ALL_FRAMES,
                    WebKit.UserScriptInjectionTime.START)

    if mode == 'lite':
        _add_script(ucm, _BLOCK_3RD_PARTY_SCRIPTS_JS,
                    WebKit.UserContentInjectedFrames.MAIN_FRAME_ONLY,
                    WebKit.UserScriptInjectionTime.START)


def _add_script(ucm, source: str, frames, inject_time) -> None:
    script = WebKit.UserScript.new(source, frames, inject_time, None, None)
    ucm.add_script(script)
