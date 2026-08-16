# THISCLOUD Documentation

Documentación operativa de THISCLOUD, organizada por tema. Esta documentación describe **cómo usar** el producto desde la CLI (`thiscloud`) y cómo encajan los componentes. Para guías de desarrollo, consulta `AGENTS.md` en la raíz y `platform/CLAUDE.md`.

## Índice de temas

| Carpeta | Contenido |
|---|---|
| [`cluster/`](cluster/cluster.md) | Inicialización, alta de nodos, unirse a un clúster, gestión de nodos, estado, HA |
| [`vm/`](vm/vm.md) | Ciclo de vida de máquinas virtuales: crear, arrancar, snapshot, clonar, migrar, discos y NICs |
| [`network/`](network/network.md) | Redes virtuales: creación, listado y borrado con CIDR, gateway y VLAN |
| [`storage/`](storage/storage.md) | Pools de almacenamiento replicado: Linstor, DRBD y local |
| [`images/`](images/images.md) | Imágenes y plantillas de VM: registro, verificación SHA-256, formatos |
| [`architecture/`](architecture/architecture.md) | Arquitectura de los cinco componentes y flujo de datos entre ellos |
| [`update/`](update/update.md) | Actualización del sistema: comprobación, instalación y rollback |

## Cómo usar estos documentos

- Cada documento asume un daemon THISCLOUD en funcionamiento accesible vía HTTP.
- Los comandos usan los valores por defecto de las variables de entorno (ver tabla abajo).
- Todas las rutas de la API del daemon viven bajo `/api/v1`; el contrato completo está en [`docs/api/openapi.yaml`](../docs/api/openapi.yaml).
- Los ejemplos de salida son representativos: la forma exacta de las tablas puede variar entre versiones.

## Variables de entorno relevantes

| Variable | Valor por defecto | Uso |
|---|---|---|
| `THISCLOUD_API_URL` | `http://127.0.0.1:8080` | URL de la API del daemon (CLI) |
| `THISCLOUD_API_URL` (CLI base) | `http://127.0.0.1:8080/api/v1` | Base de recursos de la CLI |
| `THISCLOUD_CONFIG_DIR` | `/etc/thiscloud` | Directorio de configuración |
| `THISCLOUD_DATA_DIR` | `/var/lib/thiscloud` | Directorio de datos |
| `THISCLOUD_UPDATE_REPO` | `THISJOWI/THISCLOUD` | Repositorio GitHub para actualizaciones |
| `THISCLOUD_UPDATE_TOKEN` | *(vacío)* | Token para repos privados en actualizaciones |

## Referencias rápidas

- **Contrato de API (OpenAPI):** `docs/api/openapi.yaml`
- **Guía de desarrollo:** `AGENTS.md` (raíz), `platform/CLAUDE.md`
- **Construcción del ISO:** `platform/iso/README.md`