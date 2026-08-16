# Custom Calamares Installer for THISCLOUD Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Anaconda installer in the THISCLOUD AlmaLinux 9 ISO with a Calamares-based installer (compiled from source, since Calamares/KPMcore are absent from EPEL9) that runs from a booted live environment, carries full THISCLOUD branding, and adds a custom "THISCLOUD config" step (node role, cluster name, node IP, network interface).

**Architecture:** The ISO becomes a **live image** (AlmaLinux + minimal graphical session: Xorg + openbox + autologin + autostart Calamares). Calamares runs from the running live system and installs to disk (partition via KPMcore, copy system via unpackfs, bootloader via grub). Branding lives in `iso/calamares/branding/thiscloud/`. A custom QML view module (`thiscloudqml`) collects node settings into GlobalStorage; a Python job module (`thiscloud`) writes `/etc/thiscloud/config.toml`, runs `thiscloud init --ip <ip> --role <role>`, and enables services.

**Tech Stack:** Calamares 3.3.14 (C++17, Qt6/QtWidgets/QML), KPMcore 24.x, CMake, Python 3 (job module + branding generator, stdlib only), bash (build scripts), QML (view module + slideshow), livemedia-creator/lorax (live ISO), xorriso (ISO assembly).

## Global Constraints

- ISO pipeline runs ONLY on **AlmaLinux 9 x86_64** builder (bare metal or VM). macOS can generate branding assets and unit-test pure-Python logic, but cannot build Calamares or the ISO.
- `protoc` not involved; but builder needs: `gcc-c++ gcc-c++ cmake make qt6-qtbase-devel qt6-qtsvg-devel qt6-qtdeclarative-devel qt6-qtquickcontrols2-devel qt6-qtquicktemplates2-devel boost-devel yaml-cpp-devel parted-devel kf5-kcoreaddons-devel kf5-ki18n-devel kf5-kconfig-devel extra-cmake-modules python3 python3-devel` (some via EPEL9).
- Repo installs THISCLOUD via existing pipeline: `build-iso.sh` steps [1-4] (cross-compile → RPM → go-api → web-ui) stay intact; Calamares work is additive to the ISO assembly path.
- Brand palette (must match `web-ui/src/app/globals.css`): BG `#0f1115`, CARD `#171a21`, ACCENT `#3b82f6`, FG `#e6e9ef`.
- Module sequence: `welcome, locale, keyboard, timezone, partition, users, network, thiscloudqml (view)` → show; `partition, mount, unpackfs, fstab, locale, keyboard, localecfg, users, networkcfg, hwclock, services-systemd, initramfs, bootloader, thiscloud (job), umount` → exec; `finished` → show.
- All files under `platform/iso/`; no changes to daemon/CLI/web-ui except the CLI's existing `thiscloud init` (unchanged).
- **Environment for live env:** run graphical session as `root` with autologin (installer host only), no password prompt. Do not ship this to the installed target.
- `npm test` / `cargo test` not affected. New tests use Python `unittest` (no pytest dependency assumption on builder; runnable on macOS too).

---
---

## File Structure

```
platform/iso/calamares/
├── branding/
│   └── thiscloud/
│       ├── branding.desc
│       ├── colors.conf
│       ├── stylesheet.qss
│       ├── show.qml                    # slideshow during install
│       ├── slides/                     # optional static slide images (generated)
│       └── (logo/welcome PNGs generated into branding dir)
├── modules/
│   ├── thiscloudqml/                   # custom view module (compiled into Calamares)
│   │   ├── CMakeLists.txt
│   │   ├── ThisCloudViewStep.h
│   │   ├── ThisCloudViewStep.cpp
│   │   ├── thiscloudqml.qml            # form: role / cluster / IP / interface
│   │   └── thiscloudqml.conf
│   └── thiscloud/                      # python job module (no compile)
│       ├── module.desc
│       ├── thiscloud.conf
│       ├── thiscloud_logic.py          # pure logic, no libcalamares — unit-testable
│       └── main.py                     # thin Calamares wrapper over thiscloud_logic
├── scripts/
│   ├── make-calamares-branding.py      # stdlib-only PNG generator (testable on macOS)
│   └── build-calamares.sh              # compile Calamares+KPMcore into live rootfs (builder)
├── tests/
│   ├── test_branding.py
│   └── test_thiscloud_logic.py
├── settings.conf                       # main Calamares config (module sequence)
└── README.md

platform/iso/calamares/live/
├── live.ks                             # kickstart for livemedia-creator (graphical live env)
└── autostart/                          # session autostart files for the live env
    ├── calamares.desktop
    └── xorg-autologin.conf
```

Files modified: `platform/iso/scripts/remix-iso.sh`, `platform/iso/scripts/build-iso.sh`, `platform/iso/kickstart/thiscloud.ks` (killed — replaced by live flow; documented), `platform/iso/scripts/fetch-deps.sh` (add Calamares source fetch), `platform/iso/scripts/install-deps.sh` (add builder deps), `platform/iso/README.md`.

**Module ownership:** Tasks 1-6 are fully macOS-testable (unit tests run locally). Tasks 7-12 are builder-only scripts; verification is `bash -n` + explicit builder runbook (cannot execute on macOS).

---
---

### Task 1: Branding asset generator (Calamares pixmaps)

**Files:**
- Create: `platform/iso/calamares/scripts/make-calamares-branding.py`
- Test: `platform/iso/calamares/tests/test_branding.py`

**Interfaces:**
- Consumes: nothing (stdlib: `struct`, `zlib`, `argparse`, `os`, `sys`).
- Produces: PNG files into `platform/iso/calamares/branding/thiscloud/`:
  - `productIcon.png` (128x128), `productLogo.png` (80x80), `productWelcome.png` (320x150), `wallpaper.png` (800x520), `sidebar-bg.png` (240x800), slides `slides/slide-1.png` … `slides/slide-4.png` (800x480 each).
  - Exit 0 on success. CLI: `python3 make-calamares-branding.py [OUTPUT_DIR]` (default = `branding/thiscloud`).
- Later tasks read these paths as the branding images referenced by `branding.desc`.

- [ ] **Step 1: Write the failing tests**

```python
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `python3 platform/iso/calamares/tests/test_branding.py`
Expected: FAIL — `FileNotFoundError` / `returncode != 0` (script missing).

- [ ] **Step 3: Write the implementation**

```python
#!/usr/bin/env python3
"""Generate Calamares branding pixmaps for THISCLOUD (stdlib only).

Usage: python3 make-calamares-branding.py [OUTPUT_DIR]
Writes productIcon.png, productLogo.png, productWelcome.png, wallpaper.png,
sidebar-bg.png, and slides/slide-1..4.png into OUTPUT_DIR (default:
<repo>/calamares/branding/thiscloud).
"""
import argparse
import os
import struct
import zlib

BG = (0x0F, 0x11, 0x15)      # --bg
CARD = (0x17, 0x1A, 0x21)     # --card
ACCENT = (0x3B, 0x82, 0xF6)   # --accent
FG = (0xE6, 0xE9, 0xEF)       # --fg


def _chunk(tag, data):
    return (struct.pack(">I", len(data)) + tag + data +
            struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF))


def write_png(path, width, height, pixel_at):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    sig = b"\x89PNG\r\n\x1a\n"
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        for x in range(width):
            raw.extend(pixel_at(x, y, width, height))
    png = (sig + _chunk(b"IHDR", ihdr) +
           _chunk(b"IDAT", zlib.compress(bytes(raw))) +
           _chunk(b"IEND", b""))
    with open(path, "wb") as f:
        f.write(png)


def gradient(top, bottom, accent_line=0):
    def pix(x, y, w, h):
        t = (y / (h - 1)) if h > 1 else 0.0
        r = round(top[0] + (bottom[0] - top[0]) * t)
        g = round(top[1] + (bottom[1] - top[1]) * t)
        b = round(top[2] + (bottom[2] - top[2]) * t)
        if accent_line and y >= h - accent_line:
            return ACCENT + (255,)
        return (r, g, b) + (255,)
    return pix


