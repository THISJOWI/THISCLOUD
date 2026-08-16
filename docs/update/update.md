# Update — actualización del sistema

Guía operativa para comprobar e instalar actualizaciones de THISCLOUD con `thiscloud update`.

## Índice

- [Comprobar la versión instalada](#comprobar-la-versión-instalada)
- [Comprobar si hay actualizaciones](#comprobar-si-hay-actualizaciones)
- [Instalar una actualización](#instalar-una-actualización)
- [Cómo funciona internamente](#cómo-funciona-internamente)
- [Rollback](#rollback)
- [Variables de entorno](#variables-de-entorno)

## Comprobar la versión instalada

```sh
thiscloud update --version
```

Imprime la versión semver instalada (leída de `/etc/thiscloud/version`) o `unknown (… missing)` si el archivo no existe.

## Comprobar si hay actualizaciones

```sh
thiscloud update --check
```

Consulta la última release de GitHub (`THISJOWI/THISCLOUD`) y compara contra la versión instalada:

- Si está al día: `THISCLOUD is up to date (<versión>)`.
- Si hay una nueva: muestra la versión, las notas de release y `Run sudo thiscloud update to install`.

## Instalar una actualización

```sh
sudo thiscloud update
```

Secuencia de instalación:

1. Descarga la última release de GitHub.
2. Descarga `manifest.json` y verifica contra la lista de assets de la release.
3. Descarga cada asset listado en el manifest y verifica su checksum **antes** de tocar el sistema.
4. Hace copia de seguridad del estado actual en `/etc/thiscloud/backup-v<versión>/`.
5. Instala: RPMs (`dnf localinstall`), binario `thiscloud-api`, web-ui (`/usr/share/thiscloud/web-ui`), unidades systemd.
6. Reinicia los servicios `thiscloudd`, `thiscloud-api` y `thiscloud-webui`.
7. Escribe la versión nueva en `/etc/thiscloud/version`.

> Requiere ser root (`sudo`). Un usuario sin privilegios solo recibe el aviso de que hay actualización disponible, sin instalar nada.

## Cómo funciona internamente

- La fuente es la API de GitHub: `GET /repos/<owner>/<repo>/releases/latest` (por defecto `THISJOWI/THISCLOUD`).
- La release debe contener `manifest.json`, que lista los assets con su `sha256`.
- Si falta `manifest.json` en la release, el instalador se niega a continuar.
- Si la verificación de checksum falla, aborta **sin hacer cambios**.
- La copia de seguridad incluye: config de `/etc/thiscloud`, binarios reemplazados (`thiscloud-api`, `thiscloudd`, `thiscloud`), el árbol de web-ui, unidades systemd y las versiones RPM.

## Rollback

Si la instalación falla o algún servicio no arranca tras reiniciar, el instalador:

1. Restaura binarios, web-ui y unidades systemd desde `/etc/thiscloud/backup-v<versión>/`.
2. Recarga systemd y reinicia los servicios.
3. Hace `dnf downgrade` de los paquetes RPM si se registraron versiones previas.

Tras un rollback, revisa el estado de los servicios:

```sh
systemctl status thiscloudd thiscloud-api thiscloud-webui
```

## Variables de entorno

| Variable | Por defecto | Descripción |
|---|---|---|
| `THISCLOUD_UPDATE_REPO` | `THISJOWI/THISCLOUD` | Repositorio GitHub (`owner/repo`) o URL `https://github.com/owner/repo` |
| `THISCLOUD_UPDATE_TOKEN` | *(vacío)* | Token de GitHub para repos privados o evitar límites de rate-limit |

Ejemplo con repositorio personalizado:

```sh
THISCLOUD_UPDATE_REPO=miorg/micloud sudo thiscloud update
```