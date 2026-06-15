"""Configuration manager — reads TOML with defaults fallback."""

import sys
from pathlib import Path

if sys.version_info >= (3, 11):
    import tomllib
else:
    try:
        import tomllib
    except ImportError:
        try:
            import tomli as tomllib  # type: ignore
        except ImportError:
            tomllib = None  # type: ignore

BASE_DIR = Path(__file__).parent.parent

DEFAULTS: dict = {
    'browser': {
        'max_tabs': 5,
        'default_mode': 'normal',
        'home_url': 'https://www.google.com',
        'ram_alert_mb': 300,
        'idle_tab_timeout_seconds': 0,
        'window_width': 1200,
        'window_height': 800,
    },
    'blocklists': {
        'block_trackers': True,
        'block_ads': True,
    },
    'agent': {
        'enabled': True,
        'base_url': 'http://localhost:8742',
        'timeout_seconds': 10,
    },
    'logging': {
        'enabled': True,
        'log_file': str(Path.home() / '.bauer-browser' / 'navigation.log'),
    },
}


class ConfigManager:
    def __init__(self):
        self._cfg: dict = {}
        # Deep-copy defaults
        for section, values in DEFAULTS.items():
            self._cfg[section] = dict(values)
        self._load_user_config()

    def _load_user_config(self):
        path = BASE_DIR / 'config' / 'settings.toml'
        if not path.exists():
            return
        if tomllib is None:
            print('[config] Warning: tomllib not available, using defaults only.')
            return
        try:
            with open(path, 'rb') as f:
                user = tomllib.load(f)
            for section, values in user.items():
                if section in self._cfg and isinstance(values, dict):
                    self._cfg[section].update(values)
                else:
                    self._cfg[section] = values
        except Exception as e:
            print(f'[config] Could not load settings.toml: {e}')

    def get(self, section: str, key: str, default=None):
        return self._cfg.get(section, {}).get(key, default)

    def section(self, name: str) -> dict:
        return self._cfg.get(name, {})
