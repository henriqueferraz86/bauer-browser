"""Navigation and resource logger."""

import datetime
from pathlib import Path


class BrowserLogger:
    def __init__(self, config):
        self.enabled: bool = config.get('logging', 'enabled', True)
        log_file = config.get(
            'logging', 'log_file',
            str(Path.home() / '.bauer-browser' / 'navigation.log')
        )
        self.log_path = Path(log_file)
        if self.enabled:
            self.log_path.parent.mkdir(parents=True, exist_ok=True)

    def _write(self, parts: list[str]) -> None:
        if not self.enabled:
            return
        try:
            line = ' | '.join(parts) + '\n'
            with open(self.log_path, 'a', encoding='utf-8') as f:
                f.write(line)
        except Exception as e:
            print(f'[logger] Error: {e}')

    def _now(self) -> str:
        return datetime.datetime.now().isoformat(timespec='seconds')

    def log_navigation(self, url: str, mode: str, ram_mb: float = 0.0) -> None:
        parts = [self._now(), 'NAV', f'mode={mode}', f'url={url}']
        if ram_mb > 0:
            parts.append(f'ram={ram_mb:.1f}MB')
        self._write(parts)

    def log_mode_change(self, tab_id: str, old_mode: str, new_mode: str) -> None:
        self._write([self._now(), 'MODE', f'tab={tab_id}', f'{old_mode}->{new_mode}'])

    def log_agent_request(self, url: str) -> None:
        self._write([self._now(), 'AGENT_REQ', f'url={url}'])

    def log_resource(self, tab_id: str, ram_mb: float, mode: str) -> None:
        self._write([self._now(), 'RESOURCE', f'tab={tab_id}',
                     f'mode={mode}', f'ram={ram_mb:.1f}MB'])
