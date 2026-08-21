# Notificaciones de escritorio

## Comprobar si el problema es la aplicación o el escritorio

```bash
./xai-imagine-generator --test-notificacion
```

Imprime el diagnóstico del entorno y el resultado **real** de la llamada, sin
abrir la ventana. Interpretación:

| Salida | Significado | Qué hacer |
|---|---|---|
| `no hay DBUS_SESSION_BUS_ADDRESS` | El proceso no está en una sesión de escritorio | Lanzar la aplicación desde la sesión gráfica, no desde un `sudo`, `ssh` o servicio del sistema |
| `no responde ningún demonio de notificaciones` | Nadie escucha en D-Bus | Instalar/arrancar el demonio del escritorio (GNOME y KDE traen el suyo; en escritorios mínimos: `dunst` o `mako`) |
| `El escritorio aceptó el aviso` y aun así no se ve | El fallo está en el escritorio, no en la aplicación | Ver la sección siguiente |

Contraste útil: `notify-send "prueba"` desde la misma terminal. Si tampoco
aparece, el problema no es de esta aplicación.

## Si el escritorio acepta el aviso pero no se ve (GNOME)

- **No molestar** activado (menú de estado, arriba a la derecha).
- **Pantalla completa**: GNOME retiene los banners mientras hay una ventana a
  pantalla completa y los suelta al salir. Si se está jugando, es lo primero
  que hay que descartar.
- Ajustes → Notificaciones → comprobar que la aplicación no esté silenciada.

## Instalar el `.desktop` (recomendado en GNOME)

Sin él GNOME no sabe a qué aplicación pertenece el aviso y en algunas
versiones lo degrada a la bandeja en lugar de mostrar el banner.

```bash
install -Dm755 xai-imagine-generator ~/.local/bin/xai-imagine-generator
install -Dm644 packaging/batch-image-generator.desktop \
  ~/.local/share/applications/batch-image-generator.desktop
update-desktop-database ~/.local/share/applications 2>/dev/null || true
```

El nombre del fichero debe coincidir con la pista `desktop-entry` que envía la
aplicación (`batch-image-generator`), definida en `src/notify.rs`.
