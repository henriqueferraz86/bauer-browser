"""Tab manager — lifecycle, mode switching and RAM display."""

import uuid
from dataclasses import dataclass, field
from typing import Callable, Optional

from .compat import WebKit, Gtk, GLib, Pango
from .webview_factory import create_webview, reconfigure_webview
from .resource_monitor import ResourceMonitor
from .mode_policy import get_mode_color, get_mode_label, MODES


@dataclass
class TabInfo:
    tab_id: str
    webview: object
    mode: str
    # Label widgets
    label_box: object
    title_label: object
    ram_label: object
    mode_dot: object


class TabManager:
    def __init__(self, notebook: Gtk.Notebook, config, filter_manager, logger):
        self.notebook = notebook
        self.config = config
        self.filter_manager = filter_manager
        self.logger = logger

        self.max_tabs: int = config.get('browser', 'max_tabs', 5)
        self.default_mode: str = config.get('browser', 'default_mode', 'normal')
        self.ram_alert_mb: float = config.get('browser', 'ram_alert_mb', 300)

        self._tabs: dict[str, TabInfo] = {}

        self.monitor = ResourceMonitor(interval_ms=2000)
        self.monitor.start()

        # External callbacks
        self.on_url_changed: Optional[Callable] = None
        self.on_title_changed: Optional[Callable] = None

    # ── public API ────────────────────────────────────────────────────────────

    @property
    def count(self) -> int:
        return len(self._tabs)

    def can_open(self) -> bool:
        return self.count < self.max_tabs

    def new_tab(self, url: str = '', mode: str = None) -> Optional[str]:
        if not self.can_open():
            return None
        mode = mode or self.default_mode
        tab_id = uuid.uuid4().hex[:8]

        webview = create_webview(mode, self.filter_manager)
        label_box, title_lbl, ram_lbl, dot = self._make_label(tab_id, mode)

        info = TabInfo(tab_id, webview, mode, label_box, title_lbl, ram_lbl, dot)
        self._tabs[tab_id] = info

        page = self.notebook.append_page(webview, label_box)
        self.notebook.set_tab_reorderable(webview, True)
        self.notebook.set_current_page(page)

        self._connect(webview, tab_id)

        self.monitor.register(
            tab_id,
            lambda: self._get_pid(webview),
            lambda tid, ram: GLib.idle_add(self._on_ram, tid, ram),
        )

        target = url or self.config.get('browser', 'home_url', 'https://duckduckgo.com')
        webview.load_uri(target)
        return tab_id

    def close_tab(self, tab_id: str) -> None:
        info = self._tabs.get(tab_id)
        if not info:
            return
        self.monitor.unregister(tab_id)
        pg = self.notebook.page_num(info.webview)
        if pg >= 0:
            self.notebook.remove_page(pg)
        info.webview.destroy()
        del self._tabs[tab_id]
        if not self._tabs:
            self.new_tab()

    def set_mode(self, tab_id: str, new_mode: str) -> None:
        info = self._tabs.get(tab_id)
        if not info or info.mode == new_mode:
            return
        old = info.mode
        info.mode = new_mode
        info.mode_dot.set_markup(f'<span color="{get_mode_color(new_mode)}">●</span>')
        reconfigure_webview(info.webview, new_mode, self.filter_manager)
        info.webview.reload()
        self.logger.log_mode_change(tab_id, old, new_mode)

    def navigate(self, url: str) -> None:
        wv = self.current_webview()
        if not wv:
            return
        if not url.startswith(('http://', 'https://')):
            if '.' in url and ' ' not in url:
                url = 'https://' + url
            else:
                url = 'https://duckduckgo.com/?q=' + url.replace(' ', '+')
        wv.load_uri(url)

    def go_back(self) -> None:
        wv = self.current_webview()
        if wv and wv.can_go_back():
            wv.go_back()

    def go_forward(self) -> None:
        wv = self.current_webview()
        if wv and wv.can_go_forward():
            wv.go_forward()

    def reload(self) -> None:
        wv = self.current_webview()
        if wv:
            wv.reload()

    def current_tab(self) -> Optional[TabInfo]:
        pg = self.notebook.get_current_page()
        if pg < 0:
            return None
        wv = self.notebook.get_nth_page(pg)
        for info in self._tabs.values():
            if info.webview == wv:
                return info
        return None

    def current_webview(self):
        info = self.current_tab()
        return info.webview if info else None

    def current_mode(self) -> str:
        info = self.current_tab()
        return info.mode if info else self.default_mode

    # ── private ───────────────────────────────────────────────────────────────

    def _make_label(self, tab_id: str, mode: str):
        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=4)

        dot = Gtk.Label()
        dot.set_markup(f'<span color="{get_mode_color(mode)}">●</span>')
        box.append(dot)

        title = Gtk.Label(label='New Tab')
        title.set_max_width_chars(22)
        title.set_ellipsize(Pango.EllipsizeMode.END)
        box.append(title)

        ram = Gtk.Label(label='')
        ram.add_css_class('dim-label')
        box.append(ram)

        close = Gtk.Button(label='✕')
        close.set_has_frame(False)
        close.add_css_class('flat')
        close.connect('clicked', lambda _: self.close_tab(tab_id))
        box.append(close)

        return box, title, ram, dot

    def _connect(self, webview, tab_id: str) -> None:
        webview.connect('notify::title',
                        lambda wv, _: self._on_title(tab_id, wv))
        webview.connect('notify::uri',
                        lambda wv, _: self._on_uri(tab_id, wv))

    def _on_title(self, tab_id: str, wv) -> None:
        info = self._tabs.get(tab_id)
        if not info:
            return
        title = wv.get_title() or 'Loading…'
        info.title_label.set_text(title)
        if self.on_title_changed:
            self.on_title_changed(tab_id, title)

    def _on_uri(self, tab_id: str, wv) -> None:
        uri = wv.get_uri() or ''
        info = self._tabs.get(tab_id)
        if info and not wv.get_title():
            info.title_label.set_text(uri or 'New Tab')
        self.logger.log_navigation(uri, info.mode if info else '')
        if self.on_url_changed:
            self.on_url_changed(tab_id, uri)

    def _on_ram(self, tab_id: str, ram_mb: float) -> bool:
        info = self._tabs.get(tab_id)
        if not info:
            return False
        if ram_mb > self.ram_alert_mb:
            info.ram_label.set_markup(
                f'<span color="#e74c3c"> {ram_mb:.0f}MB⚠</span>'
            )
        else:
            info.ram_label.set_text(f' {ram_mb:.0f}MB')
        return False

    @staticmethod
    def _get_pid(webview) -> Optional[int]:
        try:
            return webview.get_web_process_identifier()
        except AttributeError:
            return None
