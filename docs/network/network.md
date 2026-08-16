# Network — redes virtuales

Guía operativa para gestionar redes virtuales con `thiscloud network ...`.

## Índice

- [Listar redes](#listar-redes)
- [Crear una red](#crear-una-red)
- [Eliminar una red](#eliminar-una-red)

## Listar redes

```sh
thiscloud network list
```

Tabla con `ID`, `NAME`, `CIDR` y `GATEWAY`.

## Crear una red

```sh
thiscloud network create --name lan --cidr 10.0.0.0/24
```

Flags:

| Flag | Por defecto | Descripción |
|---|---|---|
| `--name` | *(obligatorio)* | Nombre de la red |
| `--cidr` | *(obligatorio)* | Rango de red, p. ej. `10.0.0.0/24` |
| `--gateway` | `10.0.0.1` | IP del gateway |
| `--vlan` | *(omitido)* | ID de VLAN (opcional) |

Ejemplos:

```sh
# Red plana con gateway por defecto
thiscloud network create --name lan --cidr 10.0.0.0/24

# Red con gateway y VLAN explícitos
thiscloud network create --name dmz --cidr 192.168.50.0/24 --gateway 192.168.50.1 --vlan 50
```

## Eliminar una red

```sh
thiscloud network delete <NETWORK_ID>
```

El borrado se hace por ID (no por nombre). Obtén el ID con `thiscloud network list`.

Las redes creadas se pueden asignar a VMs en el momento de crearlas mediante `thiscloud vm create --network <NOMBRE_O_ID>` (repetible); ver [VM](../vm/vm.md#crear-una-vm).