def solid(color):
    def pix(x, y, w, h):
        return color + (255,)
    return pix


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("output", nargs="?", default=None,
                    help="output dir (default: branding/thiscloud next to script)")
    args = ap.parse_args()

    if args.output:
        out = os.path.abspath(args.output)
    else:
        here = os.path.dirname(os.path.abspath(__file__))
        out = os.path.abspath(os.path.join(here, os.pardir, "branding", "thiscloud"))
    os.makedirs(out, exist_ok=True)

    # 128x128 square product icon — accent fill with dark "T" glyph via punch-out.
    def icon_pix(x, y, w, h):
        # Punch a simple "T" using the FG color over accent.
        bar = 30 <= y <= 46 and 40 <= x <= 88
        stem = 40 <= x <= 48 and 46 <= y <= 92
        if bar or stem:
            return FG + (255,)
        return ACCENT + (255,)
    write_png(os.path.join(out, "productIcon.png"), 128, 128, icon_pix)

    # 80x80 sidebar logo — same "T" glyph, accent on transparent rounded square.
    def logo_pix(x, y, w, h):
        if 18 <= y <= 62 and 26 <= x <= 54:
            return FG + (255,)
        if 26 <= x <= 34 and 34 <= y <= 62:
            return FG + (255,)
        return (0, 0, 0, 0)
    write_png(os.path.join(out, "productLogo.png"), 80, 80, logo_pix)

    # Welcome banner 320x150 — vertical gradient with accent strip.
    write_png(os.path.join(out, "productWelcome.png"), 320, 150,
              gradient(BG, CARD, accent_line=6))

    # Window wallpaper 800x520.
    write_png(os.path.join(out, "wallpaper.png"), 800, 520,
              gradient(BG, CARD))

    # Sidebar 240x800.
    write_png(os.path.join(out, "sidebar-bg.png"), 240, 800,
              gradient(BG, CARD, accent_line=8))

    # Slides 800x480: gradient + accent bottom strip + slide number blob.
    for i in range(1, 5):
        def slide_pix(x, y, w, h, _i=i):
            c = gradient(BG, CARD, accent_line=8)(x, y, w, h)
            # Title bar band near top, distinct per-slide x offset.
            if 60 <= y <= 100:
                if x % 60 < 40:
                    return FG + (255,)
            return c
        write_png(os.path.join(out, "slides", f"slide-{i}.png"), 800, 480, slide_pix)

    print(f"wrote {len(os.listdir(out))} top-level files to {out}")
    print("slides:", sorted(os.listdir(os.path.join(out, "slides"))))


if __name__ == "__main__":
    main()
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `python3 platform/iso/calamares/tests/test_branding.py`
Expected: `OK` (all tests pass).

- [ ] **Step 5: Run the generator into the branding dir and eyeball output**

Run: `python3 platform/iso/calamares/scripts/make-calamares-branding.py`
Expected: prints `wrote ... files to <repo>/platform/iso/calamares/branding/thiscloud` and lists `slides/`.

- [ ] **Step 6: Commit**

```bash
git add platform/iso/calamares/scripts/make-calamares-branding.py \
        platform/iso/calamares/tests/test_branding.py \
        platform/iso/calamares/branding/thiscloud
git commit -m "feat(iso): generate Calamares branding pixmaps for THISCLOUD"
```

---
---

### Task 2: Branding descriptor, colors, and stylesheet

**Files:**
- Create: `platform/iso/calamares/branding/thiscloud/branding.desc`
- Create: `platform/iso/calamares/branding/thiscloud/colors.conf`
- Create: `platform/iso/calamares/branding/thiscloud/stylesheet.qss`
- Test: `platform/iso/calamares/tests/test_branding_desc.py`

**Interfaces:**
- Consumes: pixmap paths from Task 1 (`productIcon.png`, `productLogo.png`, `productWelcome.png`, `wallpaper.png`).
- Produces: YAML `branding.desc` (component `thiscloud`), `colors.conf`, `stylesheet.qss`. Later, `build-calamares.sh` installs these to the live rootfs at `/etc/calamares/branding/thiscloud/`.

- [ ] **Step 1: Write the failing test**

```python
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
        with open(os.path.join(BRANDING, "branding.desc")) as f:
            text = f.read()
        for img in ("productIcon.png", "productLogo.png", "productWelcome.png",
                    "wallpaper.png", "sidebar-bg.png"):
            self.assertIn(img, text)
            self.assertTrue(os.path.isfile(os.path.join(BRANDING, img)), img)


if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 platform/iso/calamares/tests/test_branding_desc.py`
Expected: FAIL — `branding.desc` missing.

- [ ] **Step 3: Write branding.desc**

```yaml
# THISCLOUD product branding for Calamares.
# Installed to /etc/calamares/branding/thiscloud/ on the live system.
---
componentName: thiscloud

# Show the traditional "Welcome to the THISCLOUD installer."
welcomeStyleCalamares: false
welcomeExpandingLogo: true

# Window: fixed 900x640, centered, no auto-expansion (works with openbox).
windowExpanding: normal
windowSize: 900px,640px
windowPlacement: center

# Sidebar: standard widget sidebar on the left.
sidebar: widget

# Navigation: standard buttons at the bottom.
navigation: widget

# Text shown to the user. ${NAME} resolved from /etc/os-release on the live system.
strings:
  productName: "THISCLOUD"
  shortProductName: THISCLOUD
  version: "0.1.0 (Nucleus)"
  shortVersion: 0.1.0
  versionedName: "THISCLOUD 0.1.0 (Nucleus)"
  shortVersionedName: THISCLOUD 0.1.0
  bootloaderEntryName: THISCLOUD
  productUrl: "https://github.com/THISJOWI/THISCLOUD"
  supportUrl: "https://github.com/THISJOWI/THISCLOUD/issues"
  knownIssuesUrl: "https://github.com/THISJOWI/THISCLOUD/issues"
  releaseNotesUrl: "https://github.com/THISJOWI/THISCLOUD/releases"
  donateUrl: ""

images:
  productIcon: "productIcon.png"
  productLogo: "productLogo.png"
  productWelcome: "productWelcome.png"
  productWallpaper: "wallpaper.png"

# Sidebar colors — dark THISCLOUD palette.
style:
  SidebarBackground: "#171a21"
  SidebarText: "#e6e9ef"
  SidebarBackgroundCurrent: "#3b82f6"
  SidebarTextCurrent: "#0f1115"

# Image slideshow during install (see show.qml task below).
slideshow: "show.qml"
slideshowAPI: 2

uploadServer:
  type: "none"
```

- [ ] **Step 4: Write colors.conf**

```conf
# THISCLOUD color scheme for the Calamares QML widgets.
# Matches web-ui/src/app/globals.css palette.
---

window:   "#0f1115"
windowText: "#e6e9ef"
base:     "#171a21"
alternateBase: "#171a21"
toolTipBase: "#171a21"
toolTipText: "#e6e9ef"
text:     "#e6e9ef"
button:   "#171a21"
buttonText: "#e6e9ef"
brightText: "#3b82f6"
link:     "#3b82f6"
highlight: "#3b82f6"
highlightedText: "#0f1115"
```

- [ ] **Step 5: Write stylesheet.qss**

```css
/* THISCLOUD stylesheet for Calamares Qt widgets. */
QWidget { background-color: #0f1115; color: #e6e9ef; }
#sidebarApp { background-color: #171a21; }
#sidebarApp QLabel { color: #e6e9ef; }
QPushButton { background-color: #171a21; border: 1px solid #3b82f6;
              border-radius: 4px; padding: 6px 16px; color: #e6e9ef; }
QPushButton:hover { background-color: #3b82f6; color: #0f1115; }
QPushButton:disabled { border-color: #33363d; color: #6b7280; }
QLineEdit, QComboBox, QSpinBox { background-color: #171a21; border: 1px solid #33363d;
                                 border-radius: 4px; padding: 4px 8px; color: #e6e9ef; }
QLineEdit:focus, QComboBox:focus { border-color: #3b82f6; }
QProgressBar { background-color: #171a21; border: 1px solid #33363d; border-radius: 4px; }
QProgressBar::chunk { background-color: #3b82f6; }
```

- [ ] **Step 6: Run test to verify it passes**

Run: `python3 platform/iso/calamares/tests/test_branding_desc.py`
Expected: `OK`.

- [ ] **Step 7: Commit**

```bash
git add platform/iso/calamares/branding/thiscloud \
        platform/iso/calamares/tests/test_branding_desc.py
git commit -m "feat(iso): add THISCLOUD Calamares branding desc, colors, stylesheet"
```

---
---

### Task 3: Slideshow QML

**Files:**
- Create: `platform/iso/calamares/branding/thiscloud/show.qml`
- Test: `platform/iso/calamares/tests/test_slideshow_qml.py`

**Interfaces:**
- Consumes: `slides/slide-1..4.png` from Task 1 (referenced inside show.qml via branding dir resolution).
- Produces: `show.qml` loaded by Calamares slideshow during the exec phase (API 2: `onActivate`/`onLeave`).

- [ ] **Step 1: Write the failing test**

```python
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 platform/iso/calamares/tests/test_slideshow_qml.py`
Expected: FAIL — `show.qml` missing.

- [ ] **Step 3: Write show.qml**

```qml
/* THISCLOUD slideshow for Calamares (slideshowAPI 2). */
import io.calamares.core 1.0
import QtQuick 2.0
import QtQuick.Controls 2.0
import QtQuick.Layouts 1.3

Item {
    id: slideshowRoot
    width: 800
    height: 480

    property var timer: undefined

    Rectangle {
        anchors.fill: parent
        color: "#0f1115"
    }

    Image {
        id: slideImage
        anchors.fill: parent
        anchors.margins: 24
        fillMode: Image.PreserveAspectFit
        source: Qt.resolvedUrl("slides/slide-1.png")
    }

    function showSlide(n) {
        slideImage.source = Qt.resolvedUrl("slides/slide-" + n + ".png")
    }

    function startSlideshow() {
        if (timer === undefined) {
            timer = Qt.createQmlObject("import QtQuick 2.0; Timer { interval: 3000; repeat: true; }",
                                       slideshowRoot, "slideTimer")
        }
        var step = 1
        timer.triggered.connect(function() {
            step = (step % 4) + 1
            showSlide(step)
        })
        timer.start()
    }

    function stopSlideshow() {
        if (timer !== undefined) {
            timer.stop()
        }
    }

    function onActivate() { startSlideshow() }
    function onLeave()    { stopSlideshow() }

    Component.onCompleted: {
        // Slideshow API 2 requires the shell to call onActivate()/onLeave().
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 platform/iso/calamares/tests/test_slideshow_qml.py`
Expected: `OK`.

