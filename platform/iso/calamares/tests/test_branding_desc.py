#!/usr/bin/env python3
"""Validate thiscloud branding.desc YAML structure."""
import os
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
BRANDING = os.path.join(HERE, os.pardir, "branding", "thiscloud")
BRANDING = os.path.normpath(BRANDING)


class TestBrandingDesc(unittest.TestCase):
    def test_file_exists(self):
        self.assertTrue(os.path.isfile(os.path.join(BRANDING, "branding.desc")))

    def test_minimal_yaml_keys(self):
        # No PyYAML guaranteed — do a lightweight structural check of required keys.
        with open(os.path.join(BRANDING, "branding.desc")) as f:
            text = f.read()
        for key in ("componentName:", "strings:", "productName:", "shortVersionedName:",
                    "images:", "productIcon:", "productLogo:", "productWelcome:",
                    "style:", "SidebarBackground:", "slideshow:", "welcomeStyleCalamares:"):
            self.assertIn(key, text, f"missing key {key}")

    def test_required_pngs_referenced(self):
        # Calamares images: keys are productBanner/productIcon/productLogo/
        # productWallpaper/productWelcome; SidebarBackground is a color, so
        # sidebar-bg.png is generated but never referenced by branding.desc.
        with open(os.path.join(BRANDING, "branding.desc")) as f:
            text = f.read()
        for img in ("productIcon.png", "productLogo.png", "productWelcome.png",
                    "wallpaper.png"):
            self.assertIn(img, text)
            self.assertTrue(os.path.isfile(os.path.join(BRANDING, img)), img)

    def test_sidebar_bg_exists(self):
        self.assertTrue(os.path.isfile(os.path.join(BRANDING, "sidebar-bg.png")))


if __name__ == "__main__":
    unittest.main()