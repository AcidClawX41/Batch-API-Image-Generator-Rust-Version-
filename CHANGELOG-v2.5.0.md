# v2.5.0 — Skins, Kie.AI y saneamiento

Batch Image Generator (Rust + Slint) · Eric Valls Gramunt

Primera release desde la 2.4.0. Nace de una auditoría completa del código
anterior, así que además de lo nuevo trae una tanda larga de correcciones.

---

## 1. Interfaz: tres skins, iguales en los tres sistemas

### El problema

La 2.4.0 usaba los controles de `std-widgets.slint`, que **toman su paleta
del sistema operativo**: estilo `fluent` en Windows y Linux, `cupertino` en
macOS, y variante clara u oscura según la preferencia del escritorio. El
resto de la ventana, en cambio, tenía un tema oscuro fijo (`#0d0d0d`).

En Ubuntu con el escritorio en modo claro salían dos paletas mezcladas:
campos y botones claros sobre fondo casi negro y —lo más grave— las
etiquetas de las casillas del randomizer dibujadas con el color de texto del
sistema, oscuro, sobre ese fondo negro. Eran **ilegibles**.

El global `Palette` de Slint sólo permite escribir `color-scheme` en tiempo
de ejecución; sus colores son de sólo lectura. Con widgets nativos no hay
forma de ofrecer más de dos variantes ni de garantizar el mismo aspecto en
las tres plataformas.

### La solución

- **`ui/theme.slint`** — paleta completa derivada de una única propiedad
  `skin`, conmutable en caliente.
- **`ui/widgets.slint`** — controles propios (`TButton`, `TCheckBox`,
  `TLineEdit`, `TTextEdit`, `TComboBox`, `TSpinBox`, `TProgress`,
  `TGroupBox`, `TScrollView`) dibujados sobre esa paleta.

`ui/main.slint` ya **no importa `std-widgets.slint`**. Nada consulta la
configuración del escritorio.

| Skin | Fondo | Texto | Acento |
|---|---|---|---|
| 🌙 Oscuro | `#12141a` | `#e8eaf0` | `#5b8cd4` |
| ☀ Claro | `#f4f5f7` | `#1a1c22` | `#2f6fd0` |
| 🌆 Cyberpunk | `#0a0a12` | `#e2f6ff` | `#ff2fb9` |

Se cambian desde el desplegable de la cabecera y la elección se guarda. El
color `text-dim`, usado en decenas de etiquetas, era `#888888` sobre
`#0d0d0d` —contraste ~4.0:1, por debajo del mínimo AA—; se ha subido en las
tres skins, y las casillas deshabilitadas siguen siendo legibles en lugar de
desaparecer.

### Las casillas, sobre una rejilla real

Las filas del randomizer usaban `HorizontalLayout` con
`horizontal-stretch: 1` en cada casilla. Pero **el stretch sólo reparte el
espacio *sobrante***: la anchura base de cada casilla la fija su propio
texto, así que «Orientación» y «Medias» no caían en la misma columna. Ahora
es un `GridLayout`, donde las columnas se alinean entre filas por
construcción.

---

## 2. Kie.AI: 12 modelos, generación y edición

| Modelo | Texto→imagen | Imagen→imagen | Máx. refs |
|---|---|---|---|
| GPT Image 2 | `gpt-image-2-text-to-image` | `gpt-image-2-image-to-image` | 5 |
| Qwen Image 3.0 | `qwen3/text-to-image` | `qwen3/image-to-image` | 3 |
| Nano Banana 2 | `nano-banana-2` | `nano-banana-2` | 5 |
| Nano Banana 2 Lite | `nano-banana-2-lite` | `nano-banana-2-lite` | 5 |
| Nano Banana Pro | `nano-banana-pro` | `nano-banana-pro` | 5 |
| Seedream 4.0 | `bytedance/seedream-v4-text-to-image` | — | — |
| Seedream 4.0 Edit | — | `bytedance/seedream-v4-edit` | 5 |
| Seedream 5.0 Lite | `seedream/5-lite-text-to-image` | — | — |
| Seedream 5.0 Pro | `seedream/5-pro-text-to-image` | `seedream/5-pro-image-to-image` | 5 |
| Grok Imagine | `grok-imagine/text-to-image` | `grok-imagine/image-to-image` | 2 |
| Flux.2 Pro | `flux-2/pro-text-to-image` | `flux-2/pro-image-to-image` | 5 |