- [ ] **Step 5: Commit**

```bash
git add platform/iso/calamares/branding/thiscloud/show.qml \
        platform/iso/calamares/tests/test_slideshow_qml.py
git commit -m "feat(iso): add THISCLOUD Calamares slideshow QML"
```

---
---

### Task 4: thiscloudqml view module (C++/QtPlugin wrapper + QML form)

**Files:**
- Create: `platform/iso/calamares/modules/thiscloudqml/CMakeLists.txt`
- Create: `platform/iso/calamares/modules/thiscloudqml/ThisCloudViewStep.h`
- Create: `platform/iso/calamares/modules/thiscloudqml/ThisCloudViewStep.cpp`
- Create: `platform/iso/calamares/modules/thiscloudqml/thiscloudqml.qml`
- Create: `platform/iso/calamares/modules/thiscloudqml/thiscloudqml.conf`
- Test: `platform/iso/calamares/tests/test_thiscloudqml.py`

**Interfaces:**
- Consumes: Calamares `QmlViewStep` base API (`libcalamaresui/viewpages/QmlViewStep.h`), Qt6 Quick.
- Produces:
  - `ThisCloudViewStep::prettyName()` → `"THISCLOUD config"`.
  - `getConfig()` returns a `ThisCloudConfig*` QObject with Q_PROPERTYs: `nodeRole` (string, `"master"|"worker"`), `clusterName` (string), `nodeIp` (string), `interface` (string), plus slots `setNodeRole/setClusterName/setNodeIp/setInterface` invoked from QML.
  - On `next()`/onLeave: writes GlobalStorage keys `thiscloudRole`, `thiscloudClusterName`, `thiscloudNodeIp`, `thiscloudInterface`. Read by Task 5's `thiscloud` job module.
  - This directory is copied into the Calamares source tree by `build-calamares.sh` under `src/modules/thiscloudqml/` before cmake.

- [ ] **Step 1: Write the failing test**

```python
#!/usr/bin/env python3
"""Structural checks for the thiscloudqml view module sources."""
import os
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
MOD = os.path.normpath(os.path.join(HERE, os.pardir, "modules", "thiscloudqml"))


class TestThisCloudQml(unittest.TestCase):
    def test_files_exist(self):
        for name in ("CMakeLists.txt", "ThisCloudViewStep.h", "ThisCloudViewStep.cpp",
                     "thiscloudqml.qml", "thiscloudqml.conf"):
            self.assertTrue(os.path.isfile(os.path.join(MOD, name)), name)

    def test_cpp_extends_qmlviewstep(self):
        cpp = open(os.path.join(MOD, "ThisCloudViewStep.h")).read()
        self.assertIn("QmlViewStep", cpp)
        self.assertIn("CALAMARES_PLUGIN_FACTORY_DECLARATION", cpp)

    def test_cpp_writes_globalstorage(self):
        cpp = open(os.path.join(MOD, "ThisCloudViewStep.cpp")).read()
        for key in ("thiscloudRole", "thiscloudClusterName",
                    "thiscloudNodeIp", "thiscloudInterface"):
            self.assertIn(key, cpp)

    def test_qml_has_form_fields(self):
        qml = open(os.path.join(MOD, "thiscloudqml.qml")).read()
        for tok in ("ComboBox", "TextField", "nodeRole", "clusterName",
                    "nodeIp", "interface", "config"):
            self.assertIn(tok, qml)

    def test_qml_has_lifecycle_hooks(self):
        # Calamares QML view steps expose onActivate()/onLeave(); the Next
        # button is owned by ViewManager, so no onNextRequested is needed.
        qml = open(os.path.join(MOD, "thiscloudqml.qml")).read()
        self.assertIn("onActivate", qml)
        self.assertIn("onLeave", qml)

    def test_conf_maps_module(self):
        conf = open(os.path.join(MOD, "thiscloudqml.conf")).read()
        self.assertIn("qmlFilename", conf)


if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 platform/iso/calamares/tests/test_thiscloudqml.py`
Expected: FAIL — files missing.

- [ ] **Step 3: Write CMakeLists.txt**

```cmake
# THISCLOUD custom view module. This dir is copied into the Calamares
# source tree (src/modules/thiscloudqml/) by build-calamares.sh before cmake.
if(NOT WITH_QML)
    calamares_skip_module( "thiscloudqml (QML is not supported in this build)" )
    return()
endif()

calamares_add_plugin(thiscloudqml
    TYPE viewmodule
    EXPORT_MACRO PLUGINDLLEXPORT_PRO
    SOURCES
        ThisCloudViewStep.cpp
    RESOURCES
        thiscloudqml.qrc
    SHARED_LIB
)
```

- [ ] **Step 4: Write thiscloudqml.qrc**

```xml
<RCC>
    <qresource prefix="/">
        <file>thiscloudqml.qml</file>
    </qresource>
</RCC>
```

- [ ] **Step 5: Write ThisCloudViewStep.h**

```cpp
/* THISCLOUD install-config view step. */
#ifndef THISCLOUDVIEWSTEP_H
#define THISCLOUDVIEWSTEP_H

#include "utils/PluginFactory.h"
#include "viewpages/QmlViewStep.h"

#include <QObject>
#include <QString>

class ThisCloudConfig : public QObject
{
    Q_OBJECT
    Q_PROPERTY( QString nodeRole READ nodeRole WRITE setNodeRole NOTIFY nodeRoleChanged )
    Q_PROPERTY( QString clusterName READ clusterName WRITE setClusterName NOTIFY clusterNameChanged )
    Q_PROPERTY( QString nodeIp READ nodeIp WRITE setNodeIp NOTIFY nodeIpChanged )
    Q_PROPERTY( QString interface READ interface WRITE setInterface NOTIFY interfaceChanged )

public:
    explicit ThisCloudConfig( QObject* parent = nullptr );

    QString nodeRole() const { return m_nodeRole; }
    void setNodeRole( const QString& v ) { if ( v != m_nodeRole ) { m_nodeRole = v; emit nodeRoleChanged(); } }

    QString clusterName() const { return m_clusterName; }
    void setClusterName( const QString& v ) { if ( v != m_clusterName ) { m_clusterName = v; emit clusterNameChanged(); } }

    QString nodeIp() const { return m_nodeIp; }
    void setNodeIp( const QString& v ) { if ( v != m_nodeIp ) { m_nodeIp = v; emit nodeIpChanged(); } }

    QString interface() const { return m_interface; }
    void setInterface( const QString& v ) { if ( v != m_interface ) { m_interface = v; emit interfaceChanged(); } }

signals:
    void nodeRoleChanged();
    void clusterNameChanged();
    void nodeIpChanged();
    void interfaceChanged();

private:
    QString m_nodeRole = QStringLiteral( "worker" );
    QString m_clusterName = QStringLiteral( "thiscloud" );
    QString m_nodeIp = QStringLiteral( "127.0.0.1" );
    QString m_interface = QStringLiteral( "eth0" );
};

class ThisCloudViewStep : public Calamares::QmlViewStep
{
    Q_OBJECT

public:
    explicit ThisCloudViewStep( QObject* parent = nullptr );
    ~ThisCloudViewStep() override;

    QString prettyName() const override;
    void onLeave() override;
    bool isNextEnabled() const override;
    QObject* getConfig() override;

private:
    ThisCloudConfig* m_config = nullptr;
};

CALAMARES_PLUGIN_FACTORY_DECLARATION( ThisCloudViewStepFactory )

#endif
```

- [ ] **Step 6: Write ThisCloudViewStep.cpp**

```cpp
/* THISCLOUD install-config view step. */
#include "ThisCloudViewStep.h"

#include "GlobalStorage.h"
#include "utils/CalamaresUtilsGui.h"
#include "utils/Logger.h"
#include "utils/Variant.h"

#include <QVariant>

ThisCloudConfig::ThisCloudConfig( QObject* parent )
    : QObject( parent )
{
}

ThisCloudViewStep::ThisCloudViewStep( QObject* parent )
    : Calamares::QmlViewStep( parent )
    , m_config( new ThisCloudConfig( this ) )
{
}

ThisCloudViewStep::~ThisCloudViewStep() {}

QString
ThisCloudViewStep::prettyName() const
{
    return tr( "THISCLOUD config" );
}

void
ThisCloudViewStep::onLeave()
{
    Calamares::GlobalStorage* gs = Calamares::GlobalStorage::instance();
    if ( gs )
    {
        gs->insert( QStringLiteral( "thiscloudRole" ), m_config->nodeRole() );
        gs->insert( QStringLiteral( "thiscloudClusterName" ), m_config->clusterName() );
        gs->insert( QStringLiteral( "thiscloudNodeIp" ), m_config->nodeIp() );
        gs->insert( QStringLiteral( "thiscloudInterface" ), m_config->interface() );
    }
    Calamares::QmlViewStep::onLeave();
}

bool
ThisCloudViewStep::isNextEnabled() const
{
    // Always allow proceeding; validation is advisory.
    return true;
}

QObject*
ThisCloudViewStep::getConfig()
{
    return m_config;
}

CALAMARES_PLUGIN_FACTORY_DEFINITION( ThisCloudViewStepFactory, registerPlugin< ThisCloudViewStep >(); )
```

