"""WebKit version compatibility layer.

Supports WebKit 6.0 (GTK4, Ubuntu 24.04+) and WebKit2 4.1 (Ubuntu 22.04).
"""

import gi
gi.require_version('Gtk', '4.0')

WEBKIT_VERSION = 0

try:
    gi.require_version('WebKit', '6.0')
    from gi.repository import WebKit
    WEBKIT_VERSION = 6
except ValueError:
    pass

if WEBKIT_VERSION == 0:
    try:
        gi.require_version('WebKit2', '4.1')
        from gi.repository import WebKit2 as WebKit  # noqa: F811
        WEBKIT_VERSION = 4
    except ValueError:
        raise ImportError(
            "WebKitGTK not found.\n"
            "Ubuntu 24.04+:  sudo apt install gir1.2-webkit-6.0\n"
            "Ubuntu 22.04:   sudo apt install gir1.2-webkit2-4.1"
        )

from gi.repository import Gtk, GLib, GObject, Gio, Pango  # noqa: E402

__all__ = ['WebKit', 'Gtk', 'GLib', 'GObject', 'Gio', 'Pango', 'WEBKIT_VERSION']


def run_javascript(webview, script: str, callback, user_data=None):
    """Run JS in a WebView — handles both WebKit 6.x and 4.x APIs."""
    if WEBKIT_VERSION >= 6:
        webview.evaluate_javascript(script, -1, None, None, None, callback, user_data)
    else:
        webview.run_javascript(script, None, callback, user_data)


def finish_javascript(webview, result) -> str:
    """Finish a JS evaluation and return the result as a string."""
    try:
        if WEBKIT_VERSION >= 6:
            js_result = webview.evaluate_javascript_finish(result)
        else:
            js_result = webview.run_javascript_finish(result)

        if js_result is None:
            return ''

        js_value = js_result.get_js_value()
        return js_value.to_string() if js_value else ''
    except Exception as e:
        print(f'[compat] JS finish error: {e}')
        return ''
