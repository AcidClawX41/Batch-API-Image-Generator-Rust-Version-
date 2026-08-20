//! models.rs — Tabla única de modelos.
//!
//! Batch Image Generator — Eric Valls Gramunt
//!
//! POR QUÉ EXISTE
//! --------------
//! Hasta ahora la información de cada modelo estaba repartida en tres
//! sitios que había que mantener sincronizados a mano:
//!
//!   1. `MODEL_CATALOG` en `main.rs` — proveedor e identificador.
//!   2. `model-list` en `ui/main.slint` — etiquetas, **en el mismo orden**.
//!   3. Una cascada de `contains()` / `ends_with()` en `api.rs` que deducía
//!      el endpoint de imagen a imagen y el nombre del campo de la imagen a
//!      partir del identificador.
//!
//! Ninguno de los tres lo verificaba el compilador. El comentario
//! `// ← was missing` que había junto a `ends_with("/edit-fast")` es la
//! prueba de que la cascada ya falló una vez.
//!
//! Al integrar Kie.AI eso dejó de ser sostenible: sus modelos usan **tres
//! nombres distintos** para el mismo campo de imagen de referencia
//! (`input_urls`, `image_urls`, `image_input`) sin ninguna regla deducible
//! del identificador. Adivinar por subcadenas era inviable.
//!
//! Ahora cada modelo se declara una vez, aquí, y de esta tabla salen el
//! catálogo, las etiquetas de la interfaz y el enrutado de la petición.

use crate::api::ImageProvider;

/// Nombre y forma del campo que transporta las imágenes de referencia.
///
/// No se deduce del identificador: se declara explícitamente por modelo,
/// porque no hay ninguna regla que lo relacione con el nombre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefField {
    /// El modelo no acepta imágenes de referencia.
    None,
    /// WaveSpeed: `"image": "<data-uri>"` (una sola).
    WsImage,
    /// WaveSpeed: `"images": ["<data-uri>", …]`.
    WsImages,
    /// OpenAI `/v1/images/edits`: multipart, parte `image`.
    OpenAiMultipart,
    /// xAI `/v1/images/edits`: `"images": [{ "type": "image_url", "url": … }]`.
    XaiImages,
    /// Kie.AI: `"input_urls": [ "<url>", … ]`.
    KieInputUrls,
    /// Kie.AI: `"image_urls": [ "<url>", … ]`.
    KieImageUrls,
    /// Kie.AI: `"image_input": [ "<url>", … ]`.
    KieImageInput,
}

/// Cómo expresa cada API el tamaño de salida.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SizeStyle {
    /// El modelo no acepta parámetros de tamaño.
    None,
    /// WaveSpeed: `"size": "1024*1024"`.
    WsSize,
    /// Kie.AI: sólo `"aspect_ratio"`.
    KieAspect,
    /// Kie.AI: `"aspect_ratio"` + `"resolution"` + `"output_format"`.
    KieAspectResolution,
    /// Kie.AI: `"image_size"` + `"image_resolution"`.
    KieImageSize,
    /// Kie.AI (Qwen 3): `"resolution"` (1K/2K) + `"image_size"` con una
    /// relación de aspecto («1:1») + `"output_format"`. Ojo: aquí
    /// `image_size` significa algo distinto que en Seedream 4.0, donde es
    /// un preajuste («square_hd»). Mismo nombre, otro significado.
    KieQwen,
    /// Kie.AI (Seedream 5.x): `"aspect_ratio"` + `"quality"` +
    /// `"output_format"`.
    KieAspectQuality,
}

/// Todo lo que hace falta saber de un modelo para construir su petición.
pub struct ModelSpec {
    /// Etiqueta mostrada en el desplegable. La lista de la interfaz se
    /// genera desde aquí, así que no puede desincronizarse.
    pub label: &'static str,
    pub provider: ImageProvider,
    /// Identificador enviado en modo texto→imagen.
    /// `None` = el modelo **exige** imagen de referencia (endpoints de
    /// edición puros).
    pub t2i_id: Option<&'static str>,
    /// Identificador enviado en modo imagen→imagen.
    /// `None` = el modelo es texto→imagen puro.
    pub i2i_id: Option<&'static str>,
    pub ref_field: RefField,
    /// Máximo de imágenes de referencia que acepta. Las sobrantes se
    /// descartan avisando en el log, en lugar de provocar un 400.
    pub max_refs: usize,
    pub size_style: SizeStyle,
    /// Relación de aspecto que acepta este modelo, tal cual viaja en la
    /// petición. Cadena vacía = no enviar el campo y dejar que el modelo
    /// aplique su propio valor por defecto.
    ///
    /// Aquí estaba el fallo que hacía que Kie.AI rechazara todo con
    /// «aspect_ratio is not within the range of allowed options»: se enviaba
    /// `"auto"` a todos los modelos, pero **sólo la familia Nano Banana lo
    /// admite**. Grok y Flux.2 devolvían un 500. Como con el nombre del campo
    /// de imagen, no hay un valor universal: se declara por modelo, y si no
    /// está verificado en la documentación, no se envía nada.
    pub aspect: &'static str,
}