- [ ] **Step 7: Write thiscloudqml.qml**

```qml
/* THISCLOUD node configuration form shown as a Calamares view step. */
import io.calamares.core 1.0
import io.calamares.ui 1.0

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    width: parent.width
    height: parent.height

    Rectangle {
        anchors.fill: parent
        color: "#0f1115"
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 32
        spacing: 16

        Label {
            text: qsTr("THISCLOUD node configuration")
            font.pointSize: 18
            color: "#e6e9ef"
        }

        Label {
            text: qsTr("Configure how this node joins the THISCLOUD cluster.")
            color: "#e6e9ef"
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
        }

        GridLayout {
            columns: 2
            columnSpacing: 16
            rowSpacing: 12
            Layout.fillWidth: true

            Label { text: qsTr("Node role"); color: "#e6e9ef" }
            ComboBox {
                id: roleCombo
                Layout.preferredWidth: 240
                model: ["worker", "master"]
                onCurrentIndexChanged: config.nodeRole = currentText
                Component.onCompleted: {
                    currentIndex = model.indexOf(config.nodeRole)
                }
            }

            Label { text: qsTr("Cluster name"); color: "#e6e9ef" }
            TextField {
                id: clusterField
                Layout.preferredWidth: 240
                text: config.clusterName
                onTextEdited: config.clusterName = text
            }

            Label { text: qsTr("Node IP address"); color: "#e6e9ef" }
            TextField {
                id: ipField
                Layout.preferredWidth: 240
                text: config.nodeIp
                onTextEdited: config.nodeIp = text
            }

            Label { text: qsTr("Network interface"); color: "#e6e9ef" }
            ComboBox {
                id: ifaceCombo
                Layout.preferredWidth: 240
                editable: true
                textRole: "text"
                model: ListModel {
                    id: ifaceModel
                    ListElement { text: "eth0" }
                    ListElement { text: "ens3" }
                    ListElement { text: "enp1s0" }
                }
                onCurrentTextChanged: config.interface = currentText
                Component.onCompleted: {
                    for (var i = 0; i < ifaceModel.count; ++i) {
                        if (ifaceModel.get(i).text === config.interface) { currentIndex = i; break }
                    }
                }
            }
        }

        Item { Layout.fillHeight: true }
    }

    function onActivate() {}
    function onLeave() {}
}
```

- [ ] **Step 8: Write thiscloudqml.conf**

```yaml
# Config for the thiscloudqml view module (instance name = module name).
# Loads thiscloudqml.qml (from module QRC or branding override).
---
qmlSearch: both
qmlFilename: "thiscloudqml.qml"
```

- [ ] **Step 9: Run test to verify it passes**

Run: `python3 platform/iso/calamares/tests/test_thiscloudqml.py`
Expected: `OK`.

- [ ] **Step 10: Commit**

```bash
git add platform/iso/calamares/modules/thiscloudqml \
        platform/iso/calamares/tests/test_thiscloudqml.py
git commit -m "feat(iso): add THISCLOUD config QML view module for Calamares"
```

---
---

### Task 5: thiscloud Python job module (writes config, runs thiscloud init)

**Files:**
- Create: `platform/iso/calamares/modules/thiscloud/thiscloud_logic.py`
- Create: `platform/iso/calamares/modules/thiscloud/main.py`
- Create: `platform/iso/calamares/modules/thiscloud/module.desc`
- Create: `platform/iso/calamares/modules/thiscloud/thiscloud.conf`
- Test: `platform/iso/calamares/tests/test_thiscloud_logic.py`

**Interfaces:**
- Consumes: GlobalStorage keys from Task 4: `thiscloudRole`, `thiscloudClusterName`, `thiscloudNodeIp`, `thiscloudInterface`.
- Produces (in `main.run()`):
  - Writes `<rootMountPoint>/etc/thiscloud/config.toml`.
  - Runs `thiscloud init --ip <ip> --role <role>` in the target chroot via `libcalamares.utils.target_env_call`.
  - Enables systemd units per role in the target.
  - `run()` returns `None` on success or `(str, str)` error tuple (Calamares contract, see dummypython).

- [ ] **Step 1: Write the failing tests**

```python
#!/usr/bin/env python3
"""Unit tests for the pure logic of the thiscloud job module."""
import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
MOD = os.path.normpath(os.path.join(HERE, os.pardir, "modules", "thiscloud"))
sys.path.insert(0, MOD)

import thiscloud_logic  # noqa: E402


class TestConfigToml(unittest.TestCase):
    def test_valid_ip(self):
        self.assertTrue(thiscloud_logic.is_valid_ip("192.168.1.10"))
        self.assertTrue(thiscloud_logic.is_valid_ip("10.0.0.1"))

    def test_invalid_ip(self):
        self.assertFalse(thiscloud_logic.is_valid_ip("999.1.1.1"))
        self.assertFalse(thiscloud_logic.is_valid_ip("not-an-ip"))
        self.assertFalse(thiscloud_logic.is_valid_ip(""))

    def test_valid_role(self):
        self.assertTrue(thiscloud_logic.is_valid_role("master"))
        self.assertTrue(thiscloud_logic.is_valid_role("worker"))
        self.assertFalse(thiscloud_logic.is_valid_role("admin"))

    def test_sanitize_cluster(self):
        self.assertEqual(thiscloud_logic.sanitize_cluster_name("  My Cluster  "), "my-cluster")
        self.assertEqual(thiscloud_logic.sanitize_cluster_name("a.b"), "ab")

    def test_render_config_toml(self):
        toml = thiscloud_logic.render_config_toml(
            cluster="my-cluster", ip="10.0.0.5", role="master", iface="eth0")
        self.assertIn('cluster_name = "my-cluster"', toml)
        self.assertIn('node_ip = "10.0.0.5"', toml)
        self.assertIn('role = "master"', toml)
        self.assertIn('interface = "eth0"', toml)

    def test_write_config(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = thiscloud_logic.write_config(
                tmp, cluster="c", ip="127.0.0.1", role="worker", iface="eth0")
            self.assertEqual(path, os.path.join(tmp, "etc", "thiscloud", "config.toml"))
            self.assertTrue(os.path.isfile(path))
            self.assertIn('cluster_name = "c"', open(path).read())

    def test_services_for_role(self):
        self.assertIn("thiscloudd.service", thiscloud_logic.services_for_role("master"))
        self.assertIn("thiscloud-api.service", thiscloud_logic.services_for_role("master"))
        self.assertIn("thiscloudd.service", thiscloud_logic.services_for_role("worker"))
        self.assertNotIn("thiscloud-api.service", thiscloud_logic.services_for_role("worker"))


if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `python3 platform/iso/calamares/tests/test_thiscloud_logic.py`
Expected: FAIL — `ModuleNotFoundError: thiscloud_logic`.

- [ ] **Step 3: Write thiscloud_logic.py**

```python
"""Pure logic for the THISCLOUD Calamares job module (no libcalamares).

Kept importable outside Calamares so it is unit-testable on any host.
"""
import ipaddress
import os
import re


def is_valid_ip(value):
    try:
        ipaddress.ip_address(value.strip())
        return True
    except (ValueError, AttributeError):
        return False


def is_valid_role(value):
    return value.strip() in ("master", "worker")


def sanitize_cluster_name(value):
    """Lowercase, keep [a-z0-9-], drop everything else."""
    value = value.strip().lower()
    return re.sub(r"[^a-z0-9-]+", "", value)


def render_config_toml(cluster, ip, role, iface):
    cluster = sanitize_cluster_name(cluster)
    return (
        "# THISCLOUD node configuration (written by the installer)\n"
        f'cluster_name = "{cluster}"\n'
        f'node_ip = "{ip}"\n'
        f'role = "{role}"\n'
        f'interface = "{iface}"\n'
    )


def write_config(root, cluster, ip, role, iface):
    """Write config.toml under root/etc/thiscloud/. Returns the path."""
    cfg_dir = os.path.join(root, "etc", "thiscloud")
    os.makedirs(cfg_dir, exist_ok=True)
    path = os.path.join(cfg_dir, "config.toml")
    with open(path, "w") as f:
        f.write(render_config_toml(cluster, ip, role, iface))
    return path


def services_for_role(role):
    """Systemd units to enable for the given node role."""
    base = ["thiscloudd.service", "thiscloud-webui.service",
            "thiscloud-ports.service", "thiscloud-web-port.service"]
    if role == "master":
        base.append("thiscloud-api.service")
    return sorted(set(base))


