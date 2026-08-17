#!/usr/bin/env python3
"""Validate module configuration files in platform/iso/calamares/modules/."""
import os
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
MODULES = os.path.normpath(os.path.join(HERE, os.pardir, "modules"))


class TestModuleConfs(unittest.TestCase):
    def test_welcome_conf(self):
        path = os.path.join(MODULES, "welcome", "welcome.conf")
        self.assertTrue(os.path.isfile(path), "welcome.conf missing")
        with open(path) as f:
            text = f.read()
        self.assertIn("requirements:", text)
        self.assertIn("requiredStorage: 20.0", text)
        self.assertIn("requiredRam: 4.0", text)

    def test_partition_conf(self):
        path = os.path.join(MODULES, "partition", "partition.conf")
        self.assertTrue(os.path.isfile(path), "partition.conf missing")
        with open(path) as f:
            text = f.read()
        self.assertIn("defaultFileSystemType: \"ext4\"", text)
        self.assertIn("defaultPartitionTableType: \"gpt\"", text)
        self.assertIn("initialPartitioningChoice: erase", text)

    def test_locale_conf(self):
        path = os.path.join(MODULES, "locale", "locale.conf")
        self.assertTrue(os.path.isfile(path), "locale.conf missing")
        with open(path) as f:
            text = f.read()
        self.assertIn("region:", text)
        self.assertIn("zone:", text)

    def test_keyboard_conf(self):
        path = os.path.join(MODULES, "keyboard", "keyboard.conf")
        self.assertTrue(os.path.isfile(path), "keyboard.conf missing")
        with open(path) as f:
            text = f.read()
        self.assertIn("displayStyle:", text)
        self.assertIn("guessLayout: true", text)

    def test_users_conf(self):
        path = os.path.join(MODULES, "users", "users.conf")
        self.assertTrue(os.path.isfile(path), "users.conf missing")
        with open(path) as f:
            text = f.read()
        self.assertIn("setRootPassword: true", text)
        self.assertIn("sudoersGroup: wheel", text)

    def test_finished_conf(self):
        path = os.path.join(MODULES, "finished", "finished.conf")
        self.assertTrue(os.path.isfile(path), "finished.conf missing")
        with open(path) as f:
            text = f.read()
        self.assertIn("restartNowEnabled: true", text)
        self.assertIn("restartNowChecked: true", text)


if __name__ == "__main__":
    unittest.main()
