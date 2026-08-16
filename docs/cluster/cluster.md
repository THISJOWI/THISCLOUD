# Cluster — inicialización, nodos y alta

Guía operativa para montar un clúster THISCLOUD, unir nodos y gestionarlos. Todos los comandos requieren el binario `thiscloud` instalado y apuntan al daemon vía `THISCLOUD_API_URL` (por defecto `http://127.0.0.1:8080`).

## Índice

- [Conceptos](#conceptos)
- [Inicializar el primer nodo (master)](#inicializar-el-primer-nodo-master)
- [Unirse a un clúster (worker)](#unirse-a-un-clúster-worker)
- [Estado del clúster](#estado-del-clúster)
- [Gestión de nodos](#gestión-de-nodos)
- [Drenado de nodos](#drenado-de-nodos)
- [High Availability (HA)](#high-availability-ha)

## Conceptos

- **Master**: nodo que registra al resto y actúa de punto de autoridad del clúster.
- **Worker**: nodo de cómputo registrado en un master.
- **Identidad de nodo**: el daemon guarda su `node.id`, `role` y lista de masters en `config.toml` (`[node]`).
- **Quórum**: en modo HA, el número mínimo de nodos online exigido para autorizar un failover. El quórum efectivo es `max(quorum, nodos_registrados/2 + 1)`.

## Inicializar el primer nodo (master)

Crea la configuración base, los directorios de datos y una `config.toml` con el rol indicado.

```sh
thiscloud init --ip 192.168.1.10 --role master
```

Flags:

| Flag | Por defecto | Descripción |
|---|---|---|
| `--ip` | `127.0.0.1` | IP de este nodo |
| `--role` | `master` | `master` o `worker` |

Qué hace:

1. Crea `$THISCLOUD_CONFIG_DIR/config.toml` (por defecto `/etc/thiscloud/config.toml`) con secciones `[cluster]`, `[node]`, `[compute]`, `[network]`, `[storage]` y `[marketplace]`.
2. Crea los directorios de datos en `$THISCLOUD_DATA_DIR` (`vms/`, `storage/`).
3. Imprime los siguientes pasos: editar `config.toml`, arrancar el daemon (`systemctl start thiscloudd`) y comprobar con `thiscloud status`.

Después de inicializar, arranca el daemon y verifica:

```sh
systemctl start thiscloudd
thiscloud status
```

## Unirse a un clúster (worker)

Registra el nodo actual como worker en el master y guarda su identidad en `config.toml`.

```sh
thiscloud join --master http://192.168.1.10 --ip 192.168.1.11
```

Flags:

| Flag | Obligatorio | Descripción |
|---|---|---|
| `--master` | Sí | URL de la API del master (se añade `/api/v1` automáticamente) |
| `--ip` | No | IP local que anunciar; por defecto `127.0.0.1` |

Qué hace:

1. Construye `POST /api/v1/nodes` hacia el master con `{name: hostname, address: <ip>:8080, role: "worker", cpus_total, memory_total_mb, labels}`.
2. Si el master rechaza el registro (HTTP no 2xx), aborta con el código de estado.
3. Persiste en `config.toml` la sección `[node]` con el `id` devuelto, `role = "worker"` y la lista `masters` con la URL del master.

> El worker debe poder alcanzar al master por HTTP. Si no existe `config.toml` local, el comando avisa de que la identidad no se guardó.

Registro manual equivalente con `node register`:

```sh
thiscloud node register --name worker-01 \
  --address 192.168.1.11:8080 \
  --role worker \
  --cpus 8 \
  --memory-mb 16384 \
  --label region=eu
```

## Estado del clúster

```sh
thiscloud status
```

Muestra:

- Estado del daemon: lee `/var/run/thiscloudd.pid`; si no existe, sondea `GET /api/v1/healthz`.
- Nombre del clúster y lista de nodos desde `config.toml`.

```sh
thiscloud status
# THISCLOUD Cluster Status
# ========================
# Daemon: Running (PID: 1234)
#
# Cluster: thiscloud-cluster
# Nodes: 2
#   1. 192.168.1.10 (master)
#   2. 192.168.1.11 (worker)
```

## Gestión de nodos

### Listar nodos

```sh
thiscloud node list
```

Tabla con `ID`, `NAME`, `ROLE`, `STATE` (online/offline/draining, coloreado si el terminal lo soporta), `CPUS` (usados/totales), `MEMORY` (usada/total, humanizada en G/M), `DRAIN` y `LAST SEEN` (humanizado: `now`, `5s`, `3m`, `2h`, `1d`).

### Ver detalle de un nodo

```sh
thiscloud node show <NODE_ID>
```

Muestra identidad, dirección, estado, recursos usados/totales, número de VMs, si está drenando, último visto y labels.

### Registrar un nodo manualmente

```sh
thiscloud node register --name <NOMBRE> --address <IP:PUERTO> [--role master|worker] [--cpus N] [--memory-mb N] [--label K=V ...]
```

Flags:

| Flag | Por defecto | Descripción |
|---|---|---|
| `--name` | *(obligatorio)* | Nombre del nodo |
| `--address` | *(obligatorio)* | Dirección del agente (`ip:port`) |
| `--role` | `worker` | `master` o `worker` |
| `--cpus` | `0` | CPUs totales (0 = desconocido) |
| `--memory-mb` | `0` | Memoria total en MB (0 = desconocido) |
| `--label` | *(vacío)* | Label de afinidad (repetible) |

### Eliminar un nodo

```sh
thiscloud node remove <NODE_ID>
```

Quita el nodo del registro del clúster.

## Drenado de nodos

Drenar excluye un nodo del planificador (no recibe VMs nuevas); útil para mantenimiento.

```sh
thiscloud node drain <NODE_ID>      # empieza a drenar
thiscloud node undrain <NODE_ID>    # vuelve a admitir carga
```

El estado del nodo pasa a `draining` / `online` respectivamente.

## High Availability (HA)

El failover automático de VMs está controlado por la sección `[ha]` de `config.toml`:

| Clave | Por defecto | Descripción |
|---|---|---|
| `enabled` | `true` | Interruptor maestro del failover HA |
| `quorum` | `2` | Mínimo de nodos online para autorizar failover |
| `scan_interval_secs` | `10` | Segundos entre escaneos automáticos de HA |

Ejemplo:

```toml
[ha]
enabled = true
quorum = 2
scan_interval_secs = 10
```

Reglas:

- Si los nodos online caen por debajo del quórum efectivo, se bloquea la reubicación (evita split-brain).
- El quórum efectivo es `max(quorum, nodos_registrados / 2 + 1)`.
- Para migrar una VM a otro nodo manualmente, ver [VM migrate](../vm/vm.md#migrar-una-vm-live-migration).