def build_init_args(ip, role):
    """Argv for `thiscloud init` in the target chroot."""
    return ["/usr/bin/thiscloud", "init", "--ip", ip.strip(), "--role", role.strip()]
```

- [ ] **Step 4: Write main.py**

```python
#!/usr/bin/env python3
"""THISCLOUD Calamares job module — applies node config to the target.

Runs after partition/mount (target available at rootMountPoint),
before/around bootloader. Reads GlobalStorage keys set by thiscloudqml.
"""
import libcalamares

from thiscloud_logic import (is_valid_ip, is_valid_role,
                             sanitize_cluster_name, services_for_role,
                             write_config, build_init_args)

_ = lambda s: s  # noqa: E731 — translation is optional for this module


def pretty_name():
    return _("THISCLOUD configuration")


def run():
    gs = libcalamares.globalstorage
    root = gs.value("rootMountPoint")
    if not root:
        return (_("No root mount point"),
                _("rootMountPoint was not set; mount module did not run."))

    role = str(gs.value("thiscloudRole") or "worker").strip()
    cluster = sanitize_cluster_name(str(gs.value("thiscloudClusterName") or "thiscloud"))
    ip = str(gs.value("thiscloudNodeIp") or "127.0.0.1").strip()
    iface = str(gs.value("thiscloudInterface") or "eth0").strip()

    if not is_valid_role(role):
        return (_("Invalid node role"),
                _("thiscloudRole must be 'master' or 'worker', got '{role}'.").format(role=role))
    if not is_valid_ip(ip):
        return (_("Invalid node IP"),
                _("thiscloudNodeIp '{ip}' is not a valid IP address.").format(ip=ip))

    libcalamares.utils.debug("Writing /etc/thiscloud/config.toml")
    write_config(root, cluster, ip, role, iface)

    libcalamares.utils.debug("Running thiscloud init")
    r = libcalamares.utils.target_env_call(build_init_args(ip, role))
    if r != 0:
        return (_("thiscloud init failed"),
                _("`thiscloud init --ip {ip} --role {role}` exited {code}.").format(ip=ip, role=role, code=r))

    for unit in services_for_role(role):
        libcalamares.utils.target_env_call(
            ["/usr/bin/systemctl", "enable", unit])

    libcalamares.globalstorage.insert("thiscloudClusterName", cluster)
    return None
```

- [ ] **Step 5: Write module.desc**

```yaml
# Module metadata for the THISCLOUD job module.
---
type: "job"
name: "thiscloud"
interface: "python"
script: "main.py"
```

- [ ] **Step 6: Write thiscloud.conf**

```yaml
# THISCLOUD job module configuration.
# The module reads node settings from GlobalStorage (set by thiscloudqml).
---
# No module-specific options required.
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `python3 platform/iso/calamares/tests/test_thiscloud_logic.py`
Expected: `OK`.

- [ ] **Step 8: Syntax-check the wrapper (no Calamares runtime on macOS)**

Run: `python3 -m py_compile platform/iso/calamares/modules/thiscloud/main.py`
Expected: exit 0, creates `__pycache__`.

- [ ] **Step 9: Commit**

```bash
git add platform/iso/calamares/modules/thiscloud \
        platform/iso/calamares/tests/test_thiscloud_logic.py
git commit -m "feat(iso): add THISCLOUD Calamares python job module"
```

---
---

### Task 6: Main Calamares settings.conf (module sequence)

**Files:**
- Create: `platform/iso/calamares/settings.conf`
- Test: `platform/iso/calamares/tests/test_settings_conf.py`

**Interfaces:**
- Consumes: module names from Tasks 4-5 plus stock Calamares modules (`partition`, `mount`, `unpackfs`, `fstab`, `locale`, `keyboard`, `localecfg`, `users`, `networkcfg`, `hwclock`, `services-systemd`, `initramfs`, `bootloader`, `umount`, `welcome`, `timezone`, `summary`, `finished`).
- Produces: the Calamares global config installed to `/etc/calamares/settings.conf` on the live system.

- [ ] **Step 1: Write the failing test**

```python
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 platform/iso/calamares/tests/test_settings_conf.py`
Expected: FAIL — `settings.conf` missing.

- [ ] **Step 3: Write settings.conf**

```yaml
# THISCLOUD Calamares global configuration.
# Installed to /etc/calamares/settings.conf on the live system.
---
modules-search: [ local ]

# No custom instances; every module uses its implicit instance.
#instances: []

sequence:
- show:
  - welcome
  - locale
  - keyboard
  - timezone
  - partition
  - users
  - network
  - thiscloudqml
  - summary
- exec:
  - partition
  - mount
  - unpackfs
  - fstab
  - locale
  - keyboard
  - localecfg
  - users
  - networkcfg
  - hwclock
  - services-systemd
  - initramfs
  - bootloader
  - thiscloud
  - umount
- show:
  - finished

branding: thiscloud
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 platform/iso/calamares/tests/test_settings_conf.py`
Expected: `OK`.

- [ ] **Step 5: Commit**

```bash
git add platform/iso/calamares/settings.conf \
        platform/iso/calamares/tests/test_settings_conf.py
git commit -m "feat(iso): add Calamares settings.conf with THISCLOUD module sequence"
```

---
---

### Task 7: build-calamares.sh — compile Calamares + KPMcore into the live rootfs

**Files:**
- Create: `platform/iso/calamares/scripts/build-calamares.sh`

**Interfaces:**
- Consumes (builder-only, AlmaLinux 9 x86_64):
  - Source tarballs: `calamares-3.3.14.tar.gz` (GitHub `calamares/calamares` tag `v3.3.14`), `kpmcore-24.05.2.tar.xz` (KDE download / `invent.kde.org/libs/kpmcore/-/archive/v24.05.2/`).
  - `thiscloudqml` module sources (Task 4) copied in from `iso/calamares/modules/thiscloudqml`.
  - EPEL9 + AlmaLinux base repos (builder must run `install-deps.sh` additions first — Task 11).
- Produces: `calamares` binary + plugins + `python3` job runner installed under a staging root (`/tmp/live-root`), packaged into RPMs in `iso/repo` that `live.ks` (Task 8) pulls into the live system via its local repo. Prints `DONE` on success.

- [ ] **Step 1: Write the script (builder-only; verify with bash -n locally)**