Cada identificador está verificado uno a uno contra `docs.kie.ai`. Ninguno
está deducido por analogía.

### El flujo de Kie.AI es distinto

1. **Las referencias deben ser URL públicas.** Kie.AI no acepta data URIs,
   así que la aplicación sube antes cada imagen con la API de subida en
   base64 y usa la `downloadUrl` que devuelve.
2. `POST /api/v1/jobs/createTask` devuelve un `taskId`.
3. Se sondea `GET /api/v1/jobs/recordInfo?taskId=…` hasta `state ==
   "success"`.
4. Las URL del resultado llegan dentro de `data.resultJson`, que es una
   **cadena JSON anidada** con un array `resultUrls`.

Kie.AI puede además responder **HTTP 200 con un código de error en el
cuerpo**; ese caso se detecta en vez de tratarse como éxito.

Hay un tercer campo de API key en la interfaz. Como los otros dos, **no se
persiste**.

---

## 3. Tabla unificada de modelos

Kie.AI es lo que hizo inviable seguir como estábamos. Sus modelos usan
**cuatro nombres distintos** para el campo de imagen de referencia:

```
gpt-image-2-image-to-image   →  "input_urls"
seedream/5-pro-image-to-image →  "image_urls"
nano-banana-pro              →  "image_input"
qwen3/image-to-image         →  "image_urls"   (pero máximo 3, no 5)
```

Y peor: **`image_size` significa cosas distintas según el modelo**. En
Seedream 4.0 es un preajuste (`"square_hd"`); en Qwen 3 es una relación de
aspecto (`"1:1"`). Mismo nombre, semántica opuesta. Ninguna heurística sobre
el identificador puede acertar con eso.

`src/models.rs` es ahora la única fuente. Cada modelo declara etiqueta,
proveedor, identificador en cada modo, nombre del campo de imagen, máximo de
referencias y estilo de tamaño. De la tabla salen el enrutado, el catálogo
**y las etiquetas del desplegable**, que Rust rellena al arrancar.

Antes esa lista estaba escrita a mano en `ui/main.slint` y tenía que
coincidir en contenido **y en orden** con `MODEL_CATALOG`, sin que nada lo
verificara: si se desfasaban, el desplegable mostraba un nombre y se enviaba
otro modelo. Ese riesgo ha desaparecido.

**Total: 51 modelos** en 5 proveedores.

---

## 4. Correcciones

### Críticas

**Burst sobrescribía imágenes.** El nombre de archivo tenía resolución de
**un segundo** (`%Y%m%d_%H%M%S`) y se escribía con `fs::write`, que trunca.
Dos generaciones que terminaran en el mismo segundo —normal en Burst—
producían **un solo archivo**: la segunda pisaba a la primera en silencio
mientras el log anunciaba las dos como guardadas. Ahora el nombre incluye
milisegundos y un contador, y se escribe con `create_new`.

**Cuatro panics por cortar cadenas por bytes.** `&prompt[..200]`,
`&base_prompt[..100]`, `&body[..200]` y `base[..end]` indexaban por byte. En
Rust eso **entra en pánico** si el índice cae dentro de un carácter
multibyte: bastaba una `ñ`, un acento o un emoji en la posición equivocada.
Uno de ellos estaba en el camino de cada generación.

**`burst_mode` no se reiniciaba.** Iniciar Burst y después pulsar «Iniciar
Loop» sin pasar por «Detener» dejaba la bandera activa: el loop con
intervalo generaba sin pausa. Con APIs de pago, eso es dinero.

### Correctitud

**La matriz de compatibilidad ya se aplica.** `wavespeed_supports_i2i()`
ignoraba su argumento y devolvía siempre `true`, así que el mensaje de error
para modelos texto→imagen puros era **código inalcanzable**: con Flux 2 Max
y una imagen cargada se construía un endpoint inventado y WaveSpeed
devolvía un 400 críptico. El caso simétrico —endpoint de edición sin
imagen— también se cubre ahora.

