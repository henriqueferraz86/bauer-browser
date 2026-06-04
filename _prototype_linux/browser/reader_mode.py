"""Reader mode — extracts page content and renders a clean version."""

import json
from pathlib import Path

from .compat import run_javascript, finish_javascript, WebKit

BASE_DIR = Path(__file__).parent.parent

_EXTRACT_JS = """\
(function () {
    var selectors = [
        'article', 'main', '[role="main"]', '.post-content',
        '.article-body', '.entry-content', '.content', '#content',
        '.post', '.story', '.article'
    ];
    var root = null;
    for (var i = 0; i < selectors.length; i++) {
        root = document.querySelector(selectors[i]);
        if (root) break;
    }
    if (!root) root = document.body;

    var clone = root.cloneNode(true);
    clone.querySelectorAll(
        'script,style,nav,header,footer,aside,iframe,' +
        '.ad,.ads,.advertisement,.social,.share,.comments,' +
        '.sidebar,[class*="banner"],[class*="popup"],[id*="popup"],' +
        '[class*="cookie"],[class*="subscribe"]'
    ).forEach(function (el) { el.remove(); });

    return JSON.stringify({
        title: document.title || '',
        url: window.location.href,
        html: clone.innerHTML
    });
})();
"""

_HTML_TEMPLATE = """\
<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title}</title>
<style>{css}</style>
</head>
<body>
<div class="reader-container">
  <h1 class="reader-title">{title}</h1>
  <div class="reader-url">{url}</div>
  <div class="reader-content">{html}</div>
</div>
</body>
</html>
"""


def _load_css() -> str:
    path = BASE_DIR / 'assets' / 'reader_mode.css'
    try:
        return path.read_text(encoding='utf-8')
    except FileNotFoundError:
        return (
            "body{font-family:Georgia,serif;font-size:18px;line-height:1.8;"
            "max-width:720px;margin:40px auto;padding:0 20px;color:#2c3e50}"
        )


def activate(webview) -> None:
    """Replace the current page with a reader-friendly version."""

    def on_done(wv, result, _):
        raw = finish_javascript(wv, result)
        if not raw:
            return
        try:
            data = json.loads(raw)
        except json.JSONDecodeError:
            return

        html = _HTML_TEMPLATE.format(
            title=_esc(data.get('title', 'Reader Mode')),
            url=_esc(data.get('url', '')),
            html=data.get('html', '<p>Could not extract content.</p>'),
            css=_load_css(),
        )
        webview.load_html(html, data.get('url', ''))

    run_javascript(webview, _EXTRACT_JS, on_done)


def _esc(s: str) -> str:
    return s.replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;')
