# Changelog

## [0.5.0-alpha.1](https://github.com/THISJOWI/THISCLOUD/compare/v0.4.0-alpha.1...v0.5.0-alpha.1) (2026-08-19)


### Features

* **auth:** JWT auth middleware for daemon — T0.2 ([678e8d2](https://github.com/THISJOWI/THISCLOUD/commit/678e8d21bfe1f6f284ca78eac8da284953ba54c3))
* **auth:** T0.3 RBAC — role-based access control, login public, env override ([8623552](https://github.com/THISJOWI/THISCLOUD/commit/8623552e6a68e63bcdb860a7216724fcf2570fc4))
* **cli:** add thiscloud update with GitHub Releases ([2ca5d34](https://github.com/THISJOWI/THISCLOUD/commit/2ca5d348321b939fcae596e75838473be1ddfbeb))
* **compute:** device hotplug and memory ballooning with metric publication (T1.6, T1.7) ([6c50575](https://github.com/THISJOWI/THISCLOUD/commit/6c50575a8e43e7148b1e6e3aedb85ca3469aada6))
* **compute:** HA live migration + automatic failover (T1.4) ([19852fd](https://github.com/THISJOWI/THISCLOUD/commit/19852fdce335d90320a039a9587d56361b793d5d))
* **go-api:** add generic daemon proxy and VM id reconciliation ([567a817](https://github.com/THISJOWI/THISCLOUD/commit/567a817e6b9236af254952acc55465fadb2bb6e5))
* **go-api:** proxy VM disks from daemon via /api/v1/vm-disks ([e3cae70](https://github.com/THISJOWI/THISCLOUD/commit/e3cae7051c054b8ec7a15a33ac9e38dd1ea532f6))
* **images:** image registry (T1.2) with template support + VM boot from image ([ebb1830](https://github.com/THISJOWI/THISCLOUD/commit/ebb183019b5dd01420769194cacac4b833b66cbc))
* **iso-upload:** register images by local file upload ([f40f6c6](https://github.com/THISJOWI/THISCLOUD/commit/f40f6c660773f44db015435a26872a9b797ecc51))
* **iso:** add Calamares settings.conf with THISCLOUD module sequence ([c9f173a](https://github.com/THISJOWI/THISCLOUD/commit/c9f173ac41600f1c9e84ebdd3cc00be4daccbe5a))
* **iso:** add live environment kickstart hosting Calamares ([2cad06f](https://github.com/THISJOWI/THISCLOUD/commit/2cad06f6ed8b137f8e0d940474eabdcc13bf46dd))
* **iso:** add THISCLOUD Calamares branding desc, colors, stylesheet ([7baae18](https://github.com/THISJOWI/THISCLOUD/commit/7baae18865369094ed0c9ccef3863cd1b161059c))
* **iso:** add THISCLOUD Calamares slideshow QML ([1e022e8](https://github.com/THISJOWI/THISCLOUD/commit/1e022e8fdcf2ffdad06b6528833558d69c9cc0af))
* **iso:** add THISCLOUD config QML view module for Calamares ([c91c6c5](https://github.com/THISJOWI/THISCLOUD/commit/c91c6c5053fc51319b29b3d77d35e8717d3d7dcd))
* **iso:** add THISCLOUD python job module for Calamares ([28e55af](https://github.com/THISJOWI/THISCLOUD/commit/28e55afebb67eaac2b89e48ff292ee50de9f20cd))
* **iso:** generate Calamares branding pixmaps for THISCLOUD ([40a22be](https://github.com/THISJOWI/THISCLOUD/commit/40a22beccc55f6145bc239fd787fa88b1472a0af))
* **iso:** route ISO build through Calamares live installer ([26b6842](https://github.com/THISJOWI/THISCLOUD/commit/26b68426a45aecbd51fd927514ae086403d0c75d))
* **network:** VPC routers, DHCP/DNS, floating IPs (T2.1) ([cdb1211](https://github.com/THISJOWI/THISCLOUD/commit/cdb1211fd174d2903edafa07ccc517b098b7a37e))
* **node:** self-registering cluster agent with real usage heartbeats ([2f24f92](https://github.com/THISJOWI/THISCLOUD/commit/2f24f92a85c2a545ab94b10ef90824b3d3d256cd))
* **nodes:** expose cluster nodes to web UI (sidebar + dashboard) ([9c8dbbf](https://github.com/THISJOWI/THISCLOUD/commit/9c8dbbffa77e4bec6efc4bb66f6ff2fe6af92729))
* **platform:** cluster state backup and restore with retention (T0.7) ([4fafef4](https://github.com/THISJOWI/THISCLOUD/commit/4fafef41e579fcfcd4afdfcaff51e7f3c3fd12ad))
* **platform:** health and readiness checks across daemon, go-api, cli, web (T0.8) ([fc2ebec](https://github.com/THISJOWI/THISCLOUD/commit/fc2ebec69fab9e65d4dd77854bd88b56c09f5c8d))
* **platform:** multi-node cluster support, quotas, audit + scheduler ([3b474b4](https://github.com/THISJOWI/THISCLOUD/commit/3b474b47bec26d5093c3edf97dbb17d8fafc3774))
* **release:** add release-please configs and sync web-ui baseline version ([29b9dc8](https://github.com/THISJOWI/THISCLOUD/commit/29b9dc83fa244f5cffb83037ba0fad1e4f867972))
* S3 RadosGW multitenant (T3.2) + Prometheus metrics (T5.1) ([6f6d5d0](https://github.com/THISJOWI/THISCLOUD/commit/6f6d5d0ed008123797db32cba92b654807e02be1))
* **storage:** CephBackend for RBD pools (T3.1) ([82cd938](https://github.com/THISJOWI/THISCLOUD/commit/82cd9384048629c3a9944ebd7dc046ef6f274f4a))
* **web-ui:** add listVmDisks API helper and contract test ([f8c1b94](https://github.com/THISJOWI/THISCLOUD/commit/f8c1b9491b75eaaeddea0ccc213121674b0af14c))
* **web:** liquid-glass UI redesign + fix console/VM bugs ([62cb930](https://github.com/THISJOWI/THISCLOUD/commit/62cb930ec8f545163fc14b123b2f6f099d6589a0))
* **web:** match Stitch mockup — Material 3 tokens, Resource Tree nav, telemetry cards ([1537ac4](https://github.com/THISJOWI/THISCLOUD/commit/1537ac43ff0e8987164992a0aa26b5d22fff2b03))
* **web:** Stitch redesign + Proxmox-style VM creation modal ([e0c7c05](https://github.com/THISJOWI/THISCLOUD/commit/e0c7c054b831fca2e4fb7a27bacba36d65e9995b))


### Bug Fixes

* **auth:** root always admin, logout uses correct host ([2a8ee5c](https://github.com/THISJOWI/THISCLOUD/commit/2a8ee5cce59dc7fb64e6fc36d4c8d379449e0318))
* **ci,iso:** secure ISO before container rm, bump 0.1 versions, require musl-gcc ([19b35b5](https://github.com/THISJOWI/THISCLOUD/commit/19b35b595f7fd79e77fa5b0fb351cab92110e673))
* **cli:** make status report live daemon state ([3e917a8](https://github.com/THISJOWI/THISCLOUD/commit/3e917a845d7f79d77f450f6bbe2ead47b03137e4))
* **clippy:** drop redundant let _ in test init_secret calls ([57c23c5](https://github.com/THISJOWI/THISCLOUD/commit/57c23c500e6a169e52fa1319be1dd3b2580812ea))
* **clippy:** etcd integration tests wait on spawned processes ([1c473d1](https://github.com/THISJOWI/THISCLOUD/commit/1c473d1f5f056861430e135d1f7a8b6e711312de))
* **clippy:** resolve -D warnings in auth and test_config ([4fd3cb9](https://github.com/THISJOWI/THISCLOUD/commit/4fd3cb98312910342f180a4e3bc64a16c878a42b))
* **cli:** update ignores broken repos when installing RPMs ([b36cbb6](https://github.com/THISJOWI/THISCLOUD/commit/b36cbb63363f8b3dc7f56850439fe68a65f72813))
* **compute:** assign UUID in create_vm module layer, not just http handler ([c331286](https://github.com/THISJOWI/THISCLOUD/commit/c3312864876b548835ad9a84c099fe927c2e2ea8))
* **daemon:** apply effective state in node get endpoint ([03a980c](https://github.com/THISJOWI/THISCLOUD/commit/03a980c52665cfab33bc276656ae0fadcc77daf4))
* **go-api:** emit daemon-compatible payloads and delete storage by name ([e23ab04](https://github.com/THISJOWI/THISCLOUD/commit/e23ab043128c12e99e0d383f714cfbea184dd522))
* **go-api:** reject VMs the daemon refuses and pick nodes from a list ([b9e78df](https://github.com/THISJOWI/THISCLOUD/commit/b9e78dfe945374dffb65b719b540a1e8e401cc29))
* **go-api:** serialize empty resource list as [] not null ([eecc508](https://github.com/THISJOWI/THISCLOUD/commit/eecc508c50d123d5238c2bc95a7369625c070658))
* **go-api:** sync VM id to daemon and backfill legacy ids ([5cabc61](https://github.com/THISJOWI/THISCLOUD/commit/5cabc61cbb2b4794a7c60d639f31ab22218c3601))
* **iso:** bundle etcd static binary; daemon survives missing etcd ([b122981](https://github.com/THISJOWI/THISCLOUD/commit/b1229811d7a8e28a2f746a2ab946e90160c74584))
* **iso:** guard ls|head pipes against SIGPIPE ([9f514cb](https://github.com/THISJOWI/THISCLOUD/commit/9f514cbbf751ad0d201e26323e00dc785ba28d63))
* **iso:** point live ISO repo checks at iso/repo/thiscloud dnf repo ([9ca2ef7](https://github.com/THISJOWI/THISCLOUD/commit/9ca2ef714ea4ce0aced6b1a016eaef5d115f4571))
* **node:** daemon self-heartbeat keeps nodes online ([84a2944](https://github.com/THISJOWI/THISCLOUD/commit/84a294445ac981f45c6fbc1997f2a3a3f9053305))
* **node:** deterministic MemoryNodeStore.list() ordering by id ([3b03f74](https://github.com/THISJOWI/THISCLOUD/commit/3b03f7428de2c8112b7056c66691af888f7fe517))
* VM deletion 405 on empty-id state + node name-or-id resolution ([31f46ba](https://github.com/THISJOWI/THISCLOUD/commit/31f46bab753f75c647927824a1bb6bcd6ee13f23))
* **web-ui:** generate SESSION_SECRET at boot and open port 3000 ([b3f14ab](https://github.com/THISJOWI/THISCLOUD/commit/b3f14ab24eaa711b8fe556d635d4215ed5514d58))

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
