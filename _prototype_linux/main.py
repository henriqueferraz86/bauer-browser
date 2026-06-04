#!/usr/bin/env python3
"""Bauer Browser — MVP Entry Point."""

import sys

import gi
gi.require_version('Gtk', '4.0')

from gi.repository import Gtk, GLib
from browser.config_manager import ConfigManager
from browser.window import BrowserWindow


class BauerBrowser(Gtk.Application):
    def __init__(self):
        super().__init__(application_id='os.bauer.browser')
        self.config = ConfigManager()

    def do_activate(self):
        existing = self.get_windows()
        if existing:
            existing[0].present()
            return
        win = BrowserWindow(application=self, config=self.config)
        win.present()


def main():
    app = BauerBrowser()
    return app.run(sys.argv)


if __name__ == '__main__':
    sys.exit(main())