**xAI respeta el modelo elegido.** `xai_edit_model()` descartaba la
selección y enviaba siempre `"grok-imagine-image-quality"`, identificador
que ni figuraba en el catálogo. También se ha quitado el `aspect_ratio:
"16:9"` hardcodeado, que ignoraba el selector de resolución.

**El sondeo ya no se traga los errores.** Reintentaba hasta 180 veces ante
cualquier error HTTP: una key inválida costaba **tres minutos** y acababa en
un «timeout» engañoso. Ahora un 4xx —salvo 429— aborta al instante.

**«Detener» cancela de verdad.** Antes sólo ponía `running = false`: la
petición seguía viva hasta su timeout de 180 s, y minutos después aparecía
otra imagen en la carpeta, facturada.

**La extensión ya no miente.** Todo se guardaba como `.png` aunque el
proveedor devolviera JPEG o WebP. Ahora se deduce de los magic bytes.

**Aviso al exceder el máximo de referencias**, en vez de provocar un 400.

### Rendimiento

- **Regex recompiladas en cada generación**: los tres `Regex::new()` de
  `modify_prompt` se ejecutaban por imagen y por pulsación del interruptor;
  el patrón de las uñas tiene medio centenar de alternativas anidadas. Ahora
  son `LazyLock`.
- **Log con crecimiento O(n²)**: cada línea clonaba la cadena completa.
  Ahora está acotado a 200 KB y se recorta por la cabecera.
- **Un `reqwest::Client` nuevo por petición** en seis sitios, lo que
  descartaba el keep-alive y forzaba un handshake TLS completo por
  generación. Ahora hay un cliente compartido.

---

## 5. Persistencia de preferencias

`src/config.rs` guarda skin, carpeta, modelo, resolución, intervalo, modo,
tema y los 21 interruptores del randomizer.

| Sistema | Ruta |
|---|---|
| Linux | `~/.config/batch-image-generator/config.json` |
| macOS | `~/Library/Application Support/batch-image-generator/config.json` |
| Windows | `%APPDATA%\batch-image-generator\config.json` |

**Las API keys NO se guardan.** El sitio correcto es el llavero del sistema
(crate `keyring`), no un fichero en claro. Pendiente de decidir.

---

## 6. Repositorio y proceso

- **Fuera del control de versiones** el binario compilado de 7,6 MB (era el
  63 % del peso del repo), `xai-imagine-generator.d`, `.DS_Store` y
  `.idea/`. *Sale del árbol, no del historial*: purgarlo de ahí exige
  reescribir commits.
- **CI nuevo** en cada push y PR, en las tres plataformas, con `cargo test`
  y `cargo clippy -D warnings`. El único workflow anterior se disparaba con
  `release: published`: **nada comprobaba que `main` compilara**.
- **46 tests** donde antes no había ninguno.
- El `Cargo.lock` estaba desincronizado con `Cargo.toml` (2.3.0 frente a
  2.4.0).
- El README anunciaba 42 modelos; el catálogo tenía 40.

---

## 7. Correcciones posteriores a las primeras pruebas de uso

**El cuadro del prompt era casi ineditable.** Mis controles propios llevaban
un `TouchArea` superpuesto que llamaba a `focus()` al hacer clic. El problema
es que ese TouchArea **se traga el evento**: el `TextInput` de debajo nunca lo
recibía, así que no se podía colocar el cursor con el ratón ni seleccionar
arrastrando. El `TextEdit` de Slint no lleva ninguno, precisamente por esto.
Eliminado en `TLineEdit` y `TTextEdit`.

Además faltaba el seguimiento del cursor: al escribir más allá de la zona
visible, la vista no acompañaba al cursor. Con un prompt de 1000+ caracteres
el campo parecía haberse colgado. Añadido `cursor-position-changed`.

Verificado con el ratón y el teclado sobre la aplicación en marcha: clic para
colocar el cursor, arrastre para seleccionar, y sustitución de la selección
al escribir.

**Kie.AI parecía colgarse.** No se colgaba, pero el log se quedaba parado en
«Enviando petición…» mientras por debajo subía las imágenes, creaba la tarea
y sondeaba el resultado — varios minutos sin decir nada. Ahora informa de
cada paso: subida de cada referencia con su tamaño, creación de la tarea,
identificador asignado, y un aviso cada ~15 s con el estado real (`en
espera`, `en cola`, `generando`) y el tiempo transcurrido. El sondeo usa
espera escalonada (1 s al principio, luego 2 s, luego 5 s) para responder
rápido en los modelos veloces sin castigar la API en los lentos. Si agota la
espera, el mensaje dice cuántos segundos pasaron y qué mirar.

