"""Bauer Agent HTTP client — optional IA integration."""

import json
import threading
import urllib.request
import urllib.error
from typing import Callable

from .compat import GLib, run_javascript, finish_javascript

_EXTRACT_TEXT_JS = """\
(function () {
    return document.body ? document.body.innerText.slice(0, 8000) : '';
})();
"""


class BauerAgentClient:
    def __init__(self, config):
        self.enabled: bool = config.get('agent', 'enabled', True)
        self.base_url: str = config.get('agent', 'base_url', 'http://localhost:8742')
        self.timeout: int = config.get('agent', 'timeout_seconds', 10)
        self._available = False

        if self.enabled:
            self._check_health()
            GLib.timeout_add(30_000, self._health_tick)

    @property
    def is_available(self) -> bool:
        return self.enabled and self._available

    # ── health check ──────────────────────────────────────────────────────────

    def _health_tick(self) -> bool:
        self._check_health()
        return True

    def _check_health(self) -> None:
        def _do():
            ok = False
            try:
                with urllib.request.urlopen(f'{self.base_url}/health', timeout=3) as r:
                    ok = r.status == 200
            except Exception:
                pass
            GLib.idle_add(self._set_available, ok)

        threading.Thread(target=_do, daemon=True).start()

    def _set_available(self, ok: bool) -> bool:
        self._available = ok
        return False

    # ── summarize ─────────────────────────────────────────────────────────────

    def summarize_page(
        self,
        webview,
        on_result: Callable[[str], None],
        on_error: Callable[[str], None],
    ) -> None:
        """Extract page text and send to Bauer Agent for summary."""

        def on_js_done(wv, result, _):
            text = finish_javascript(wv, result)
            url = wv.get_uri() or ''

            def _send():
                try:
                    payload = json.dumps({
                        'url': url,
                        'content': text,
                        'mode': 'summary',
                    }).encode()
                    req = urllib.request.Request(
                        f'{self.base_url}/summarize',
                        data=payload,
                        headers={'Content-Type': 'application/json'},
                        method='POST',
                    )
                    with urllib.request.urlopen(req, timeout=self.timeout) as r:
                        data = json.loads(r.read().decode())
                    summary = data.get('summary', '(sem resposta)')
                    GLib.idle_add(lambda: on_result(summary) or False)
                except Exception as e:
                    GLib.idle_add(lambda: on_error(str(e)) or False)

            threading.Thread(target=_send, daemon=True).start()

        run_javascript(webview, _EXTRACT_TEXT_JS, on_js_done)
