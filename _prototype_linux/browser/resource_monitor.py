"""Resource monitor — polls /proc for WebKit process RAM usage."""

from typing import Callable, Optional
from .compat import GLib


def read_ram_mb(pid: int) -> Optional[float]:
    """Read RSS memory for a PID from /proc. Returns MB or None on error."""
    try:
        with open(f'/proc/{pid}/status', 'r') as f:
            for line in f:
                if line.startswith('VmRSS:'):
                    # "VmRSS:   12345 kB"
                    return int(line.split()[1]) / 1024.0
    except (FileNotFoundError, PermissionError, ValueError, IndexError):
        pass
    return None


class ResourceMonitor:
    """Polls RAM usage per tab using GLib.timeout_add."""

    def __init__(self, interval_ms: int = 2000):
        self.interval_ms = interval_ms
        self._watchers: dict[str, dict] = {}
        self._running = False

    def register(
        self,
        tab_id: str,
        pid_getter: Callable[[], Optional[int]],
        on_update: Callable[[str, float], None],
    ) -> None:
        """Register a tab for monitoring.

        pid_getter: callable returning the WebKit process PID (may change over time)
        on_update: called in the GLib main loop with (tab_id, ram_mb)
        """
        self._watchers[tab_id] = {
            'pid_getter': pid_getter,
            'on_update': on_update,
            'last_ram': 0.0,
        }

    def unregister(self, tab_id: str) -> None:
        self._watchers.pop(tab_id, None)

    def start(self) -> None:
        if self._running:
            return
        self._running = True
        GLib.timeout_add(self.interval_ms, self._poll)

    def stop(self) -> None:
        self._running = False

    def get_ram(self, tab_id: str) -> float:
        return self._watchers.get(tab_id, {}).get('last_ram', 0.0)

    def _poll(self) -> bool:
        if not self._running:
            return False  # Remove timeout

        for tab_id, w in list(self._watchers.items()):
            try:
                pid = w['pid_getter']()
                if pid:
                    ram = read_ram_mb(pid)
                    if ram is not None:
                        w['last_ram'] = ram
                        w['on_update'](tab_id, ram)
            except Exception:
                pass

        return True  # Keep polling