**Kie.AI rechazaba todas las peticiones.** El proveedor devolvía HTTP 500 con
`aspect_ratio is not within the range of allowed options` y no se podía
enviar nada. La causa: yo mandaba `aspect_ratio: "auto"` a todos los modelos,
pero **sólo la familia Nano Banana lo admite** — Grok y Flux.2 lo rechazan.

Corregido con el mismo criterio que el resto de la tabla: cada modelo declara
su relación de aspecto (`aspect`), verificada en la documentación, y si no
está verificada **no se envía el campo** y decide el modelo. Además,
«Resolución: Auto» pasa a significar de verdad *no enviar tamaño alguno*;
antes mandaba `1K` por su cuenta. Cuatro tests de regresión cubren el caso,
incluido uno que recorre todo el catálogo comprobando que ningún modelo
recibe un `aspect_ratio` que no haya declarado.

**Y una segunda corrección encima de la anterior.** Al arreglar el
`aspect_ratio` dejé de enviar también `quality`, `output_format` y
`resolution` cuando la resolución estaba en «Auto» — me pasé de frenada. Pero
**todos los ejemplos de la documentación incluyen esos campos**, y Seedream
5.0 Pro empezó a responder «This field is required». El problema original era
el *valor* de un campo, no la *presencia* de los campos.

Ahora cada familia envía exactamente el juego que aparece en su ejemplo
oficial, y «Auto» significa usar el valor por defecto de ese ejemplo (`1K`),
no omitir nada. Grok pasa a enviar `aspect_ratio: "3:2"`, el valor de su
ejemplo, en lugar de nada.

Verificando esto apareció el mejor argumento a favor de la tabla de modelos:
**Nano Banana 2 Lite no se comporta como sus hermanos**. Nano Banana 2 y Pro
usan `image_input` y llevan `resolution` y `output_format`; el Lite usa
`image_urls` y sólo `aspect_ratio`. Mismo fabricante, misma familia, misma
generación — y tres diferencias. Hay un test que lo deja por escrito.

**El error 524 ya se distingue.** Kie.AI lo emite cuando su propio worker
agota el tiempo, no cuando la petición es incorrecta. Antes se reportaba como
un fallo genérico y llevaba a buscar el problema donde no estaba; ahora el
mensaje dice explícitamente que la tarea se aceptó y que el retraso es del
proveedor, y sugiere reintentar, bajar resolución o cambiar de modelo.

**El desplegable dice ahora qué hace cada modelo.** Los sufijos («[I2I]»,
«[★ 5 imgs]») se escribían a mano y unos modelos los llevaban y otros no. Al
fusionar el Grok de edición con el de texto en una sola entrada, el
desplegable dejó de mostrar que seguía haciendo imagen→imagen y parecía que
el modelo hubiera desaparecido. Ahora la etiqueta se genera desde la tabla:
cada modelo muestra `T2I`, `I2I` o `T2I+I2I` y su límite de referencias, y no
puede quedarse desfasada.

**Grok en Kie.AI ya genera desde texto.** Sólo estaba la edición, lo que daba
a entender que por Kie.AI no se podía usar Grok para texto→imagen. Añadidos
también Flux.2 Pro, Qwen Image 3.0, Seedream 5.0 Pro y Lite, y Nano Banana 2
Lite. **Total: 51 modelos.**

---

## 8. Super Randomizer

Un segundo modo del randomizer, **sin sustituir al de siempre**. Con el botón
en ON, en cada generación se sortea **cuántas** categorías entran —entre 1 y
las 21— y **cuáles**. Un Burst largo deja de repetir la misma combinación una
y otra vez.

Decisiones de diseño:

- **El número de categorías es uniforme entre 1 y 21.** Así salen tanto
  tiradas mínimas (una sola categoría, cambio sutil) como máximas (las 21,
  cambio radical). Nunca sale vacío: una generación sin ninguna categoría
  sería idéntica al prompt base y el modo no tendría efecto.
