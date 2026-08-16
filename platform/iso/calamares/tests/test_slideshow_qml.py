#!/usr/bin/env python3
"""Lightweight structural checks for the slideshow QML."""
import os
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
QML = os.path.normpath(os.path.join(HERE, os.pardir, "branding", "thiscloud", "show.qml"))


class TestSlideshowQml(unittest.TestCase):
    def test_exists(self):
        self.assertTrue(os.path.isfile(QML))

    def test_imports_calamares(self):
        text = open(QML).read()
        self.assertIn("import io.calamares.core", text)
        self.assertIn("import QtQuick", text)

    def test_api2_hooks(self):
        text = open(QML).read()
        self.assertIn("onActivate", text)
        self.assertIn("onLeave", text)

    def test_slides_referenced(self):
        text = open(QML).read()
        for i in range(1, 5):
            self.assertIn(f"slide-{i}.png", text)


if __name__ == "__main__":
    unittest.main()