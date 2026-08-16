# Images — imágenes y plantillas de VM

Guía operativa para gestionar imágenes y plantillas con `thiscloud image ...`. Una imagen registrada puede usarse para arrancar VMs (`thiscloud vm create --image <NOMBRE_O_ID>`), y las marcadas como plantilla se pueden clonar.

## Índice

- [Listar imágenes](#listar-imágenes)
- [Ver detalle](#ver-detalle)
- [Registrar una imagen](#registrar-una-imagen)
- [Marcar una imagen como plantilla](#marcar-una-imagen-como-plantilla)
- [Eliminar una imagen](#eliminar-una-imagen)

## Listar imágenes

```sh
thiscloud image list
```

Tabla con `ID`, `NAME`, `FORMAT`, `OS`, `VERSION` y `TMPL` (si es plantilla).

## Ver detalle

```sh
thiscloud image show <IMAGEN>
```

Muestra: ID, nombre, fuente, checksum SHA-256, formato, familia de SO, versión, si es plantilla y estado.

## Registrar una imagen

```sh
thiscloud image register --name alma-9 --source https://example.com/alma-9.qcow2 --format qcow2 --os-family alma
```

Flags:

| Flag | Por defecto | Descripción |
|---|---|---|
| `--name` | *(obligatorio)* | Nombre de la imagen |
| `--source` | *(obligatorio)* | Referencia: URL HTTP(S) o ruta local en el pool |
| `--format` | `qcow2` | `qcow2`, `iso`, `raw` o `cloud-init` |
| `--os-family` | `generic` | `generic`, `ubuntu`, `debian`, `fedora`, `alma`, `rocky` |
| `--version` | `latest` | Etiqueta de versión |
| `--sha256` | *(vacío)* | Checksum SHA-256 esperado (verificación) |
| `--template` | `false` | Registrar como plantilla reutilizable |

Ejemplos:

```sh
# Imagen Ubuntu 24.04 verificada por checksum
thiscloud image register --name ubuntu-24.04 \
  --source https://cloud-images.ubuntu.com/noble/current/noble-server-cloudimg-amd64.qcow2 \
  --format qcow2 --os-family ubuntu --version 24.04 \
  --sha256 <CHECKSUM_ESPERADO>

# Imagen local de disco en el pool
thiscloud image register --name golden --source /var/lib/thiscloud/vms/golden.qcow2 --format qcow2 --template
```

## Marcar una imagen como plantilla

```sh
thiscloud image template <IMAGEN>                # marca como plantilla (--template true)
thiscloud image template <IMAGEN> --template false   # deja de ser plantilla
```

Las plantillas pueden clonarse con `thiscloud vm clone`; ver [VM](../vm/vm.md#clonar).

## Eliminar una imagen

```sh
thiscloud image delete <IMAGEN>
```

Acepta ID o nombre.