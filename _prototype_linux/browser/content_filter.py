"""Content filtering — loads WebKit Content Blocker rules from JSON files."""

from pathlib import Path

from .compat import WebKit, GLib

BASE_DIR = Path(__file__).parent.parent
FILTER_CACHE = Path.home() / '.bauer-browser' / 'filter-cache'


class ContentFilterManager:
    """Manages WebKit UserContentFilter objects loaded from JSON blocklists."""

    def __init__(self, config):
        self._config = config
        self._filters: dict[str, object] = {}  # filter_id -> WebKit.UserContentFilter
        FILTER_CACHE.mkdir(parents=True, exist_ok=True)
        self._store = WebKit.UserContentFilterStore.new(str(FILTER_CACHE))
        self._load_all()

    def _load_all(self):
        blocklists_dir = BASE_DIR / 'assets' / 'blocklists'
        to_load = []

        if self._config.get('blocklists', 'block_trackers', True):
            to_load.append(('bauer-trackers', blocklists_dir / 'trackers.json'))
        if self._config.get('blocklists', 'block_ads', True):
            to_load.append(('bauer-ads', blocklists_dir / 'ads.json'))

        for filter_id, path in to_load:
            if path.exists():
                self._save_filter(filter_id, path)
            else:
                print(f'[filter] Blocklist not found: {path}')

    def _save_filter(self, filter_id: str, path: Path):
        try:
            data = path.read_bytes()
        except OSError as e:
            print(f'[filter] Cannot read {path}: {e}')
            return

        def on_saved(store, result, _user_data):
            try:
                cf = store.save_finish(result)
                self._filters[filter_id] = cf
                print(f'[filter] Loaded: {filter_id}')
            except Exception as e:
                print(f'[filter] Save error for {filter_id}: {e}')

        self._store.save(filter_id, GLib.Bytes.new(data), None, on_saved, None)

    def apply_to_webview(self, webview, mode: str) -> None:
        """Apply relevant filters to a WebView. Full mode skips filtering."""
        manager = webview.get_user_content_manager()
        manager.remove_all_filters()

        if mode == 'full':
            return

        for cf in self._filters.values():
            try:
                manager.add_filter(cf)
            except Exception as e:
                print(f'[filter] Apply error: {e}')


_instance: ContentFilterManager | None = None


def get_filter_manager(config=None) -> ContentFilterManager | None:
    global _instance
    if _instance is None and config is not None:
        _instance = ContentFilterManager(config)
    return _instance