impl ModelSpec {
    pub fn supports_i2i(&self) -> bool {
        self.i2i_id.is_some()
    }

    pub fn supports_t2i(&self) -> bool {
        self.t2i_id.is_some()
    }

    /// Etiqueta para el desplegable, con el modo y el límite de referencias
    /// añadidos automáticamente.
    ///
    /// Antes los sufijos («[I2I]», «[★ 5 imgs]») se escribían a mano en cada
    /// entrada, así que unos modelos los llevaban y otros no. Al fusionar el
    /// Grok de edición con el de texto en una sola entrada, el desplegable
    /// dejó de mostrar que seguía haciendo imagen→imagen y parecía que el
    /// modelo hubiera desaparecido. Generándolo aquí, todos lo dicen y
    /// ninguno puede quedarse desfasado.
    pub fn ui_label(&self) -> String {
        let modo = match (self.supports_t2i(), self.supports_i2i()) {
            (true, true) => "T2I+I2I",
            (true, false) => "T2I",
            (false, true) => "I2I",
            (false, false) => "?",
        };
        if self.max_refs > 0 {
            format!(
                "{}   ·  {} · hasta {} img{}",
                self.label,
                modo,
                self.max_refs,
                if self.max_refs == 1 { "" } else { "s" }
            )
        } else {
            format!("{}   ·  {}", self.label, modo)
        }
    }

    /// Identificador de red según el modo en que se vaya a usar.
    pub fn wire_id(&self, has_refs: bool) -> Option<&'static str> {
        if has_refs {
            self.i2i_id
        } else {
            self.t2i_id
        }
    }
}

/// Constructor abreviado para modelos sólo texto→imagen.
const fn t2i(
    label: &'static str,
    provider: ImageProvider,
    id: &'static str,
    size_style: SizeStyle,
) -> ModelSpec {
    ModelSpec {
        label,
        provider,
        t2i_id: Some(id),
        i2i_id: None,
        ref_field: RefField::None,
        max_refs: 0,
        size_style,
        aspect: "",
    }
}