- **El sorteo se ve.** Las casillas se actualizan en cada generación para
  mostrar qué ha tocado, y el log lo escribe con nombres:
  `🎰 Super Randomizer: 11 de 21 categorías — Expresión, Ropa, Pose, …`.
- **Tu selección manual no se pierde.** Al activarlo se guarda una copia; al
  apagarlo vuelve intacta. Y lo que se escribe en `config.json` es siempre esa
  copia, nunca la última tirada aleatoria — de lo contrario, cerrar la
  aplicación con el modo activo te habría dejado la combinación del sorteo
  como si fuera tu elección.
- **Enciende el randomizer si estaba apagado**, porque sin él no haría nada, y
  lo dice en el log en vez de hacerlo en silencio.
- Mientras está activo, las casillas se ven atenuadas: las gobierna el sorteo,
  no tú.

Sólo afecta al Modo A. El Modo B ya genera el prompt entero desde los pools y
no usa estas casillas.

Verificado sobre la aplicación en marcha: activación, sorteo visible en las
casillas, prompt resultante con las categorías sorteadas, y restauración
exacta de la selección manual al apagarlo.

---

## 9. Banco de prompts

Cinco ranuras donde guardar prompts, con **Guardar**, **Cargar** y **Vaciar**,
y un interruptor **Prompt aleatorio**: con él en ON, cada generación toma como
base uno de los prompts guardados, elegido al azar. El randomizer y el Super
Randomizer se aplican encima, así que se combinan sin estorbarse.

- **No sobrescribe el cuadro de texto.** Lo que hay escrito ahí es tuyo y se
  conserva; el prompt sorteado se usa sólo para esa generación y se ve montado
  en el preview. El log dice qué ranura tocó.
- **Con el banco vacío avisa y usa el prompt escrito**, en lugar de generar
  con la cadena vacía o fallar.
- Las cinco ranuras se guardan en `config.json`. Una configuración anterior sin
  el campo se normaliza a cinco ranuras vacías al cargar, de modo que el índice
  del desplegable nunca se sale de rango.

---

## 10. Notificaciones de escritorio

Un interruptor general más un aviso por tipo de suceso: generación correcta,
timeout del servidor, rechazo por políticas de contenido y otros errores.
Incluye un botón **Probar aviso** para comprobar que tu escritorio los muestra
sin esperar a una generación.

Sobre `notify-rust`, que cubre los tres sistemas con una sola API:

| Sistema | Mecanismo |
|---|---|
| Linux | Especificación XDG por **D-Bus** |
| Windows 10/11 | Toasts WinRT |
| macOS | `UNUserNotificationCenter` |

En Linux funciona igual en **Wayland y en XWayland**, porque el transporte es
D-Bus y no el servidor gráfico: se habla con el demonio de notificaciones del
escritorio (GNOME Shell, KDE, mako, dunst…), no con X11 ni con el compositor.

Dos reglas en la implementación:

1. **Nunca bloquear la interfaz.** `show()` hace E/S y puede tardar o fallar;
   cada aviso sale en su propio hilo.
2. **Nunca tumbar la aplicación.** Sin demonio de notificaciones, sin permiso
   en macOS o sin sesión de escritorio, el error se anota y se sigue
   generando. Un aviso es un extra, jamás un requisito.

El cuerpo común (`summary`, `body`, `appname`) existe en las tres
plataformas. En Linux se añaden además, tras compilación condicional, un icono
y la pista `desktop-entry` (ver §10 bis).

**Clasificar el fallo hay que hacerlo por el texto**: los proveedores no
devuelven un código uniforme ni para «rechazado por contenido» ni para «se me
acabó el tiempo». Los patrones salen de mensajes reales observados en
ejecución, y la política de contenido tiene prioridad sobre el timeout cuando
un mensaje menciona las dos cosas.

**«Generación correcta» viene desactivado a propósito**: en un Burst largo
serían cientos de avisos. La interfaz lo advierte si lo activas.

Verificado de punta a punta en Linux: demonio D-Bus real, aviso mostrado por
el escritorio y capturado.

---

## 10 bis. Por qué no se veía ninguna notificación en Ubuntu

Reportado tras la primera prueba real: las notificaciones estaban en ON, el
log decía «Aviso de prueba enviado» y en el escritorio no aparecía nada.

