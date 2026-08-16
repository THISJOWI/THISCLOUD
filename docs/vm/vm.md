# VM — ciclo de vida de máquinas virtuales

Guía operativa para gestionar máquinas virtuales con `thiscloud vm ...`. Los subcomandos aceptan el ID o el nombre de la VM salvo que se indique.

## Índice

- [Listar VMs](#listar-vms)
- [Ver detalle](#ver-detalle)
- [Crear una VM](#crear-una-vm)
- [Arrancar / parar / borrar](#arrancar--parar--borrar)
- [Snapshots](#snapshots)
- [Clonar](#clonar)
- [Redimensionar](#redimensionar)
- [Migrar una VM (live migration)](#migrar-una-vm-live-migration)
- [Discos adicionales](#discos-adicionales)
- [NICs adicionales](#nics-adicionales)
- [Consola](#consola)

## Listar VMs

```sh
thiscloud vm list
```

Tabla con `ID`, `NAME`, `CPUS` y `STATUS`.

## Ver detalle

```sh
thiscloud vm show <VM>
```

Muestra: ID, nombre, CPUs, memoria (MB), disco (`disk_path`), kernel y argumentos, estado, UEFI, TPM, si es plantilla, cloud-init, redes, discos de datos y snapshots.

## Crear una VM

```sh
thiscloud vm create --name web-01 --cpus 2 --memory 4096
```

Flags:

| Flag | Por defecto | Descripción |
|---|---|---|
| `--name` | *(obligatorio)* | Nombre de la VM |
| `--cpus` | `1` | Número de vCPUs |
| `--memory` | `1024` | Memoria en MB |
| `--disk` | auto | Ruta del disco qcow2; si se omite, `/var/lib/thiscloud/vms/<name>.qcow2` |
| `--kernel` | *(vacío)* | Ruta del binario del kernel para cloud-hypervisor |
| `--kernel-args` | *(vacío)* | Argumentos de arranque del kernel |
| `--network` | *(vacío)* | Nombre o ID de red (repetible) |
| `--cloud-init` | *(vacío)* | Cloud-config aplicado en el primer arranque |
| `--uefi` | `false` | Arranque con firmware UEFI (OVMF) |
| `--tpm` | `false` | Dispositivo vTPM (requiere UEFI) |
| `--template` | `false` | Marcar la VM como plantilla reutilizable |
| `--node` | *(vacío)* | Colocar la VM en un nodo concreto (vacío = mejor ajuste) |
| `--affinity` | *(vacío)* | Label de afinidad del planificador (repetible) |
| `--anti-affinity` | *(vacío)* | Label de anti-afinidad del planificador (repetible) |
| `--image` | *(vacío)* | Arrancar desde una imagen registrada (nombre o id); deriva `disk_path` |

Ejemplos:

```sh
# VM básica con dos redes
thiscloud vm create --name web-01 --cpus 2 --memory 4096 --network lan --network dmz

# VM UEFI + vTPM con cloud-init
thiscloud vm create --name secured --cpus 4 --memory 8192 --uefi --tpm \
  --cloud-init '#cloud-config
users:
  - name: admin
    sudo: ALL=(ALL) NOPASSWD:ALL'

# VM a partir de una imagen registrada, fijada en un nodo con afinidad
thiscloud vm create --name db-01 --image alma-9 --node worker-01 --affinity ssd=high
```

## Arrancar / parar / borrar

```sh
thiscloud vm start <VM>
thiscloud vm stop <VM>
thiscloud vm delete <VM>
```

## Snapshots

```sh
# Crear un snapshot
thiscloud vm snapshot <VM> --name <NOMBRE_SNAPSHOT>

# Restaurar la VM desde un snapshot
thiscloud vm restore <VM> --snapshot-id <SNAPSHOT_ID>
```

Los snapshots existentes aparecen en `thiscloud vm show <VM>`.

## Clonar

Clona una VM o plantilla en una VM nueva:

```sh
thiscloud vm clone <VM_ORIGEN> --name <NUEVO_NOMBRE>
```

## Redimensionar

```sh
thiscloud vm resize <VM> [--cpus N] [--memory N]
```

Ambos flags por defecto a `0`, que significa "sin cambio". Se pueden cambiar solo CPUs, solo memoria o ambos.

## Migrar una VM (live migration)

```sh
thiscloud vm migrate <VM> --target-node <NODE_ID>
```

Mueve la VM a otro nodo del clúster (usado por HA; el destino se identifica por ID de nodo, consultable con `thiscloud node list`).

## Discos adicionales

```sh
# Añadir un disco de datos
thiscloud vm attach-disk <VM> --path /var/lib/thiscloud/vms/<VM>-data.qcow2 --size-gb 50

# Desconectar un disco
thiscloud vm detach-disk <VM> --disk-id <DISK_ID>
```

## NICs adicionales

```sh
# Conectar una NIC (tap/red)
thiscloud vm attach-nic <VM> --tap <TAP>

# Desconectar una NIC
thiscloud vm detach-nic <VM> --tap <TAP>
```

## Consola

```sh
thiscloud vm console <VM>
```

Devuelve la URL de acceso a la consola (VNC/vsock) de la VM.