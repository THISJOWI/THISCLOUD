# Architecture — componentes y flujo de datos

Descripción de la arquitectura de THISCLOUD: los cinco componentes, cómo se relacionan y el patrón que siguen los módulos de recursos del daemon.

## Índice

- [Visión general](#visión-general)
- [Componentes](#componentes)
- [Flujo de datos](#flujo-de-datos)
- [Contrato de API](#contrato-de-api)
- [Patrón de módulos del daemon](#patrón-de-módulos-del-daemon)
- [Backends de recursos](#backends-de-recursos)

## Visión general

```
┌───────────────────────────────────────────────┐
│                Web UI (Next.js)                │
│          Portal / Admin / Console              │
│                       │                        │
│                       ▼                        │
│          Go API (orquestador, :8081)           │
│      tfstate store + CRUD /api/v1/resources    │
│                       │                        │
│                       ▼                        │
│        thiscloudd (daemon Rust, :8080)         │
│   /api/v1 — dueño real de los recursos         │
│  ┌────────┬──────────┬──────────┬───────────┐  │
│  │ Compute│ Network  │ Storage  │Marketplace│  │
│  │  VMs   │  OVN     │ Linstor/ │   apps    │  │
│  │cloud-  │          │ DRBD/local│          │  │
│  │hyperv. │          │          │           │  │
│  └────────┴──────────┴──────────┴───────────┘  │
└───────────────────────────────────────────────┘
```

El **daemon es la única fuente de verdad física** de los recursos. El CLI y el Go API son clientes del daemon; el Web UI nunca habla con el daemon directamente.

## Componentes

| Componente | Ruta | Stack | Rol |
|---|---|---|---|
| `thiscloud-cli` | `platform/thiscloud-cli/` | Rust (clap) | CLI `thiscloud`; cliente HTTP del daemon en `:8080` |
| `thiscloudd` | `platform/thiscloudd/` | Rust (axum) | Daemon; API HTTP `:8080` bajo `/api/v1`; gestiona compute, network, storage, marketplace, images, nodes, quota, s3, metrics |
| `go-api` | `platform/go-api/` | Go | Orquestador en `:8081`; CRUD tipo Terraform provider sobre `/api/v1/resources`; aplica cambios llamando al daemon |
| `web-ui` | `platform/web-ui/` | Next.js 14 | Dashboard (Portal `/`, Admin `/admin`, Console `/console`); cliente del Go API |
| `iso` | `platform/iso/` | bash/kickstart | Pipeline de ISO AlmaLinux 9 que instala el sistema completo |

## Flujo de datos

1. **Web UI → Go API**: los componentes del servidor (`src/lib/api.ts`) llaman al Go API en `:8081` (`API_URL`). Los componentes de cliente pasan por `/api/proxy/[...path]`. El UI no accede al daemon.
2. **Go API → daemon**: `internal/backend` es un cliente HTTP del daemon. El Go API mantiene un estado tfstate-style (`internal/state`, persistido en `THISCLOUD_STATE_FILE`) y aplica los cambios llamando a `/api/v1` del daemon ("apply"). Nunca gestiona recursos directamente.
3. **CLI → daemon**: `thiscloud` habla con `:8080` usando `THISCLOUD_API_URL`.
4. **Daemon**: cada módulo de recurso atiende rutas bajo `/api/v1` y delega en un backend (mock en desarrollo, real dentro del ISO).

## Contrato de API

- **Single source of truth**: `docs/api/openapi.yaml`. El daemon lo sirve en `GET /api/v1/openapi.yaml`.
- Todas las rutas de recursos viven bajo `/api/v1` (versión única).
- El Go API expone su propio contrato tipo Terraform en `/api/v1/resources`, distinto del del daemon.
- El Web UI genera sus tipos TypeScript desde el OpenAPI (`npm run gen:api` → `src/lib/api-types.ts`).

### Rutas públicas (sin auth) del daemon

| Ruta | Método | Descripción |
|---|---|---|
| `/api/v1/healthz` | GET | Sondeo de salud |
| `/api/v1/openapi.yaml` | GET | Contrato OpenAPI |

### Autenticación

Cuando la autenticación está habilitada (`[auth]` en config), las rutas de recursos bajo `/api/v1` quedan protegidas por JWT:

- `POST /api/v1/auth/login` (username + password) devuelve token, rol, tenant y expiración.
- El token se exige en el resto de rutas de `/api/v1`.

## Patrón de módulos del daemon

Cada tipo de recurso sigue una capa única (ver `platform/thiscloudd/src/core/module.rs` y cualquier módulo, p. ej. `network/`):

```
model → backend trait (Mock*Backend / real) → store (Memory*Store / Etcd*Store)
      → *Module → axum http.rs → CLI command espejo
```

| Capa | Responsabilidad |
|---|---|
| `model.rs` | Estructuras de datos del recurso |
| `backend.rs` | Trait de backend + `Mock*Backend` (memoria, para tests/macOS) y backends reales (shell out; solo dentro del ISO) |
| `store.rs` | Persistencia: `Memory*Store` o `Etcd*Store` |
| `module.rs` | Orquestación de la lógica de negocio |
| `http.rs` | Rutas axum del recurso |
| CLI | Comando espejo en `thiscloud-cli/src/commands/` |

Recursos implementados con este patrón: compute (VMs), network, storage, marketplace, image, node, quota, s3 y metrics.

## Backends de recursos

| Recurso | Backend real | Backend mock (dev/macOS) |
|---|---|---|
| Compute | cloud-hypervisor | `MockComputeBackend` |
| Network | OVN/OVS | `MockNetworkBackend` |
| Storage | Linstor / DRBD | `MockStorageBackend` |
| Marketplace | Docker | `MockMarketplaceBackend` |

Los backends mock permiten ejecutar y probar todo el sistema en macOS/desarrollo sin virtualización real. Los backends reales solo se ejecutan dentro del ISO.

## Variables de entorno clave

| Variable | Por defecto | Usado por |
|---|---|---|
| `THISCLOUD_API_URL` | `http://127.0.0.1:8080` | CLI → daemon; Go API → daemon |
| `THISCLOUD_STATE_FILE` | `./thiscloud.tfstate` | Go API (estado tfstate) |
| `THISCLOUD_API_BIND` | `127.0.0.1:8081` | Go API (bind) |
| `NEXT_PUBLIC_API_URL` | `http://127.0.0.1:8081` | Web UI (cliente) |
| `API_URL` | `http://127.0.0.1:8081` | Web UI (servidor) |