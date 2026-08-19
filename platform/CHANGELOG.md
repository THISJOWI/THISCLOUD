# Changelog

## [0.4.0-alpha.1](https://github.com/THISJOWI/THISCLOUD/compare/v0.4.0-alpha...v0.4.0-alpha.1) (2026-08-18)


### Bug Fixes

* **iso:** fetch kpmcore source from GitHub mirror, not invent.kde.org ([4a3e024](https://github.com/THISJOWI/THISCLOUD/commit/4a3e0249a78886d58533acdb3da5fffc0de5d453))

## [0.4.0-alpha](https://github.com/THISJOWI/THISCLOUD/compare/v0.3.0...v0.4.0-alpha) (2026-08-18)


### Features

* **compute:** device hotplug and memory ballooning with metric publication (T1.6, T1.7) ([6c50575](https://github.com/THISJOWI/THISCLOUD/commit/6c50575a8e43e7148b1e6e3aedb85ca3469aada6))
* **go-api:** add generic daemon proxy and VM id reconciliation ([567a817](https://github.com/THISJOWI/THISCLOUD/commit/567a817e6b9236af254952acc55465fadb2bb6e5))
* **go-api:** proxy VM disks from daemon via /api/v1/vm-disks ([e3cae70](https://github.com/THISJOWI/THISCLOUD/commit/e3cae7051c054b8ec7a15a33ac9e38dd1ea532f6))
* **iso-upload:** register images by local file upload ([f40f6c6](https://github.com/THISJOWI/THISCLOUD/commit/f40f6c660773f44db015435a26872a9b797ecc51))
* **iso:** add Calamares settings.conf with THISCLOUD module sequence ([c9f173a](https://github.com/THISJOWI/THISCLOUD/commit/c9f173ac41600f1c9e84ebdd3cc00be4daccbe5a))
* **iso:** add live environment kickstart hosting Calamares ([2cad06f](https://github.com/THISJOWI/THISCLOUD/commit/2cad06f6ed8b137f8e0d940474eabdcc13bf46dd))
* **iso:** add THISCLOUD Calamares branding desc, colors, stylesheet ([7baae18](https://github.com/THISJOWI/THISCLOUD/commit/7baae18865369094ed0c9ccef3863cd1b161059c))
* **iso:** add THISCLOUD Calamares slideshow QML ([1e022e8](https://github.com/THISJOWI/THISCLOUD/commit/1e022e8fdcf2ffdad06b6528833558d69c9cc0af))
* **iso:** add THISCLOUD config QML view module for Calamares ([c91c6c5](https://github.com/THISJOWI/THISCLOUD/commit/c91c6c5053fc51319b29b3d77d35e8717d3d7dcd))
* **iso:** add THISCLOUD python job module for Calamares ([28e55af](https://github.com/THISJOWI/THISCLOUD/commit/28e55afebb67eaac2b89e48ff292ee50de9f20cd))
* **iso:** generate Calamares branding pixmaps for THISCLOUD ([40a22be](https://github.com/THISJOWI/THISCLOUD/commit/40a22beccc55f6145bc239fd787fa88b1472a0af))
* **iso:** route ISO build through Calamares live installer ([26b6842](https://github.com/THISJOWI/THISCLOUD/commit/26b68426a45aecbd51fd927514ae086403d0c75d))
* **platform:** cluster state backup and restore with retention (T0.7) ([4fafef4](https://github.com/THISJOWI/THISCLOUD/commit/4fafef41e579fcfcd4afdfcaff51e7f3c3fd12ad))
* **platform:** health and readiness checks across daemon, go-api, cli, web (T0.8) ([fc2ebec](https://github.com/THISJOWI/THISCLOUD/commit/fc2ebec69fab9e65d4dd77854bd88b56c09f5c8d))


### Bug Fixes

* **cli:** make status report live daemon state ([3e917a8](https://github.com/THISJOWI/THISCLOUD/commit/3e917a845d7f79d77f450f6bbe2ead47b03137e4))
* **cli:** update ignores broken repos when installing RPMs ([b36cbb6](https://github.com/THISJOWI/THISCLOUD/commit/b36cbb63363f8b3dc7f56850439fe68a65f72813))
* **compute:** assign UUID in create_vm module layer, not just http handler ([c331286](https://github.com/THISJOWI/THISCLOUD/commit/c3312864876b548835ad9a84c099fe927c2e2ea8))
* **daemon:** apply effective state in node get endpoint ([03a980c](https://github.com/THISJOWI/THISCLOUD/commit/03a980c52665cfab33bc276656ae0fadcc77daf4))
* **go-api:** emit daemon-compatible payloads and delete storage by name ([e23ab04](https://github.com/THISJOWI/THISCLOUD/commit/e23ab043128c12e99e0d383f714cfbea184dd522))
* **go-api:** reject VMs the daemon refuses and pick nodes from a list ([b9e78df](https://github.com/THISJOWI/THISCLOUD/commit/b9e78dfe945374dffb65b719b540a1e8e401cc29))
* **go-api:** sync VM id to daemon and backfill legacy ids ([5cabc61](https://github.com/THISJOWI/THISCLOUD/commit/5cabc61cbb2b4794a7c60d639f31ab22218c3601))
* **iso:** point live ISO repo checks at iso/repo/thiscloud dnf repo ([9ca2ef7](https://github.com/THISJOWI/THISCLOUD/commit/9ca2ef714ea4ce0aced6b1a016eaef5d115f4571))
* VM deletion 405 on empty-id state + node name-or-id resolution ([31f46ba](https://github.com/THISJOWI/THISCLOUD/commit/31f46bab753f75c647927824a1bb6bcd6ee13f23))
