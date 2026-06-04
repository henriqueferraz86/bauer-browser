"""Main browser window — GTK4 ApplicationWindow."""

from .compat import Gtk, GLib
from .tab_manager import TabManager
from .content_filter import get_filter_manager
from .agent_client import BauerAgentClient
from .reader_mode import activate as activate_reader
from .logger import BrowserLogger
from .mode_policy import MODES, get_mode_label, mode_index


class BrowserWindow(Gtk.ApplicationWindow):
    def __init__(self, application, config):
        super().__init__(application=application)
        self.config = config
        self.logger = BrowserLogger(config)
        self.agent = BauerAgentClient(config)

        w = config.get('browser', 'window_width', 1200)
        h = config.get('browser', 'window_height', 800)
        self.set_default_size(w, h)
        self.set_title('Bauer Browser')

        self._updating_dropdown = False  # guard against recursive mode signals

        filter_mgr = get_filter_manager(config)

        self._build_ui()

        self.tabs = TabManager(self.notebook, config, filter_mgr, self.logger)
        self.tabs.on_url_changed = self._sync_url
        self.tabs.on_title_changed = self._sync_title

        home = config.get('browser', 'home_url', 'https://duckduckgo.com')
        self.tabs.new_tab(home)

        GLib.timeout_add(5_000, self._refresh_agent_btn)

    # ── UI construction ───────────────────────────────────────────────────────

    def _build_ui(self):
        root = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        self.set_child(root)

        self._build_headerbar()

        self.notebook = Gtk.Notebook()
        self.notebook.set_scrollable(True)
        self.notebook.set_show_border(False)
        self.notebook.set_vexpand(True)
        self.notebook.connect('switch-page', self._on_tab_switch)
        root.append(self.notebook)

        self.agent_panel = self._build_agent_panel()
        root.append(self.agent_panel)

    def _build_headerbar(self):
        hb = Gtk.HeaderBar()
        hb.set_show_title_buttons(True)
        self.set_titlebar(hb)

        # ── Left: navigation + new tab ────────────────────────────────────
        left = Gtk.Box(spacing=2)

        self.btn_back = Gtk.Button.new_from_icon_name('go-previous-symbolic')
        self.btn_back.set_tooltip_text('Back')
        self.btn_back.connect('clicked', lambda _: self.tabs.go_back())
        left.append(self.btn_back)

        self.btn_fwd = Gtk.Button.new_from_icon_name('go-next-symbolic')
        self.btn_fwd.set_tooltip_text('Forward')
        self.btn_fwd.connect('clicked', lambda _: self.tabs.go_forward())
        left.append(self.btn_fwd)

        self.btn_reload = Gtk.Button.new_from_icon_name('view-refresh-symbolic')
        self.btn_reload.set_tooltip_text('Reload')
        self.btn_reload.connect('clicked', lambda _: self.tabs.reload())
        left.append(self.btn_reload)

        btn_new = Gtk.Button.new_from_icon_name('tab-new-symbolic')
        btn_new.set_tooltip_text('New Tab')
        btn_new.connect('clicked', self._on_new_tab)
        left.append(btn_new)

        hb.pack_start(left)

        # ── Centre: URL bar ───────────────────────────────────────────────
        self.url_entry = Gtk.Entry()
        self.url_entry.set_hexpand(True)
        self.url_entry.set_placeholder_text('Enter URL or search…')
        self.url_entry.connect('activate', self._on_url_activate)
        hb.set_title_widget(self.url_entry)

        # ── Right: mode + reader + agent ──────────────────────────────────
        right = Gtk.Box(spacing=6)

        mode_strings = Gtk.StringList.new(
            [get_mode_label(m) for m in MODES]
        )
        self.mode_drop = Gtk.DropDown.new(mode_strings, None)
        self.mode_drop.set_selected(mode_index(
            self.config.get('browser', 'default_mode', 'normal')
        ))
        self.mode_drop.set_tooltip_text('Page mode')
        self.mode_drop.connect('notify::selected', self._on_mode_changed)
        right.append(self.mode_drop)

        btn_reader = Gtk.Button(label='📖')
        btn_reader.set_tooltip_text('Reader Mode')
        btn_reader.connect('clicked', self._on_reader)
        right.append(btn_reader)

        self.btn_agent = Gtk.Button(label='🤖 Resumir')
        self.btn_agent.set_tooltip_text('Summarize with Bauer Agent')
        self.btn_agent.connect('clicked', self._on_agent)
        right.append(self.btn_agent)

        hb.pack_end(right)

    def _build_agent_panel(self) -> Gtk.Box:
        panel = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        panel.set_margin_start(12)
        panel.set_margin_end(12)
        panel.set_margin_bottom(8)
        panel.set_visible(False)

        header = Gtk.Box()
        lbl = Gtk.Label(label='🤖 Bauer Agent — Resumo')
        lbl.set_hexpand(True)
        lbl.set_xalign(0.0)
        header.append(lbl)
        btn_close = Gtk.Button(label='✕')
        btn_close.set_has_frame(False)
        btn_close.connect('clicked', lambda _: panel.set_visible(False))
        header.append(btn_close)
        panel.append(header)

        scroll = Gtk.ScrolledWindow()
        scroll.set_min_content_height(120)
        scroll.set_max_content_height(220)
        self.agent_text = Gtk.TextView()
        self.agent_text.set_editable(False)
        self.agent_text.set_wrap_mode(Gtk.WrapMode.WORD)
        self.agent_text.set_left_margin(8)
        self.agent_text.set_right_margin(8)
        scroll.set_child(self.agent_text)
        panel.append(scroll)

        return panel

    # ── Event handlers ────────────────────────────────────────────────────────

    def _on_url_activate(self, entry):
        url = entry.get_text().strip()
        if url:
            self.tabs.navigate(url)

    def _on_new_tab(self, _btn):
        if not self.tabs.can_open():
            self._alert(f'Máximo de {self.tabs.max_tabs} abas atingido.')
            return
        home = self.config.get('browser', 'home_url', 'https://duckduckgo.com')
        self.tabs.new_tab(home)

    def _on_mode_changed(self, drop, _param):
        if self._updating_dropdown:
            return
        idx = drop.get_selected()
        mode = MODES[idx] if idx < len(MODES) else 'normal'
        info = self.tabs.current_tab()
        if info:
            self.tabs.set_mode(info.tab_id, mode)

    def _on_reader(self, _btn):
        wv = self.tabs.current_webview()
        if wv:
            activate_reader(wv)

    def _on_agent(self, _btn):
        if not self.agent.is_available:
            self._show_agent('Bauer Agent não está disponível.\n\n'
                             f'Inicie o agente em {self.agent.base_url}')
            return
        wv = self.tabs.current_webview()
        if not wv:
            return
        self.btn_agent.set_sensitive(False)
        self.btn_agent.set_label('⏳ Processando…')
        self.logger.log_agent_request(wv.get_uri() or '')
        self.agent.summarize_page(
            wv,
            on_result=self._show_agent,
            on_error=lambda e: self._show_agent(f'Erro: {e}'),
        )

    def _on_tab_switch(self, _nb, _page, _page_num):
        info = self.tabs.current_tab()
        if not info:
            return
        self.url_entry.set_text(info.webview.get_uri() or '')
        title = info.webview.get_title() or 'Bauer Browser'
        self.set_title(f'{title} — Bauer Browser')
        self._updating_dropdown = True
        self.mode_drop.set_selected(mode_index(info.mode))
        self._updating_dropdown = False
        self._refresh_nav()

    # ── Sync callbacks ────────────────────────────────────────────────────────

    def _sync_url(self, tab_id: str, uri: str):
        info = self.tabs.current_tab()
        if info and info.tab_id == tab_id:
            self.url_entry.set_text(uri)
            self._refresh_nav()

    def _sync_title(self, tab_id: str, title: str):
        info = self.tabs.current_tab()
        if info and info.tab_id == tab_id:
            self.set_title(f'{title} — Bauer Browser')

    # ── Helpers ───────────────────────────────────────────────────────────────

    def _refresh_nav(self):
        wv = self.tabs.current_webview()
        if wv:
            self.btn_back.set_sensitive(wv.can_go_back())
            self.btn_fwd.set_sensitive(wv.can_go_forward())

    def _refresh_agent_btn(self) -> bool:
        if self.agent.is_available:
            tip = f'Resumir com Bauer Agent ({self.agent.base_url})'
        else:
            tip = f'Bauer Agent indisponível ({self.agent.base_url})'
        self.btn_agent.set_tooltip_text(tip)
        return True

    def _show_agent(self, text: str):
        self.agent_text.get_buffer().set_text(text)
        self.agent_panel.set_visible(True)
        self.btn_agent.set_sensitive(True)
        self.btn_agent.set_label('🤖 Resumir')

    def _alert(self, message: str):
        dialog = Gtk.MessageDialog(
            transient_for=self,
            modal=True,
            message_type=Gtk.MessageType.INFO,
            buttons=Gtk.ButtonsType.OK,
            text=message,
        )
        dialog.connect('response', lambda d, _: d.destroy())
        dialog.present()
