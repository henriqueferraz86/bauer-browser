"""Mode policies — WebKit settings for Lite, Normal and Full modes."""

from .compat import WebKit

MODES = ('lite', 'normal', 'full')

MODE_LABELS = {
    'lite':   '🌿 Lite',
    'normal': '⚡ Normal',
    'full':   '🔥 Full',
}

MODE_COLORS = {
    'lite':   '#2ecc71',
    'normal': '#3498db',
    'full':   '#e74c3c',
}


def apply_mode_settings(webview, mode: str) -> None:
    """Apply WebKit settings to a WebView for the given mode."""
    s = webview.get_settings()

    if mode == 'lite':
        s.set_enable_javascript(True)
        s.set_media_playback_requires_user_gesture(True)
        s.set_enable_webgl(False)
        s.set_javascript_can_open_windows_automatically(False)
        s.set_enable_page_cache(False)
        s.set_enable_media_stream(False)
        _try_set(s, 'set_enable_encrypted_media', False)
        _try_set(s, 'set_enable_dns_prefetching', False)

    elif mode == 'normal':
        s.set_enable_javascript(True)
        s.set_media_playback_requires_user_gesture(True)
        s.set_enable_webgl(True)
        s.set_javascript_can_open_windows_automatically(False)
        s.set_enable_page_cache(True)
        s.set_enable_media_stream(True)
        _try_set(s, 'set_enable_encrypted_media', True)
        _try_set(s, 'set_enable_dns_prefetching', True)

    elif mode == 'full':
        s.set_enable_javascript(True)
        s.set_media_playback_requires_user_gesture(False)
        s.set_enable_webgl(True)
        s.set_javascript_can_open_windows_automatically(True)
        s.set_enable_page_cache(True)
        s.set_enable_media_stream(True)
        _try_set(s, 'set_enable_encrypted_media', True)
        _try_set(s, 'set_enable_dns_prefetching', True)


def _try_set(settings, method: str, value):
    fn = getattr(settings, method, None)
    if fn:
        try:
            fn(value)
        except Exception:
            pass


def get_mode_color(mode: str) -> str:
    return MODE_COLORS.get(mode, '#888888')


def get_mode_label(mode: str) -> str:
    return MODE_LABELS.get(mode, mode.title())


def mode_index(mode: str) -> int:
    return list(MODES).index(mode) if mode in MODES else 1