pub const CATALOG: &[ModelSpec] = &[
    // ══ xAI ═══════════════════════════════════════════════════════════════
    //
    // La edición en xAI va a `/v1/images/edits`. La 2.4.0 ignoraba el modelo
    // elegido y enviaba siempre `grok-imagine-image-quality` —que ni siquiera
    // estaba en el catálogo—. Ahora cada entrada declara qué identificador
    // usa en edición.
    ModelSpec {
        label: "xAI — Grok Imagine Image",
        provider: ImageProvider::Xai,
        t2i_id: Some("grok-imagine-image"),
        i2i_id: Some("grok-imagine-image"),
        ref_field: RefField::XaiImages,
        max_refs: 2,
        aspect: "",
        size_style: SizeStyle::None,
    },
    ModelSpec {
        label: "xAI — Grok Imagine Image Pro",
        provider: ImageProvider::Xai,
        t2i_id: Some("grok-imagine-image-pro"),
        i2i_id: Some("grok-imagine-image-pro"),
        ref_field: RefField::XaiImages,
        max_refs: 2,
        aspect: "",
        size_style: SizeStyle::None,
    },
    // ══ Google ════════════════════════════════════════════════════════════
    // Sin imagen a imagen por este endpoint.
    t2i(
        "Google — gemini-2.5-flash-image",
        ImageProvider::Google,
        "gemini-2.5-flash-image",
        SizeStyle::None,
    ),
    t2i(
        "Google — gemini-3-pro-image-preview",
        ImageProvider::Google,
        "gemini-3-pro-image-preview",
        SizeStyle::None,
    ),
    // ══ OpenAI ════════════════════════════════════════════════════════════
    ModelSpec {
        label: "OpenAI — gpt-image-1.5",
        provider: ImageProvider::OpenAi,
        t2i_id: Some("gpt-image-1.5"),
        i2i_id: Some("gpt-image-1.5"),
        ref_field: RefField::OpenAiMultipart,
        max_refs: 1,
        aspect: "",
        size_style: SizeStyle::None,
    },
    ModelSpec {
        label: "OpenAI — gpt-image-1",
        provider: ImageProvider::OpenAi,
        t2i_id: Some("gpt-image-1"),
        i2i_id: Some("gpt-image-1"),
        ref_field: RefField::OpenAiMultipart,
        max_refs: 1,
        aspect: "",
        size_style: SizeStyle::None,
    },
    ModelSpec {
        label: "OpenAI — gpt-image-1-mini",
        provider: ImageProvider::OpenAi,
        t2i_id: Some("gpt-image-1-mini"),
        i2i_id: Some("gpt-image-1-mini"),
        ref_field: RefField::OpenAiMultipart,
        max_refs: 1,
        aspect: "",
        size_style: SizeStyle::None,
    },
    // dall-e-3 no soporta el endpoint de edición.
    t2i(
        "OpenAI — dall-e-3 (legacy)",
        ImageProvider::OpenAi,
        "dall-e-3",
        SizeStyle::None,
    ),
    // ══ WaveSpeed — Flux 2 (sólo texto→imagen) ════════════════════════════
    //
    // Aquí estaba el fallo H-04: `wavespeed_supports_i2i()` devolvía siempre
    // `true`, así que con una imagen cargada se construía un endpoint
    // inventado (`.../flux-2-max/image-to-image`) y WaveSpeed respondía un
    // 400 críptico. Con `i2i_id: None` se muestra el mensaje correcto.
    t2i(
        "WaveSpeed — Flux 2 Max", ImageProvider::WaveSpeed,
        "wavespeed-ai/flux-2-max/text-to-image", SizeStyle::WsSize),
    t2i(
        "WaveSpeed — Flux 2 Dev", ImageProvider::WaveSpeed,
        "wavespeed-ai/flux-2-dev/text-to-image", SizeStyle::WsSize),
    t2i(
        "WaveSpeed — Flux 2 Flash", ImageProvider::WaveSpeed,
        "wavespeed-ai/flux-2-flash/text-to-image", SizeStyle::WsSize),
    t2i(
        "WaveSpeed — Flux 2 Flex", ImageProvider::WaveSpeed,
        "wavespeed-ai/flux-2-flex/text-to-image", SizeStyle::WsSize),
    // ══ WaveSpeed — Flux Kontext (edición nativa, 1 imagen) ═══════════════
    ModelSpec {
        label: "WaveSpeed — Flux Kontext Max",
        provider: ImageProvider::WaveSpeed,
        t2i_id: Some("wavespeed-ai/flux-kontext-max/text-to-image"),
        i2i_id: Some("wavespeed-ai/flux-kontext-max"),
        ref_field: RefField::WsImage,
        max_refs: 1,
        aspect: "",
        size_style: SizeStyle::WsSize,
    },
    ModelSpec {
        label: "WaveSpeed — Flux Kontext Pro",
        provider: ImageProvider::WaveSpeed,
        t2i_id: Some("wavespeed-ai/flux-kontext-pro/text-to-image"),
        i2i_id: Some("wavespeed-ai/flux-kontext-pro"),
        ref_field: RefField::WsImage,
        max_refs: 1,
        aspect: "",
        size_style: SizeStyle::WsSize,
    },
    ModelSpec {
        label: "WaveSpeed — Flux Kontext Dev",
        provider: ImageProvider::WaveSpeed,
        t2i_id: Some("wavespeed-ai/flux-kontext-dev"),
        i2i_id: Some("wavespeed-ai/flux-kontext-dev"),
        ref_field: RefField::WsImage,
        max_refs: 1,
        aspect: "",
        size_style: SizeStyle::WsSize,
    },
    // ══ WaveSpeed — Flux Kontext Multi (hasta 5) ══════════════════════════
    ModelSpec {
        label: "WaveSpeed — Flux Kontext Max Multi",
        provider: ImageProvider::WaveSpeed,
        t2i_id: None,
        i2i_id: Some("wavespeed-ai/flux-kontext-max/multi"),
        ref_field: RefField::WsImages,
        max_refs: 5,
        aspect: "",
        size_style: SizeStyle::WsSize,
    },
    ModelSpec {
        label: "WaveSpeed — Flux Kontext Pro Multi",
        provider: ImageProvider::WaveSpeed,
        t2i_id: None,
        i2i_id: Some("wavespeed-ai/flux-kontext-pro/multi"),
        ref_field: RefField::WsImages,
        max_refs: 5,
        aspect: "",
        size_style: SizeStyle::WsSize,
    },
    // ══ WaveSpeed — WAN ═══════════════════════════════════════════════════
    ModelSpec {
        label: "WaveSpeed — WAN 2.2",
        provider: ImageProvider::WaveSpeed,
        t2i_id: None,
        i2i_id: Some("wavespeed-ai/wan-2.2/image-to-image"),
        ref_field: RefField::WsImages,
        max_refs: 2,
        aspect: "",
        size_style: SizeStyle::WsSize,
    },
    // WAN 2.6 no tiene endpoint de imagen a imagen; para eso está WAN 2.7.
    t2i(
        "WaveSpeed — WAN 2.6", ImageProvider::WaveSpeed,
        "alibaba/wan-2.6/text-to-image", SizeStyle::WsSize),
    ModelSpec {
        label: "WaveSpeed — WAN 2.7 Edit",
        provider: ImageProvider::WaveSpeed,
        t2i_id: None,
        i2i_id: Some("alibaba/wan-2.7/image-edit"),
        ref_field: RefField::WsImages,
        max_refs: 2,
        aspect: "",
        size_style: SizeStyle::WsSize,
    },
    // ══ WaveSpeed — UNO ═══════════════════════════════════════════════════
    ModelSpec {
        label: "WaveSpeed — UNO",
        provider: ImageProvider::WaveSpeed,
        t2i_id: None,
        i2i_id: Some("wavespeed-ai/uno"),
        ref_field: RefField::WsImages,
        max_refs: 5,
        aspect: "",
        size_style: SizeStyle::WsSize,
    },
    // ══ WaveSpeed — resto texto→imagen ════════════════════════════════════
    t2i(
        "WaveSpeed — Seedream 5.0 Lite", ImageProvider::WaveSpeed,
        "bytedance/seedream-v5.0-lite", SizeStyle::WsSize),
    t2i(
        "WaveSpeed — Seedream 4.5", ImageProvider::WaveSpeed,
        "bytedance/seedream-v4.5", SizeStyle::WsSize),
    t2i(
        "WaveSpeed — Nano Banana 2", ImageProvider::WaveSpeed,
        "google/nano-banana-2/text-to-image", SizeStyle::WsSize),
    t2i(
        "WaveSpeed — Nano Banana Pro", ImageProvider::WaveSpeed,
        "google/nano-banana-pro/text-to-image", SizeStyle::WsSize),
    t2i(
        "WaveSpeed — Dreamina 3.1", ImageProvider::WaveSpeed,
        "bytedance/dreamina-v3.1/text-to-image", SizeStyle::WsSize),
    t2i(
        "WaveSpeed — Qwen Image 2.0 Pro", ImageProvider::WaveSpeed,
        "wavespeed-ai/qwen-image-2.0-pro/text-to-image", SizeStyle::WsSize),
    t2i(
        "WaveSpeed — Kling O3", ImageProvider::WaveSpeed,
        "kwaivgi/kling-image-o3/text-to-image", SizeStyle::WsSize),
    t2i(
        "WaveSpeed — Grok 2 Image", ImageProvider::WaveSpeed,
        "x-ai/grok-2-image", SizeStyle::WsSize),
    t2i(
        "WaveSpeed — Grok Imagine Image", ImageProvider::WaveSpeed,
        "x-ai/grok-imagine-image-text-to-image", SizeStyle::WsSize),
    t2i(
        "WaveSpeed — GPT Image 2", ImageProvider::WaveSpeed,
        "openai/gpt-image-2/text-to-image", SizeStyle::WsSize),
    // ══ WaveSpeed — endpoints de edición ══════════════════════════════════
    ModelSpec {
        label: "WaveSpeed — Grok Imagine Edit",
        provider: ImageProvider::WaveSpeed,
        t2i_id: None,
        i2i_id: Some("x-ai/grok-imagine-image/edit"),
        ref_field: RefField::WsImage,
        max_refs: 1,
        aspect: "",
        size_style: SizeStyle::WsSize,
    },
    ModelSpec {
        label: "WaveSpeed — Nano Banana 2 Edit",
        provider: ImageProvider::WaveSpeed,
        t2i_id: None,
        i2i_id: Some("google/nano-banana-2/edit"),
        ref_field: RefField::WsImages,
        max_refs: 5, // la app expone 5 slots aunque el modelo admita más
        aspect: "",
        size_style: SizeStyle::WsSize,
    },
    ModelSpec {
        label: "WaveSpeed — Nano Banana 2 Edit Fast",
        provider: ImageProvider::WaveSpeed,
        t2i_id: None,
        i2i_id: Some("google/nano-banana-2/edit-fast"),
        ref_field: RefField::WsImages,
        max_refs: 5,
        aspect: "",
        size_style: SizeStyle::WsSize,
    },
    ModelSpec {
        label: "WaveSpeed — Nano Banana Pro Edit",
        provider: ImageProvider::WaveSpeed,
        t2i_id: None,
        i2i_id: Some("google/nano-banana-pro/edit"),
        ref_field: RefField::WsImages,
        max_refs: 5,
        aspect: "",
        size_style: SizeStyle::WsSize,
    },
    ModelSpec {
        label: "WaveSpeed — Seedream 5.0 Lite Edit",
        provider: ImageProvider::WaveSpeed,
        t2i_id: None,
        i2i_id: Some("bytedance/seedream-v5.0-lite/edit"),
        ref_field: RefField::WsImages,
        max_refs: 5,
        aspect: "",
        size_style: SizeStyle::WsSize,
    },
    ModelSpec {
        label: "WaveSpeed — Seedream 4.5 Edit",
        provider: ImageProvider::WaveSpeed,
        t2i_id: None,
        i2i_id: Some("bytedance/seedream-v4.5/edit"),
        ref_field: RefField::WsImages,
        max_refs: 5,
        aspect: "",
        size_style: SizeStyle::WsSize,
    },
    ModelSpec {
        label: "WaveSpeed — Qwen Image Edit",
        provider: ImageProvider::WaveSpeed,
        t2i_id: None,
        i2i_id: Some("wavespeed-ai/qwen-image/edit"),
        ref_field: RefField::WsImages,
        max_refs: 5,
        aspect: "",
        size_style: SizeStyle::WsSize,
    },
    ModelSpec {
        label: "WaveSpeed — Flux 2 Klein Edit",
        provider: ImageProvider::WaveSpeed,
        t2i_id: None,
        i2i_id: Some("wavespeed-ai/flux-2-klein-4b/edit"),
        ref_field: RefField::WsImages,
        max_refs: 5,
        aspect: "",
        size_style: SizeStyle::WsSize,
    },
    ModelSpec {
        label: "WaveSpeed — GPT Image 2 Edit",
        provider: ImageProvider::WaveSpeed,
        t2i_id: None,
        i2i_id: Some("openai/gpt-image-2/edit"),
        ref_field: RefField::WsImages,
        max_refs: 5,
        aspect: "",
        size_style: SizeStyle::WsSize,
    },
    // ══ Kie.AI ════════════════════════════════════════════════════════════
    //
    // Identificadores y nombres de campo verificados uno a uno contra
    // docs.kie.ai (agosto de 2026). Obsérvese que tres modelos distintos
    // llaman al mismo concepto `input_urls`, `image_urls` e `image_input`:
    // no hay forma de deducirlo del identificador, de ahí esta tabla.
    ModelSpec {
        // Ídem: sus valores de aspect_ratio no están documentados.
        label: "Kie.AI — GPT Image 2",
        provider: ImageProvider::KieAi,
        t2i_id: Some("gpt-image-2-text-to-image"),
        i2i_id: Some("gpt-image-2-image-to-image"),
        ref_field: RefField::KieInputUrls,
        max_refs: 5,
        aspect: "",
        size_style: SizeStyle::KieAspect,
    },
    ModelSpec {
        label: "Kie.AI — Nano Banana 2",
        provider: ImageProvider::KieAi,
        t2i_id: Some("nano-banana-2"),
        i2i_id: Some("nano-banana-2"),
        ref_field: RefField::KieImageInput,
        max_refs: 5,
        aspect: "auto",
        size_style: SizeStyle::KieAspectResolution,
    },
    ModelSpec {
        label: "Kie.AI — Nano Banana Pro",
        provider: ImageProvider::KieAi,
        t2i_id: Some("nano-banana-pro"),
        i2i_id: Some("nano-banana-pro"),
        ref_field: RefField::KieImageInput,
        max_refs: 5,
        aspect: "auto",
        size_style: SizeStyle::KieAspectResolution,
    },
    ModelSpec {
        label: "Kie.AI — Seedream 4.0",
        provider: ImageProvider::KieAi,
        t2i_id: Some("bytedance/seedream-v4-text-to-image"),
        i2i_id: None,
        ref_field: RefField::None,
        max_refs: 0,
        aspect: "square_hd",
        size_style: SizeStyle::KieImageSize,
    },
    ModelSpec {
        label: "Kie.AI — Seedream 4.0 Edit",
        provider: ImageProvider::KieAi,
        t2i_id: None,
        i2i_id: Some("bytedance/seedream-v4-edit"),
        ref_field: RefField::KieImageUrls,
        max_refs: 5,
        aspect: "square_hd",
        size_style: SizeStyle::KieImageSize,
    },
    // Grok en Kie.AI hace las dos cosas. Antes sólo estaba la edición, que
    // era una carencia: el desplegable daba a entender que por Kie.AI no se
    // podía generar con Grok desde texto.
    //
    // Nota sobre la facturación: la página de precios de Kie.AI agrupa estos
    // dos endpoints bajo la etiqueta `grok-imagine-image-2-0` (Text to Image
    // e Image Edit). Esa etiqueta es de facturación, no un identificador de
    // API: los que acepta `createTask` son los de abajo.
    ModelSpec {
        // `aspect` vacío a propósito: la documentación de Grok sólo muestra
        // un ejemplo con "3:2" y no enumera los valores admitidos. Enviar
        // "auto" era justo lo que provocaba el 500. Sin el campo, el modelo
        // aplica su propio valor por defecto.
        label: "Kie.AI — Grok Imagine",
        provider: ImageProvider::KieAi,
        t2i_id: Some("grok-imagine/text-to-image"),
        i2i_id: Some("grok-imagine/image-to-image"),
        ref_field: RefField::KieImageUrls,
        max_refs: 2,
        aspect: "3:2",
        size_style: SizeStyle::KieAspect,
    },
    ModelSpec {
        label: "Kie.AI — Flux.2 Pro",
        provider: ImageProvider::KieAi,
        t2i_id: Some("flux-2/pro-text-to-image"),
        i2i_id: Some("flux-2/pro-image-to-image"),
        ref_field: RefField::KieInputUrls,
        max_refs: 5,
        aspect: "1:1",
        size_style: SizeStyle::KieAspectResolution,
    },
    ModelSpec {
        label: "Kie.AI — Qwen Image 3.0",
        provider: ImageProvider::KieAi,
        t2i_id: Some("qwen3/text-to-image"),
        i2i_id: Some("qwen3/image-to-image"),
        ref_field: RefField::KieImageUrls,
        max_refs: 3, // la documentación fija el tope en 3
        aspect: "1:1",
        size_style: SizeStyle::KieQwen,
    },
    ModelSpec {
        label: "Kie.AI — Seedream 5.0 Pro",
        provider: ImageProvider::KieAi,
        t2i_id: Some("seedream/5-pro-text-to-image"),
        i2i_id: Some("seedream/5-pro-image-to-image"),
        ref_field: RefField::KieImageUrls,
        max_refs: 5,
        aspect: "1:1",
        size_style: SizeStyle::KieAspectQuality,
    },
    ModelSpec {
        label: "Kie.AI — Seedream 5.0 Lite",
        provider: ImageProvider::KieAi,
        t2i_id: Some("seedream/5-lite-text-to-image"),
        i2i_id: None,
        ref_field: RefField::None,
        max_refs: 0,
        aspect: "1:1",
        size_style: SizeStyle::KieAspectQuality,
    },
    ModelSpec {
        label: "Kie.AI — Nano Banana 2 Lite",
        provider: ImageProvider::KieAi,
        t2i_id: Some("nano-banana-2-lite"),
        i2i_id: Some("nano-banana-2-lite"),
        ref_field: RefField::KieImageUrls,
        max_refs: 5, // el modelo admite 10; la app expone 5 slots
        aspect: "auto",
        size_style: SizeStyle::KieAspect,
    },
];

