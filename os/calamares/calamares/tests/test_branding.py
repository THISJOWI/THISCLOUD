#!/usr/bin/env python3
"""Tests for make-calamares-branding.py — PNG generation, stdlib only."""
import os
import struct
import subprocess
import sys
import tempfile
import unittest
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, os.pardir, "scripts", "make-calamares-branding.py")


class PngHelpers(unittest.TestCase):
    def png_size(self, path):
        with open(path, "rb") as f:
            data = f.read()
        self.assertTrue(data[:8] == b"\x89PNG\r\n\x1a\n", "bad PNG signature")
        width, height = struct.unpack(">II", data[16:24])
        return width, height

    def png_crc_ok(self, path):
        with open(path, "rb") as f:
            data = f.read()
        pos, n = 8, len(data)
        while pos < n:
            length, = struct.unpack(">I", data[pos:pos + 4])
            tag = data[pos + 4:pos + 8]
            crc, = struct.unpack(">I", data[pos + 8 + length:pos + 12 + length])
            expect = zlib.crc32(data[pos + 4:pos + 8 + length]) & 0xffffffff
            if crc != expect:
                return False
            pos += 12 + length
        return True


class TestGenerator(PngHelpers):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="calamares-branding-")
        self.addCleanup(lambda: _rmtree(self.tmp))
        self.rc = subprocess.run(
            [sys.executable, SCRIPT, self.tmp], capture_output=True, text=True
        )

    def test_exit_zero(self):
        self.assertEqual(self.rc.returncode, 0, self.rc.stderr)

    def test_expected_files(self):
        for name in ("productIcon.png", "productLogo.png", "productWelcome.png",
                     "wallpaper.png", "sidebar-bg.png"):
            self.assertTrue(os.path.isfile(os.path.join(self.tmp, name)), name)

    def test_sizes(self):
        self.assertEqual(self.png_size(os.path.join(self.tmp, "productIcon.png")), (128, 128))
        self.assertEqual(self.png_size(os.path.join(self.tmp, "productLogo.png")), (80, 80))
        self.assertEqual(self.png_size(os.path.join(self.tmp, "productWelcome.png")), (320, 150))
        self.assertEqual(self.png_size(os.path.join(self.tmp, "wallpaper.png")), (800, 520))

    def test_slides(self):
        for i in range(1, 5):
            p = os.path.join(self.tmp, "slides", f"slide-{i}.png")
            self.assertTrue(os.path.isfile(p), p)
            self.assertEqual(self.png_size(p), (800, 480))

    def test_crc(self):
        for name in ("productIcon.png", "wallpaper.png"):
            self.assertTrue(self.png_crc_ok(os.path.join(self.tmp, name)), name)


def _rmtree(p):
    import shutil
    shutil.rmtree(p, ignore_errors=True)


if __name__ == "__main__":
    unittest.main()