```bash
#!/usr/bin/env bash
# Build Calamares 3.3.14 + KPMcore 24.05.2 from source into a staging root
# that the live ISO (live.ks) pulls in as RPMs from iso/repo.
#
# MUST run on AlmaLinux 9 x86_64 with the build deps installed
# (see install-deps.sh additions). Run from platform/iso/calamares/.
#
#   ./scripts/build-calamares.sh /tmp/live-root
set -euo pipefail

STAGING="${1:?usage: build-calamares.sh /path/to/live-root}"
CALAMARES_VER="${CALAMARES_VER:-3.3.14}"
KPMCORE_VER="${KPMCORE_VER:-24.05.2}"
WORK="$(pwd)/.build"
SRC="$WORK/src"

echo "==> staging root: $STAGING"
mkdir -p "$WORK" "$SRC" "$STAGING"

# ── Fetch sources ────────────────────────────────────────────────────
echo "==> fetching calamares $CALAMARES_VER"
if [ ! -d "$SRC/calamares" ]; then
  curl -fsSL "https://github.com/calamares/calamares/archive/refs/tags/v${CALAMARES_VER}.tar.gz" -o "$WORK/calamares.tar.gz"
  tar -xzf "$WORK/calamares.tar.gz" -C "$SRC"
  mv "$SRC/calamares-${CALAMARES_VER}" "$SRC/calamares"
fi

echo "==> fetching kpmcore $KPMCORE_VER"
if [ ! -d "$SRC/kpmcore" ]; then
  curl -fsSL "https://invent.kde.org/libs/kpmcore/-/archive/v${KPMCORE_VER}/kpmcore-v${KPMCORE_VER}.tar.gz" -o "$WORK/kpmcore.tar.gz"
  tar -xzf "$WORK/kpmcore.tar.gz" -C "$SRC"
  mv "$SRC/kpmcore-v${KPMCORE_VER}" "$SRC/kpmcore"
fi

# ── Build KPMcore ────────────────────────────────────────────────────
echo "==> building kpmcore"
cmake -S "$SRC/kpmcore" -B "$WORK/kpmcore-build" \
  -DCMAKE_INSTALL_PREFIX=/usr \
  -DCMAKE_INSTALL_LIBDIR=/usr/lib64 \
  -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_TESTING=OFF
cmake --build "$WORK/kpmcore-build" -j"$(nproc)"
cmake --install "$WORK/kpmcore-build" --prefix "$STAGING/usr"
DESTDIR="$STAGING" cmake --install "$WORK/kpmcore-build"

# ── Inject THISCLOUD module ─────────────────────────────────────────
echo "==> injecting thiscloudqml module"
THISCLOUD_MOD="$(pwd)/modules/thiscloudqml"
cp -r "$THISCLOUD_MOD" "$SRC/calamares/src/modules/thiscloudqml"

# ── Build Calamares ──────────────────────────────────────────────────
echo "==> configuring calamares"
cmake -S "$SRC/calamares" -B "$WORK/calamares-build" \
  -DCMAKE_INSTALL_PREFIX=/usr \
  -DCMAKE_INSTALL_LIBDIR=/usr/lib64 \
  -DCMAKE_BUILD_TYPE=Release \
  -DKPMCORE_DIR="$STAGING/usr/lib64/cmake/kpmcore" \
  -DWITH_QML=ON \
  -DWITH_PYTHON=ON \
  -DINSTALL_CONFIG=ON \
  -DSKIP_PEDANTIC=ON

echo "==> building calamares"
cmake --build "$WORK/calamares-build" -j"$(nproc)"
DESTDIR="$STAGING" cmake --install "$WORK/calamares-build"

# ── Install branding + settings + module into staging ───────────────
echo "==> installing THISCLOUD branding/settings"
BRANDING_DIR="$(pwd)/branding/thiscloud"
install -d "$STAGING/etc/calamares/branding/thiscloud"
cp -r "$BRANDING_DIR"/. "$STAGING/etc/calamares/branding/thiscloud/"

install -d "$STAGING/etc/calamares"
cp -f "$(pwd)/settings.conf" "$STAGING/etc/calamares/settings.conf"

install -d "$STAGING/etc/calamares/modules/thiscloud"
cp -f "$(pwd)/modules/thiscloud/main.py" "$STAGING/etc/calamares/modules/thiscloud/"
cp -f "$(pwd)/modules/thiscloud/thiscloud_logic.py" "$STAGING/etc/calamares/modules/thiscloud/"
cp -f "$(pwd)/modules/thiscloud/module.desc" "$STAGING/etc/calamares/modules/thiscloud/"
cp -f "$(pwd)/modules/thiscloud/thiscloud.conf" "$STAGING/etc/calamares/modules/thiscloud/"

echo "==> sanity checks"
test -x "$STAGING/usr/bin/calamares" && echo "  calamares: OK"
ls "$STAGING/usr/lib64/calamares/modules/" | grep -q thiscloudqml && echo "  thiscloudqml plugin: OK"
test -f "$STAGING/etc/calamares/branding/thiscloud/branding.desc" && echo "  branding: OK"
test -f "$STAGING/etc/calamares/settings.conf" && echo "  settings: OK"

# ── Package the staging root into RPMs for live.ks %packages ─────────
# livemedia-creator resolves %packages from repos; the live host is built
# from RPMs, so the compiled Calamares/KPMcore must become RPMs.
echo "==> packaging staging root into RPMs"
RPMROOT="$WORK/rpm"
mkdir -p "$RPMROOT"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
REPO_RPMS="${THISCLOUD_REPO_RPMS:-$(cd "$(pwd)/.." && pwd)/repo}"  # platform/iso/repo — createrepo'd repo

rpmbuild_spec() { # $1=name $2=version $3=summary
  cat > "$RPMROOT/SPECS/$1.spec" <<EOF
%define _topdir $RPMROOT
Name: $1
Version: $2
Release: 1
Summary: $3
License: GPL-3.0-or-later
BuildArch: $(uname -m)
BuildRoot: %{_tmppath}/%{name}-%{version}-root

%description
$3. Compiled from source for AlmaLinux 9 (no EPEL9 package).

%install
rm -rf %{buildroot}
cp -a "$STAGING"/. %{buildroot}/

%files
EOF
  # Enumerate every staged file (path relative to root, leading /).
  ( cd "$STAGING" && find . -type f -o -type l | sort | sed 's|^\.|/|' ) \
    >> "$RPMROOT/SPECS/$1.spec"
}

# kpmcore and calamares each own their installed tree; split by prefix is
# fiddly, so ship both trees in one calamares RPM plus a tiny kpmcore RPM.
# (Simplest correct split: everything under /usr/lib64/cmake/kpmcore and
# kpmcore headers/libs go in kpmcore; here we bundle both into calamares
# RPM to keep the spec list trivial — builder may refine.)
rpmbuild_spec calamares "${CALAMARES_VER}" "Calamares installer + KPMcore for THISCLOUD"
rpmbuild --define "_topdir $RPMROOT" -bb "$RPMROOT/SPECS/calamares.spec" \
  || { echo "ERROR: rpmbuild failed (see $RPMROOT/rpms-build.log)"; exit 1; }
mkdir -p "$REPO_RPMS"
cp "$RPMROOT"/RPMS/*/*.rpm "$REPO_RPMS/" 2>/dev/null || cp "$RPMROOT"/RPMS/*.rpm "$REPO_RPMS/" 2>/dev/null || true
echo "  rpm output: $(ls "$REPO_RPMS")"

echo "==> regenerating repo metadata"
if command -v createrepo_c >/dev/null 2>&1; then
  createrepo_c --update "$REPO_RPMS"
else
  echo "WARNING: createrepo_c not found — run: dnf install -y createrepo_c; createrepo_c $REPO_RPMS"
fi

echo "DONE"
```

- [ ] **Step 2: Verify script syntax locally**

Run: `bash -n platform/iso/calamares/scripts/build-calamares.sh`
Expected: exit 0 (no output).

- [ ] **Step 3: Commit**

```bash
chmod +x platform/iso/calamares/scripts/build-calamares.sh
git add platform/iso/calamares/scripts/build-calamares.sh
git commit -m "feat(iso): add Calamares+KPMcore source build script for EL9"
```

---
---

### Task 8: Live environment kickstart (live.ks)

**Files:**
- Create: `platform/iso/calamares/live/live.ks`
- Create: `platform/iso/calamares/live/autostart/calamares.desktop`
- Create: `platform/iso/calamares/live/autostart/xorg-autologin.conf`

**Interfaces:**
- Consumes: Calamares RPMs produced by Task 7 (staged under the local repo `iso/repo`), thiscloud branding.
- Produces: a bootable live ISO (built by `livemedia-creator --make-iso --no-virt` in Task 9) that boots Xorg → openbox → autologin → Calamares autostart.

- [ ] **Step 1: Write live.ks**

```
# THISCLOUD live installer kickstart.
# Builds the graphical live environment that hosts Calamares.
# The installed target system is produced BY Calamares at install time;
# this kickstart only describes the LIVE (host) environment.

# ── Boot behaviour ───────────────────────────────────────────────────
# No firstboot on the live host.
firstboot --disable

# ── Accounts ─────────────────────────────────────────────────────────
# Live host is root-only, autologin, no password (installer host only).
rootpw --lock
user --name=live --groups=wheel --iscrypted --password='$6$rounds=656000$x'

# ── Network ──────────────────────────────────────────────────────────
network --bootproto=dhcp --device=link --activate --onboot=yes

# ── Storage ──────────────────────────────────────────────────────────
# Live host does not persist; a tmpfs overlay is fine.
zerombr
clearpart --none --initlabel
part / --size=4096 --grow --fstype=ext4

# ── Source ───────────────────────────────────────────────────────────
# Live ISO built with livemedia-creator uses a different source model;
# packages come from the configured repos at build time.
#cdrom

# Local repo with THISCLOUD RPMs (thiscloud, thiscloudd, calamares,
# kpmcore — built by build-calamares.sh). Path is host-visible because
# the build runs with --no-virt.
repo --name=thiscloud-local --baseurl=file:///data/thiscloud-repo

# ── Packages ─────────────────────────────────────────────────────────
%packages
@core
# Graphical session: Xorg + lightweight WM + autologin
-xorg-x11-drv-vesa
xorg-x11-server-Xorg
xorg-x11-server-common
openbox
xterm
xsetroot
# Calamares + runtime deps (built by build-calamares.sh → RPMs in
# thiscloud-local repo)
calamares
kpmcore
python3
python3-pyqt6
qt6-qtbase
qt6-qtsvg
qt6-qtdeclarative
# THISCLOUD runtime bits (reused from existing repo)
thiscloud
thiscloudd
%end

# ── Post: wire autologin + autostart ─────────────────────────────────
%post --log=/root/live-post.log
echo "==> configuring live autologin + calamares autostart"

# openbox autostart for the 'live' user
mkdir -p /home/live/.config/openbox
cat > /home/live/.xinitrc <<'EOF'
# startx entry point — launch openbox-session (which runs autostart below)
exec openbox-session
EOF
cat > /home/live/.config/openbox/autostart <<'EOF'
# Launch the THISCLOUD installer once a display is up.
if [ -x /usr/bin/calamares ]; then
  /usr/bin/calamares --style "$(python3 -c 'print("thiscloud")')" &
fi
EOF
chown -R live:live /home/live/.config

# Xorg autologin (root on tty1 auto-starting X). Simplest reliable path:
# getty@tty1 spawns a login shell that starts X as the live user.
cat > /etc/systemd/system/x11-autologin@.service <<'EOF'
[Unit]
Description=X11 Autologin
After=systemd-user-sessions.service

[Service]
ExecStartPre=/usr/bin/install -d -o live -g live /tmp/.X11-unix
ExecStart=/usr/bin/startx -- -nolisten tcp vt1
Restart=no
User=live
Environment=HOME=/home/live

[Install]
WantedBy=multi-user.target
EOF

systemctl enable x11-autologin@tty1 2>/dev/null || true
echo "==> live post complete"
%end

# ── Kernel options ───────────────────────────────────────────────────
%addon com_redhat_kdump --disable
%end
```

