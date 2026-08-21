#!/usr/bin/env python3
"""Validate the main Calamares settings.conf sequence."""
import os
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SETTINGS = os.path.normpath(os.path.join(HERE, os.pardir, "settings.conf"))


class TestSettingsConf(unittest.TestCase):
    def test_exists(self):
        self.assertTrue(os.path.isfile(SETTINGS))

    def test_sequence_contains_required_modules(self):
        text = open(SETTINGS).read()
        for mod in ("welcome", "locale", "keyboard", "timezone", "partition", "users",
                    "network", "thiscloudqml", "summary", "finished", "thiscloud",
                    "mount", "unpackfs", "fstab", "bootloader", "umount"):
            self.assertIn(mod, text)

    def test_thiscloud_in_exec(self):
        text = open(SETTINGS).read()
        exec_section = text.split("- exec:")[1].split("- show:")[0]
        self.assertIn("thiscloud", exec_section)

    def test_thiscloudqml_in_show(self):
        text = open(SETTINGS).read()
        show_section = text.split("- show:")[1].split("- exec:")[0]
        self.assertIn("thiscloudqml", show_section)


if __name__ == "__main__":
    unittest.main()