/// Etiquetas para el desplegable, en el orden del catálogo.
///
/// La interfaz ya no lleva la lista escrita a mano: se rellena desde aquí,
/// de modo que es imposible que muestre un nombre y se envíe otro modelo.
pub fn labels() -> Vec<String> {
    CATALOG.iter().map(|m| m.ui_label()).collect()
}

pub fn get(index: usize) -> &'static ModelSpec {
    CATALOG.get(index).unwrap_or(&CATALOG[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todo_modelo_admite_al_menos_un_modo() {
        for m in CATALOG {
            assert!(
                m.supports_t2i() || m.supports_i2i(),
                "«{}» no admite ni texto→imagen ni imagen→imagen",
                m.label
            );
        }
    }

    #[test]
    fn coherencia_entre_i2i_campo_y_maximo() {
        for m in CATALOG {
            if m.supports_i2i() {
                assert_ne!(
                    m.ref_field,
                    RefField::None,
                    "«{}» admite I2I pero no declara campo de imagen",
                    m.label
                );
                assert!(
                    m.max_refs >= 1,
                    "«{}» admite I2I pero su máximo de referencias es 0",
                    m.label
                );
            } else {
                assert_eq!(
                    m.ref_field,
                    RefField::None,
                    "«{}» no admite I2I pero declara campo de imagen",
                    m.label
                );
                assert_eq!(m.max_refs, 0, "«{}» no admite I2I pero max_refs != 0", m.label);
            }
        }
    }

    #[test]
    fn las_etiquetas_son_unicas() {
        let mut vistas = std::collections::HashSet::new();
        for m in CATALOG {
            assert!(vistas.insert(m.label), "etiqueta duplicada: «{}»", m.label);
        }
    }

    #[test]
    fn los_campos_kie_pertenecen_a_modelos_kie() {
        for m in CATALOG {
            let es_campo_kie = matches!(
                m.ref_field,
                RefField::KieInputUrls | RefField::KieImageUrls | RefField::KieImageInput
            );
            assert_eq!(
                es_campo_kie,
                m.provider == ImageProvider::KieAi && m.ref_field != RefField::None,
                "«{}» mezcla proveedor y forma de campo",
                m.label
            );
        }
    }

    #[test]
    fn el_estilo_de_tamano_corresponde_al_proveedor() {
        for m in CATALOG {
            match m.size_style {
                SizeStyle::WsSize => assert_eq!(
                    m.provider,
                    ImageProvider::WaveSpeed,
                    "«{}» usa el tamaño de WaveSpeed sin serlo",
                    m.label
                ),
                SizeStyle::KieAspect
                | SizeStyle::KieAspectResolution
                | SizeStyle::KieImageSize
                | SizeStyle::KieQwen
                | SizeStyle::KieAspectQuality => assert_eq!(
                    m.provider,
                    ImageProvider::KieAi,
                    "«{}» usa un tamaño de Kie.AI sin serlo",
                    m.label
                ),
                SizeStyle::None => {}
            }
        }
    }

    #[test]
    fn hay_modelos_de_kie_ai_en_ambos_modos() {
        let kie: Vec<_> = CATALOG
            .iter()
            .filter(|m| m.provider == ImageProvider::KieAi)
            .collect();
        assert!(kie.len() >= 6, "faltan modelos de Kie.AI");
        assert!(kie.iter().any(|m| m.supports_t2i()));
        assert!(kie.iter().any(|m| m.supports_i2i()));
    }

    /// Deja constancia del tamaño real del catálogo: si alguien añade o
    /// quita modelos, este test recuerda actualizar el README.
    #[test]
    fn el_catalogo_tiene_el_tamano_documentado() {
        assert_eq!(
            CATALOG.len(),
            51,
            "el catálogo ha cambiado de tamaño: actualiza el README"
        );
    }

    #[test]
    fn get_tolera_indices_fuera_de_rango() {
        assert_eq!(get(0).label, CATALOG[0].label);
        assert_eq!(get(usize::MAX).label, CATALOG[0].label);
    }
}