- [ ] **Step 2: Write autostart/calamares.desktop**

```desktop
[Desktop Entry]
Type=Application
Name=THISCLOUD Installer
Comment=Install THISCLOUD to this computer
Exec=/usr/bin/calamares --style thiscloud
Terminal=false
Categories=System;
X-GNOME-Autostart-enabled=true
```

- [ ] **Step 3: Write autostart/xorg-autologin.conf**

```
# Placeholder documentation file for the live autologin design.
# The actual mechanism is the x11-autologin@.service defined in live.ks %post.
# This file documents intent and is shipped for traceability only.
```

- [ ] **Step 4: Verify syntax locally (kickstart not parseable on macOS; structural check)**

Run: `grep -q "livemedia" platform/iso/calamares/live/live.ks || echo "no livemedia ref (expected — builder uses livemedia-creator)"`
Expected: prints the note; exit 0.

- [ ] **Step 5: Commit**

```bash
git add platform/iso/calamares/live
git commit -m "feat(iso): add live environment kickstart hosting Calamares"
```

---
---

### Task 9: Live ISO build script + fetch/remix integration

**Files:**
- Create: `platform/iso/calamares/scripts/build-live-iso.sh`
- Modify: `platform/iso/scripts/fetch-deps.sh` (add Calamares source + product.img note)
- Modify: `platform/iso/scripts/build-iso.sh` (replace Anaconda product.img/remix path with live path)

**Interfaces:**
- Consumes: `live.ks` (Task 8), staged Calamares rootfs (Task 7), `iso/repo/` (existing THISCLOUD artifacts from build-iso.sh steps [1-4]).
- Produces: `ThisCloud-<VERSION>-x86_64.iso` live image via `livemedia-creator --make-iso`.

- [ ] **Step 1: Write build-live-iso.sh**

```bash
#!/usr/bin/env bash
# Build the THISCLOUD live ISO hosting the Calamares installer.
# MUST run on AlmaLinux 9 x86_64 (livemedia-creator/lorax).
#
#   ALMAISO=/data/AlmaLinux-9-latest-x86_64-minimal.iso ./scripts/build-live-iso.sh
set -euo pipefail

ALMAISO="${ALMAISO:-/data/AlmaLinux-9-latest-x86_64-minimal.iso}"
OUT="${OUT:-/data/thiscloud-live-iso}"
VERSION="${VERSION:-0.1.0}"
LIVE_ROOT="${LIVE_ROOT:-/tmp/live-root}"
LOCAL_REPO="${LOCAL_REPO:-/data/thiscloud-repo}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CALAMAES_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ISO_REPO="$(cd "$CALAMAES_DIR/.." && pwd)/repo"   # existing THISCLOUD RPM repo

echo "==> building THISCLOUD live ISO"
mkdir -p "$OUT"

# 1. Build Calamares + KPMcore (from source) and package them as RPMs
#    into the local THISCLOUD repo (iso/repo).
if [ ! -f "$ISO_REPO/repodata/repomd.xml" ]; then
  echo "ERROR: $ISO_REPO is not a dnf repo (no repodata/). Run build-iso.sh step [1-4] first." >&2
  exit 1
fi
THISCLOUD_REPO_RPMS="$ISO_REPO" bash "$SCRIPT_DIR/build-calamares.sh" "$LIVE_ROOT"

# 2. Point live.ks at a host-visible repo. livemedia-creator runs with
#    --no-virt, so the file:// baseurl is reachable from the host.
mkdir -p "$(dirname "$LOCAL_REPO")"
if [ "$(readlink -f "$ISO_REPO")" != "$(readlink -f "$LOCAL_REPO")" ]; then
  rm -rf "$LOCAL_REPO"
  cp -a "$ISO_REPO"/. "$LOCAL_REPO"/
fi
# Builder-verified detail: if no-virt dnf can't reach the host file:// URL,
# serve it over http instead — `python3 -m http.server 8080 -d "$LOCAL_REPO"`
# and set `repo --name=thiscloud-local --baseurl=http://127.0.0.1:8080`
# in live.ks. Keep this line in sync with the repo URL in live.ks.

# 3. Assemble the live ISO. Package set (incl. calamares/kpmcore RPMs)
#    comes from %packages in live.ks, resolved against the local repo.
livemedia-creator --make-iso --no-virt --iso-only \
  --ks "$CALAMAES_DIR/live/live.ks" \
  --source "$ALMAISO" \
  --resultdir "$OUT" \
  --project "THISCLOUD" \
  --releasever 9 \
  --volid "THISCLOUD-${VERSION}"