### El fallo era mío, y era de diseño

Dos errores encadenados:

1. **`notify()` mandaba los errores de `show()` a `eprintln!`.** Quien arranca
   la aplicación desde el lanzador del escritorio no ve stderr. Es decir: hice
   el error invisible justo en el único sitio donde hacía falta verlo.
2. **El log mentía.** «Aviso de prueba enviado» se escribía en cuanto se creaba
   el hilo, no cuando el escritorio aceptaba la notificación. Decía que sí
   cuando la respuesta podía perfectamente haber sido que no.

### Qué se ha hecho

**El resultado real llega al log.** `notify()` y `test()` reciben ahora un
canal de log (`LogFn`) y anotan lo que de verdad devuelve `show()`:

```
[AVISO] 🔔 Enviando aviso de prueba…
[AVISO] 🔔 El escritorio aceptó el aviso. Si aun así no lo ves, revisa
        «No molestar» y los permisos de notificación del sistema.
[AVISO] ℹ Diagnóstico — D-Bus de sesión: presente · demonio de
        notificaciones: dunst 1.9.2 (knopwob) · escritorio: GNOME ·
        sesión: wayland
```

**`diagnostico()`** reúne los datos que separan las dos causas posibles: si
hay `DBUS_SESSION_BUS_ADDRESS`, qué demonio responde a
`GetServerInformation()`, y qué escritorio y tipo de sesión hay. En macOS y
Windows emite en su lugar la pista propia de cada plataforma.

**Pista `desktop-entry` e icono en Linux.** Sin ella GNOME Shell no sabe a qué
aplicación pertenece el aviso, y en algunas versiones lo degrada a la bandeja
en vez de mostrar el banner. Es la causa más frecuente de «se envía y no se
ve». Se acompaña de `packaging/batch-image-generator.desktop`, cuyo nombre de
fichero debe coincidir con la pista. También se fija un `timeout` de 8 s para
que no se desvanezca mientras se mira otra ventana.

**Modo diagnóstico sin interfaz:**

```bash
./xai-imagine-generator --test-notificacion
```

Imprime el diagnóstico y el resultado real por la terminal, sin abrir ventana.
Existe porque «no veo notificaciones» tiene dos causas muy distintas —la
aplicación no consigue enviarla, o el escritorio la recibe y no la muestra— y
desde la ventana no se distinguen. Contraste útil: `notify-send "prueba"`
desde la misma terminal; si tampoco aparece, el problema no es de esta
aplicación.

`packaging/README-notificaciones.md` recoge la tabla de interpretación y los
sospechosos habituales en GNOME: **No molestar** activado y **pantalla
completa** (GNOME retiene los banners mientras hay una ventana a pantalla
completa y los suelta al salir — si se está jugando, es lo primero que hay que
descartar).

### Verificación

Los dos caminos, comprobados en el contenedor:

| Escenario | Salida |
|---|---|
| D-Bus + dunst reales | `El escritorio aceptó el aviso` + demonio identificado |
| Sin `DBUS_SESSION_BUS_ADDRESS` | `El escritorio rechazó el aviso: I/O error…` + diagnóstico |

Y desde la ventana: botón «Probar aviso» pulsado en una sesión headless con
dunst, con las cuatro líneas correctas apareciendo en el log de la aplicación.

---

## 10 ter. Modelos de WaveSpeed que no aceptaban la imagen

Tres fallos distintos vistos en uso real, con el mismo síntoma aparente
(«no genera»).

### A. `Invalid request body: field "image" is required`

Afectaba a **WAN 2.2** y **Qwen Image Edit** con una imagen cargada.

**Causa raíz — mía.** Al montar la tabla de modelos di por hecho que todos los
endpoints de edición de WaveSpeed reciben `images` (array). No es así: cada
modelo tiene su propio contrato, y esos dos esperan `image`, una sola cadena.
Es exactamente el mismo error que cometí con Nano Banana 2 Lite: configurar
por analogía en vez de por documentación.

Verificado uno a uno contra la documentación oficial:

