"""Tests for mode_policy.py — run with: python -m pytest tests/"""

import sys
import types
import unittest

# ── Minimal GTK/WebKit stubs so tests run without a display ──────────────────

def _make_stub_module(name):
    m = types.ModuleType(name)
    sys.modules[name] = m
    return m

gi_mod = _make_stub_module('gi')
gi_repo = _make_stub_module('gi.repository')

class _FakeSettings:
    def __init__(self):
        self._state = {}
    def __getattr__(self, name):
        if name.startswith('set_'):
            def setter(val): self._state[name] = val
            return setter
        raise AttributeError(name)

class _FakeWebView:
    def __init__(self):
        self._settings = _FakeSettings()
    def get_settings(self):
        return self._settings

class _FakeWebKit:
    WebView = _FakeWebView

gi_repo.WebKit = _FakeWebKit()

def _require_version(mod, ver): pass
gi_mod.require_version = _require_version

# Now import (compat will pick up stubs)
sys.modules['browser.compat'] = types.ModuleType('browser.compat')
sys.modules['browser.compat'].WebKit = _FakeWebKit()
sys.modules['browser.compat'].Gtk = None
sys.modules['browser.compat'].GLib = None
sys.modules['browser.compat'].GObject = None
sys.modules['browser.compat'].Gio = None
sys.modules['browser.compat'].Pango = None
sys.modules['browser.compat'].WEBKIT_VERSION = 6
sys.modules['browser.compat'].run_javascript = lambda *a, **k: None
sys.modules['browser.compat'].finish_javascript = lambda *a, **k: ''

from browser.mode_policy import (  # noqa: E402
    apply_mode_settings, MODES, get_mode_color, get_mode_label, mode_index
)

# ── Tests ─────────────────────────────────────────────────────────────────────

class TestModes(unittest.TestCase):
    def _wv(self):
        return _FakeWebView()

    def test_modes_tuple(self):
        self.assertEqual(MODES, ('lite', 'normal', 'full'))

    def test_mode_index(self):
        self.assertEqual(mode_index('lite'), 0)
        self.assertEqual(mode_index('normal'), 1)
        self.assertEqual(mode_index('full'), 2)
        self.assertEqual(mode_index('unknown'), 1)  # fallback

    def test_mode_colors_distinct(self):
        colors = [get_mode_color(m) for m in MODES]
        self.assertEqual(len(set(colors)), 3, 'Each mode must have a unique color')

    def test_mode_labels_non_empty(self):
        for m in MODES:
            self.assertTrue(get_mode_label(m), f'Label for {m} is empty')

    def test_lite_blocks_autoplay(self):
        wv = self._wv()
        apply_mode_settings(wv, 'lite')
        self.assertTrue(
            wv.get_settings()._state.get('set_media_playback_requires_user_gesture'),
            'Lite must require user gesture for media'
        )

    def test_lite_disables_webgl(self):
        wv = self._wv()
        apply_mode_settings(wv, 'lite')
        self.assertFalse(
            wv.get_settings()._state.get('set_enable_webgl'),
            'Lite must disable WebGL'
        )

    def test_normal_blocks_autoplay(self):
        wv = self._wv()
        apply_mode_settings(wv, 'normal')
        self.assertTrue(
            wv.get_settings()._state.get('set_media_playback_requires_user_gesture'),
            'Normal must still require user gesture for autoplay'
        )

    def test_full_allows_autoplay(self):
        wv = self._wv()
        apply_mode_settings(wv, 'full')
        self.assertFalse(
            wv.get_settings()._state.get('set_media_playback_requires_user_gesture'),
            'Full must allow autoplay'
        )

    def test_full_enables_webgl(self):
        wv = self._wv()
        apply_mode_settings(wv, 'full')
        self.assertTrue(
            wv.get_settings()._state.get('set_enable_webgl'),
            'Full must enable WebGL'
        )


if __name__ == '__main__':
    unittest.main()