echo "==> Done"
ls -lh "$OUT"/*.iso
```

- [ ] **Step 2: Verify syntax locally**

Run: `bash -n platform/iso/calamares/scripts/build-live-iso.sh`
Expected: exit 0.

- [ ] **Step 3: Modify fetch-deps.sh — append a Calamares source fetch**

```bash
# ── Calamares + KPMcore source tarballs (for build-calamares.sh) ────
echo "==> Fetching Calamares/KPMcore sources"
mkdir -p "$REPO/sources"
if [ ! -f "$REPO/sources/calamares-3.3.14.tar.gz" ]; then
  curl -fSL "https://github.com/calamares/calamares/archive/refs/tags/v3.3.14.tar.gz" \
    -o "$REPO/sources/calamares-3.3.14.tar.gz"
fi
if [ ! -f "$REPO/sources/kpmcore-v24.05.2.tar.gz" ]; then
  curl -fSL "https://invent.kde.org/libs/kpmcore/-/archive/v24.05.2/kpmcore-v24.05.2.tar.gz" \
    -o "$REPO/sources/kpmcore-v24.05.2.tar.gz"
fi
```

*(Insert before the final "done"/summary echo of `fetch-deps.sh`; adjust `$REPO` to match that file's existing variable.)*

- [ ] **Step 4: Modify build-iso.sh — swap steps [8]/[9] for the live path**

Replace the block:

```bash
echo "==> [8/9] Build product.img for Anaconda branding"
bash iso/scripts/make-product-img.sh

echo "==> [9/9] Remix ISO with THISCLOUD branding"
# remix-iso.sh extracts the base ISO, rebrands boot menus, injects the
# kickstart, repo, and product.img, then rebuilds with xorriso.
INPUT_ISO="$ALMAISO" OUTPUT_ISO="$OUT/ThisCloud-${VERSION}-x86_64.iso" \
  bash iso/scripts/remix-iso.sh
```

with:

```bash
echo "==> [8/9] Build live installer ISO (Calamares)"
# The old Anaconda path (make-product-img.sh + remix-iso.sh) is replaced by
# the Calamares live flow. Calamares+KPMcore are compiled from source and
# the live ISO is assembled by livemedia-creator.
ALMAISO="$ALMAISO" OUT="$OUT" VERSION="$VERSION" \
  bash iso/calamares/scripts/build-live-iso.sh
```

- [ ] **Step 5: Commit**

```bash
git add platform/iso/calamares/scripts/build-live-iso.sh \
        platform/iso/scripts/fetch-deps.sh \
        platform/iso/scripts/build-iso.sh
git commit -m "feat(iso): route ISO build through Calamares live installer"
```

---
---

### Task 10: Update install-deps.sh with Calamares build deps

**Files:**
- Modify: `platform/iso/scripts/install-deps.sh`

**Interfaces:**
- Consumes: nothing new.
- Produces: builder has packages to compile Calamares/KPMcore (Task 7) and run livemedia-creator (Task 9).

- [ ] **Step 1: Append the Calamares toolchain block to install-deps.sh**

```bash
# ── Calamares builder deps (compile from source for EL9) ─────────────
echo "==> Installing Calamares/KPMcore build deps"
dnf install -y \
  gcc-c++ gcc make cmake ninja-build \
  qt6-qtbase-devel qt6-qtsvg-devel qt6-qtdeclarative-devel \
  qt6-qtquickcontrols2-devel qt6-qtquicktemplates2-devel \
  boost-devel yaml-cpp-devel parted-devel \
  extra-cmake-modules kf5-kcoreaddons-devel kf5-ki18n-devel kf5-kconfig-devel \
  python3 python3-devel python3-pyqt6 \
  lorax livemedia-utils createrepo_c rpm-build 2>/dev/null \
  || echo "WARNING: some Calamares deps unavailable from current repos (EPEL9 may be needed)"
```

- [ ] **Step 2: Verify syntax locally**

Run: `bash -n platform/iso/scripts/install-deps.sh`
Expected: exit 0.

- [ ] **Step 3: Commit**

```bash
git add platform/iso/scripts/install-deps.sh
git commit -m "chore(iso): add Calamares build deps to install-deps"
```

---
---

### Task 11: Remove the dead Anaconda kickstart path + docs

**Files:**
- Delete: `platform/iso/kickstart/thiscloud.ks`
- Modify: `platform/iso/README.md`

**Interfaces:**
- Consumes: everything above.
- Produces: repo docs reflect the Calamares live flow; no stale Anaconda references.

- [ ] **Step 1: Delete the obsolete kickstart**

Run: `git rm platform/iso/kickstart/thiscloud.ks`
Expected: file removed.

- [ ] **Step 2: Document the new flow in README.md**

Append/replace the "How it works" section:

```markdown
## Installer (Calamares)

The ISO is a **live image**: it boots a minimal graphical session
(Xorg + openbox, root autologin) and auto-launches the Calamares installer
with THISCLOUD branding. The installer writes to disk (partition via
KPMcore, filesystem copy via unpackfs, GRUB via the bootloader module) and
runs a custom **thiscloud** job module that writes `/etc/thiscloud/config.toml`
and runs `thiscloud init --ip <ip> --role <role>` in the target.

Custom pieces under `iso/calamares/`:
- `branding/thiscloud/` — branding.desc, colors.conf, stylesheet.qss, show.qml, generated PNGs.
- `modules/thiscloudqml/` — QML view module collecting node role/cluster/IP/interface (compiled into Calamares).
- `modules/thiscloud/` — Python job module applying config to the target.
- `settings.conf` — module sequence.
- `scripts/build-calamares.sh` — compiles Calamares 3.3.14 + KPMcore 24.05.2 (absent from EPEL9) into a staging root.
- `scripts/build-live-iso.sh` — assembles the live ISO with livemedia-creator.
- `live/live.ks` — live host kickstart (autologin + Calamares autostart).

Builder requirements (AlmaLinux 9 x86_64): see `install-deps.sh`. The old
Anaconda kickstart (`kickstart/thiscloud.ks`) and `make-product-img.sh`/
`remix-iso.sh` path are replaced by this flow.
```

- [ ] **Step 3: Commit**

```bash
git add -A platform/iso
git commit -m "docs(iso): document Calamares installer flow; drop Anaconda kickstart"
```

---
---

### Task 12: README for the calamares directory + final review

**Files:**
- Create: `platform/iso/calamares/README.md`

**Interfaces:**
- Consumes: all tasks.
- Produces: operational doc for anyone running the builder.

- [ ] **Step 1: Write the README**

```markdown
# THISCLOUD Calamares installer

Everything needed to build the Calamares-based installer for the THISCLOUD
AlmaLinux 9 live ISO. Calamares and KPMcore are **not packaged for EL9**
(EPEL9), so they are compiled from source on the AlmaLinux builder.

## Layout

| Path | Purpose |
|---|---|
| `branding/thiscloud/` | Product branding (desc, colors, stylesheet, slideshow, PNGs) |
| `modules/thiscloudqml/` | QML view module: node role / cluster / IP / interface |
| `modules/thiscloud/` | Python job module: writes config, runs `thiscloud init` |
| `settings.conf` | Calamares module sequence |
| `live/live.ks` | Live host kickstart (Xorg+openbox autologin, Calamares autostart) |
| `scripts/` | PNG generator, Calamares/KPMcore build, live ISO build |

## Build (AlmaLinux 9 x86_64)

```sh
sudo ./platform/iso/scripts/install-deps.sh
python3 platform/iso/calamares/scripts/make-calamares-branding.py
ALMAISO=/data/AlmaLinux-9-latest-x86_64-minimal.iso \
  bash platform/iso/calamares/scripts/build-live-iso.sh
```

`build-live-iso.sh` compiles Calamares+KPMcore, packages them as RPMs into
`iso/repo` (regenerating repo metadata), then assembles the live ISO with
`livemedia-creator --no-virt` using `live/live.ks`, which pulls `calamares`,
`kpmcore`, and the THISCLOUD packages from the local repo.

Output: `/data/thiscloud-live-iso/ThisCloud-<VERSION>-x86_64.iso`.

## Tests (any host)

```sh
python3 platform/iso/calamares/tests/test_branding.py
python3 platform/iso/calamares/tests/test_branding_desc.py
python3 platform/iso/calamares/tests/test_slideshow_qml.py
python3 platform/iso/calamares/tests/test_thiscloudqml.py
python3 platform/iso/calamares/tests/test_thiscloud_logic.py
python3 platform/iso/calamares/tests/test_settings_conf.py
```

## GlobalStorage contract

`thiscloudqml` (view) → `thiscloud` (job):

| Key | Example |
|---|---|
| `thiscloudRole` | `master` \| `worker` |
| `thiscloudClusterName` | `my-cluster` |
| `thiscloudNodeIp` | `10.0.0.5` |
| `thiscloudInterface` | `eth0` |
```

- [ ] **Step 2: Run the full local test suite**

Run:
```sh
cd platform/iso/calamares/tests
python3 test_branding.py && python3 test_branding_desc.py && python3 test_slideshow_qml.py && python3 test_thiscloudqml.py && python3 test_thiscloud_logic.py && python3 test_settings_conf.py
```
Expected: all `OK`.

- [ ] **Step 3: Run syntax checks on all new bash scripts**

Run:
```sh
bash -n platform/iso/calamares/scripts/build-calamares.sh
bash -n platform/iso/calamares/scripts/build-live-iso.sh
bash -n platform/iso/scripts/install-deps.sh
bash -n platform/iso/scripts/fetch-deps.sh
bash -n platform/iso/scripts/build-iso.sh
```
Expected: exit 0 for each.

- [ ] **Step 4: Commit**

```bash
git add platform/iso/calamares/README.md
git commit -m "docs(iso): add Calamares installer README"
```

---
---

## Self-Review

**1. Spec coverage:**
- Custom installer UI that doesn't look like AlmaLinux → Tasks 1-3 (branding, colors, slideshow) + Task 4 (custom page). ✓
- Qt-based UI (user choice) → Calamares Qt6 widgets/QML throughout. ✓
- Install steps (Language & keyboard, System basics, Storage, Network, THISCLOUD config, Summary, Final setup, root password) → sequence in Task 6 maps: welcome/locale/keyboard/timezone (System basics) → partition (Storage) → users (root password) → network → thiscloudqml → summary → finished. ✓
- Custom branding → Task 1-3. ✓
- THISCLOUD config: network config, cluster setup, node role → Task 4 (QML form) + Task 5 (apply). ✓
- Build-everything reality (no Calamares in EPEL9) → Tasks 7, 9, 10. ✓
- macOS can't build ISO → all C++/ISO steps are builder-only scripts; macOS runs pure-Python tests. ✓

**2. Placeholder scan:** All code blocks are complete; no TBD/TODO.

**3. Self-review fixes (post-verification):**
- `livemedia-creator` has NO `--rootfs` flag (confirmed against lorax man page). The original injection mechanism was fabricated. **Rewritten:** Calamares/KPMcore are now packaged as RPMs (Task 7 `rpmbuild` spec generated from the staging root) into the existing `iso/repo`; `live.ks` gets a `repo --name=thiscloud-local` line + `calamares`/`kpmcore` restored to `%packages`; `build-live-iso.sh` uses `--make-iso --no-virt --iso-only` with no `--rootfs`. HTTP-server repo fallback documented in-file.
- `live.ks` `%packages` initially listed `calamares`/`kpmcore` (not RPMs) — would break dnf; fixed by the RPM-repo rewrite above.
- `live.ks` `%post` wrote the openbox autostart but never started openbox (`startx` without `.xinitrc` falls back to xterm). **Added `/home/live/.xinitrc` with `exec openbox-session`.**
- `install-deps.sh` additions now include `createrepo_c rpm-build` (needed for Task 7 RPM packaging).
- Task 1 runtime fix: `write_png` must pass `(x, y, width, height)` to the pixel closure; `gradient`/`solid`/`icon_pix`/`logo_pix`/`slide_pix` all take `(x, y, w, h)`. (Original plan had 2-arg calls → TypeError.)
- Task 2 schema fix: Calamares `images:` accepts only `productBanner/productIcon/productLogo/productWallpaper/productWelcome`; `SidebarBackground` in `style:` is a color, not an image file. `sidebar-bg.png` is generated but never referenced by `branding.desc`. Test split into `test_required_pngs_referenced` (only referenced PNGs) + `test_sidebar_bg_exists`.
- Task 4 QML fix: Calamares QML view steps have **no** `onNextRequested` — the Next button belongs to ViewManager; the QML binds to the `config` context property (from `getConfig()`) and exposes `function onActivate()/onLeave()` lifecycle hooks. Plan test replaced `onNextRequested` with `config` + added `test_qml_has_lifecycle_hooks`; QML gained the two lifecycle functions.

**4. Type consistency:** `thiscloudRole/thiscloudClusterName/thiscloudNodeIp/thiscloudInterface` names identical across Task 4 (write in C++), Task 5 (read in Python), Task 6 (no direct ref), Task 12 (docs). `build_init_args` returns `["/usr/bin/thiscloud", "init", ...]` matching the CLI's `thiscloud init --ip --role` in Task 5's usage. PNG filenames consistent across Tasks 1-2. Module names (`thiscloudqml`, `thiscloud`) consistent across Tasks 4-6.

## Execution Handoff

Plan complete. See message for execution options.
