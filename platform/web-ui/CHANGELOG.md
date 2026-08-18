# Changelog

## [0.4.0-alpha](https://github.com/THISJOWI/THISCLOUD/compare/v0.3.0...v0.4.0-alpha) (2026-08-18)


### Features

* **compute:** device hotplug and memory ballooning with metric publication (T1.6, T1.7) ([6c50575](https://github.com/THISJOWI/THISCLOUD/commit/6c50575a8e43e7148b1e6e3aedb85ca3469aada6))
* **go-api:** add generic daemon proxy and VM id reconciliation ([567a817](https://github.com/THISJOWI/THISCLOUD/commit/567a817e6b9236af254952acc55465fadb2bb6e5))
* **iso-upload:** register images by local file upload ([f40f6c6](https://github.com/THISJOWI/THISCLOUD/commit/f40f6c660773f44db015435a26872a9b797ecc51))
* **platform:** health and readiness checks across daemon, go-api, cli, web (T0.8) ([fc2ebec](https://github.com/THISJOWI/THISCLOUD/commit/fc2ebec69fab9e65d4dd77854bd88b56c09f5c8d))
* **release:** add release-please configs and sync web-ui baseline version ([29b9dc8](https://github.com/THISJOWI/THISCLOUD/commit/29b9dc83fa244f5cffb83037ba0fad1e4f867972))
* **web-ui:** add listVmDisks API helper and contract test ([f8c1b94](https://github.com/THISJOWI/THISCLOUD/commit/f8c1b9491b75eaaeddea0ccc213121674b0af14c))


### Bug Fixes

* **go-api:** reject VMs the daemon refuses and pick nodes from a list ([b9e78df](https://github.com/THISJOWI/THISCLOUD/commit/b9e78dfe945374dffb65b719b540a1e8e401cc29))
* VM deletion 405 on empty-id state + node name-or-id resolution ([31f46ba](https://github.com/THISJOWI/THISCLOUD/commit/31f46bab753f75c647927824a1bb6bcd6ee13f23))