| Modelo | Campo | Máximo | Estado |
|---|---|---|---|
| WAN 2.2 (`wan-2.2/image-to-image`) | `image` | 1 | **corregido** (era `images`, 2) |
| Qwen Image Edit (`qwen-image/edit`) | `image` | 1 | **corregido** (era `images`, 5) |
| WAN 2.7 Edit (`wan-2.7/image-edit`) | `images` | 3 | máximo subido de 2 a 3 |
| Grok Imagine Edit | `image` | 1 | ya era correcto |
| UNO, Flux Kontext Multi | `images` | 5 | ya era correcto |
| Nano Banana 2 / Pro, Seedream, GPT Image 2, Flux 2 Klein | `images` | 5 | correcto (el modelo admite más; la ventana expone 5 huecos) |

### B. La descarga tiraba una generación ya pagada

**WAN 2.7 Edit** falló con `Error descargando imagen: error sending request
for url (…cloudfront.net…)`. La API había respondido bien: lo que se cortó fue
la descarga desde la CDN.

Es una asimetría que no estaba contemplada: cuando la descarga falla **el
trabajo ya está hecho y ya está cobrado**. Repetir una descarga es gratis;
repetir una generación no.

Ahora hay hasta 4 intentos con espera de 1 s, 2 s y 4 s, y se distingue lo
transitorio de lo definitivo: se reintenta ante timeout, corte de conexión,
corte a mitad del cuerpo, 5xx y 429; **no** se reintenta ante 404 o 403, que
en una URL firmada significan caducada y sólo harían perder tiempo. El
reintento se ve en el log, y si aun así se agota, la URL de la imagen queda
escrita para poder rescatarla a mano antes de que expire.

### C. Las pistas de las casillas de imagen mentían

Los textos «★ Solo Flux Kontext Multi / UNO» y «Img 1+2: xAI Grok · Img 1:
OpenAI/Flux Kontext/WAN» estaban escritos a mano y se quedaron viejos en
cuanto cambió la tabla. Peor: contradecían a lo que la aplicación enviaba.

Ahora salen del modelo seleccionado. Rust publica los máximos en paralelo a
los nombres, y la ventana avisa **antes** de generar:

- un hueco que el modelo no admite lo dice en el propio hueco;
- una imagen cargada que se va a descartar sale en naranja con «el modelo
  elegido no la usará»;
- el resumen dice cuántas de cuántas están en uso.

### Prevención

Cinco tests nuevos, todos rápidos y sin llamadas a la API:

| Test | Qué impide |
|---|---|
| `el_cuerpo_de_wavespeed_usa_el_campo_documentado_de_cada_modelo` | Comprueba el JSON exacto que sale por el cable para los cinco modelos afectados: nombre del campo, tipo (cadena vs. array) y que no se envíe el otro |
| `los_campos_de_imagen_de_wavespeed_estan_verificados` | Fija los valores comprobados contra la documentación |
| `el_campo_de_imagen_unica_solo_admite_una_referencia` | Un campo de una sola imagen no puede anunciar varias |
| `una_descarga_cortada_se_reintenta_y_se_recupera` | Servidor local que corta dos veces y responde a la tercera |
| `un_404_no_se_reintenta` | Que un fallo definitivo no gaste cuatro intentos |
| `los_maximos_de_referencias_van_en_paralelo_a_los_nombres` | Que la ventana no enseñe el máximo de otro modelo |

Para poder probar el cuerpo de la petición sin gastar créditos, la
construcción del JSON se ha separado de la llamada de red
(`wavespeed_body()`). El bug original sólo era visible enviando la petición de
verdad; ahora se detecta en medio milisegundo.

---

## 11. Pendiente para la v2.6

1. **Guardar el prompt y la semilla junto a cada imagen.** Con `seed: -1`
   fijo, una imagen buena salida de un Burst es irreproducible. De lo que
   queda, es lo de mayor valor práctico.
2. **API keys en el llavero del sistema** — ya son tres claves que
   reintroducir en cada arranque.
3. **Pools externos en JSON**, para editar prompts sin recompilar.
4. Más modelos de Kie.AI: Flux.2, Flux Kontext, Wan 2.7, Qwen 2.0,
   Seedream 4.5, Ideogram, Topaz *upscale*, Recraft *remove background*,
   4o Image, gpt-image-1.5. Cada uno es una entrada en la tabla, pero
   **después** de verificar su identificador y su campo de imagen en la
   documentación.
5. El binario de 7,6 MB sigue en el historial de git.
