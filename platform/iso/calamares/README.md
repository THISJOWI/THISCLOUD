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

`build-live-iso.sh` compiles Calamares+KPMcore, packages them as a single
`calamares` RPM into `iso/repo` (regenerating repo metadata), then assembles
the live ISO with `livemedia-creator --no-virt` using `live/live.ks`, which
pulls `calamares` and the THISCLOUD packages from the local repo.

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
