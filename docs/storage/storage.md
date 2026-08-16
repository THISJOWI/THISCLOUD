# Storage — pools de almacenamiento

Guía operativa para gestionar pools de almacenamiento con `thiscloud storage ...`.

## Índice

- [Listar pools](#listar-pools)
- [Crear un pool](#crear-un-pool)
- [Eliminar un pool](#eliminar-un-pool)
- [Tipos de pool](#tipos-de-pool)

## Listar pools

```sh
thiscloud storage list
```

Tabla con `NAME`, `TYPE`, `DEVICES` (dispositivos enlazados) y `REPL` (factor de replicación).

## Crear un pool

```sh
thiscloud storage create --name data --pool-type linstor --replication 2
```

Flags:

| Flag | Por defecto | Descripción |
|---|---|---|
| `--name` | *(obligatorio)* | Nombre del pool |
| `--pool-type` | `linstor` | `linstor`, `drbd` o `local` |
| `--replication` | `2` | Factor de replicación |
| `--devices` | *(vacío)* | Bloques de dispositivos separados por comas |

Ejemplos:

```sh
# Pool Linstor replicado 2 veces
thiscloud storage create --name data --pool-type linstor --replication 2

# Pool local sin replicación, sobre dispositivos explícitos
thiscloud storage create --name scratch --pool-type local --devices /dev/sdb,/dev/sdc

# Pool DRBD replicado 3 veces
thiscloud storage create --name ha-data --pool-type drbd --replication 3
```

## Eliminar un pool

```sh
thiscloud storage delete <POOL_NAME>
```

## Tipos de pool

| Tipo | Descripción |
|---|---|
| `linstor` | Almacenamiento gestionado por Linstor, replicación configurable |
| `drbd` | Replicación síncrona entre nodos mediante DRBD |
| `local` | Almacenamiento en disco local del nodo, sin replicación |