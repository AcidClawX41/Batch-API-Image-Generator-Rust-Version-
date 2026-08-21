//! api.rs — Cliente de generación de imágenes multi-proveedor (v2.5.0).
//!
//! Batch Image Generator — Eric Valls Gramunt
//!
//! Proveedores soportados:
//!   • xAI          — `/v1/images/generations` y `/v1/images/edits`
//!   • Google       — endpoint compatible con OpenAI (sólo texto→imagen)
//!   • OpenAI       — `/v1/images/generations` y `/v1/images/edits` (multipart)
//!   • WaveSpeed.ai — envío síncrono con respaldo por sondeo
//!   • Kie.AI       — subida de archivo + `createTask` + sondeo de `recordInfo`
//!
//! ENRUTADO
//! --------
//! Ni el endpoint ni el nombre del campo de imagen se deducen ya del
//! identificador del modelo: los declara `models::ModelSpec`. La explicación
//! completa está en `src/models.rs`.

use crate::models::{ModelSpec, RefField, SizeStyle};
use crate::util::take_chars;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

/// Cliente HTTP compartido.
///
/// La 2.4.0 construía un `reqwest::Client` nuevo en cada petición (seis
/// sitios distintos). Cada cliente lleva su propio pool de conexiones, así
/// que eso descartaba el keep-alive y forzaba un handshake TLS completo por
/// generación — dos, contando la descarga posterior de la imagen.
fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(concat!("BatchImageGenerator/", env!("CARGO_PKG_VERSION")))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

// ─── Proveedores ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageProvider {
    Xai,
    Google,
    OpenAi,
    WaveSpeed,
    KieAi,
}

impl ImageProvider {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Xai => "xAI",
            Self::Google => "Google",
            Self::OpenAi => "OpenAI",
            Self::WaveSpeed => "WaveSpeed",
            Self::KieAi => "Kie.AI",
        }
    }

    fn file_prefix(self) -> &'static str {
        match self {
            Self::Xai => "xai",
            Self::Google => "google",
            Self::OpenAi => "openai",
            Self::WaveSpeed => "wavespeed",
            Self::KieAi => "kieai",
        }
    }

    /// Cuál de los tres campos de clave de la interfaz alimenta a este
    /// proveedor.
    pub fn key_slot(self) -> KeySlot {
        match self {
            Self::WaveSpeed => KeySlot::WaveSpeed,
            Self::KieAi => KeySlot::KieAi,
            _ => KeySlot::General,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeySlot {
    General,
    WaveSpeed,
    KieAi,
}

// ─── Modo de imagen a imagen ─────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum I2iMode {
    /// La referencia guía el estilo; el prompt manda en el contenido.
    StyleReference,
    /// Se transforma directamente la imagen de referencia.
    DirectEdit,
}

// ─── Tipos compartidos ───────────────────────────────────────────────────────

#[derive(Debug)]
pub struct GenerationResult {
    pub filepath: String,
    pub filename: String,
}

/// Una imagen de referencia ya cargada en memoria: (base64, mime).
pub type RefImage = (String, String);

/// Canal para informar del avance mientras la petición está en curso.
///
/// Sin esto, una generación por Kie.AI dejaba el log parado en «Enviando
/// petición…» durante varios minutos: la subida de las referencias, la
/// creación de la tarea y el sondeo no decían nada, así que era imposible
/// distinguir «está trabajando» de «se ha colgado».
pub type ProgressFn = std::sync::Arc<dyn Fn(String) + Send + Sync>;

#[derive(Deserialize)]
struct ErrorResponse {
    error: Option<ErrorDetail>,
}

#[derive(Deserialize)]
struct ErrorDetail {
    message: Option<String>,
}

fn extract_error(body: &str) -> String {
    serde_json::from_str::<ErrorResponse>(body)
        .ok()
        .and_then(|e| e.error)
        .and_then(|e| e.message)
        .unwrap_or_else(|| take_chars(body, 400).to_string())
}

/// Un 4xx —salvo 429— es definitivo: reintentar no lo arregla.
fn is_fatal_status(status: u16) -> bool {
    (400..500).contains(&status) && status != 429
}

pub fn mime_from_ext(ext: &str) -> &'static str {
    match ext.to_ascii_lowercase().trim_start_matches('.') {
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "image/png",
    }
}

// ─── Punto de entrada ────────────────────────────────────────────────────────

/// Genera una imagen y la guarda en disco.
///
/// `ref_images` vacío ⇒ texto→imagen. Las referencias que excedan
/// `spec.max_refs` se descartan silenciosamente aquí; el aviso al usuario lo
/// emite quien llama, que es quien tiene el log.
#[allow(clippy::too_many_arguments)]
pub async fn generate_image(
    spec: &ModelSpec,
    api_key: &str,
    prompt: &str,
    output_dir: &str,
    ref_images: &[RefImage],
    i2i_mode: I2iMode,
    output_resolution: &str,
    progress: &ProgressFn,
) -> Result<GenerationResult, String> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err(format!(
            "Falta la API key de {}.",
            spec.provider.display_name()
        ));
    }

    let prompt = prompt.trim();
    if prompt.is_empty() {
        return Err("El prompt está vacío.".to_string());
    }

    std::fs::create_dir_all(output_dir).map_err(|e| format!("Error creando carpeta: {}", e))?;

    let has_refs = !ref_images.is_empty();

    // ── Compatibilidad ──
    //
    // Antes esto lo decidía `wavespeed_supports_i2i()`, que ignoraba su
    // argumento y devolvía siempre `true`: el mensaje de abajo era código
    // inalcanzable y el usuario recibía un 400 críptico del proveedor.
    if has_refs && !spec.supports_i2i() {
        return Err(format!(
            "❌ «{}» es un modelo de texto→imagen y no acepta imagen de referencia.\n\
             Quita las imágenes o elige un modelo de edición, por ejemplo:\n\
             • WaveSpeed Flux Kontext Max/Pro/Dev  (1 img)\n\
             • WaveSpeed Flux Kontext Multi / UNO  (hasta 5)\n\
             • Kie.AI GPT Image 2 · Nano Banana 2 / Pro · Seedream 4.0 Edit",
            spec.label
        ));
    }
    if !has_refs && !spec.supports_t2i() {
        return Err(format!(
            "❌ «{}» es un modelo de edición: necesita al menos una imagen de referencia.\n\
             Carga una imagen en «Img 1» o elige un modelo de texto→imagen.",
            spec.label
        ));
    }

    let refs: &[RefImage] = &ref_images[..ref_images.len().min(spec.max_refs)];

    match spec.provider {
        ImageProvider::KieAi => {
            generate_kie(spec, api_key, prompt, output_dir, refs, output_resolution, progress).await
        }
        ImageProvider::WaveSpeed => {
            generate_wavespeed(
                spec,
                api_key,
                prompt,
                output_dir,
                refs,
                i2i_mode,
                output_resolution,
                progress,
            )
            .await
        }
        ImageProvider::Xai if has_refs => {
            generate_xai_edit(spec, api_key, prompt, output_dir, refs).await
        }
        ImageProvider::OpenAi if has_refs => {
            generate_openai_edit(spec, api_key, prompt, output_dir, &refs[0]).await
        }
        _ => generate_openai_compat(spec, api_key, prompt, output_dir).await,
    }
}

// ─── Endpoint compatible con OpenAI (xAI, Google, OpenAI) ────────────────────

fn openai_api_url(provider: ImageProvider) -> &'static str {
    match provider {
        ImageProvider::Xai => "https://api.x.ai/v1/images/generations",
        ImageProvider::Google => {
            "https://generativelanguage.googleapis.com/v1beta/openai/images/generations"
        }
        _ => "https://api.openai.com/v1/images/generations",
    }
}

#[derive(Serialize)]
struct OpenAiRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    n: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<&'a str>,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    data: Option<Vec<OpenAiImageData>>,
}

#[derive(Deserialize)]
struct OpenAiImageData {
    b64_json: Option<String>,
    url: Option<String>,
}

async fn save_openai_payload(
    provider: ImageProvider,
    data: OpenAiResponse,
    output_dir: &str,
    context: &str,
) -> Result<GenerationResult, String> {
    let images = data.data.ok_or_else(|| {
        format!(
            "{} no devolvió imágenes ({}).",
            provider.display_name(),
            context
        )
    })?;
    let first = images.first().ok_or("Lista de imágenes vacía.")?;

    if let Some(b64) = first.b64_json.as_deref() {
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
            .map_err(|e| format!("Error decodificando base64: {}", e))?;
        save_image(provider, &bytes, output_dir, ext_from_magic(&bytes, None))
    } else if let Some(url) = first.url.as_deref() {
        download_and_save_for(provider, url, output_dir, None).await
    } else {
        Err("Sin datos base64 ni URL en la respuesta.".to_string())
    }
}

async fn generate_openai_compat(
    spec: &ModelSpec,
    api_key: &str,
    prompt: &str,
    output_dir: &str,
) -> Result<GenerationResult, String> {
    let provider = spec.provider;
    let model = spec
        .t2i_id
        .ok_or("Modelo sin identificador de texto→imagen.")?;

    // gpt-image-* rechaza `response_format`.
    let response_format = if provider == ImageProvider::OpenAi && model.starts_with("gpt-image-") {
        None
    } else {
        Some("b64_json")
    };

    let request = OpenAiRequest {
        model,
        prompt,
        n: 1,
        response_format,
    };

    let resp = http()
        .post(openai_api_url(provider))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| format_reqwest_error(provider, e))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!(
            "{} devolvió HTTP {}: {}",
            provider.display_name(),
            status.as_u16(),
            extract_error(&body)
        ));
    }

    let data: OpenAiResponse = resp.json().await.map_err(|e| {
        format!(
            "Error parseando respuesta de {}: {}",
            provider.display_name(),
            e
        )
    })?;

    save_openai_payload(provider, data, output_dir, "generations").await
}

// ─── OpenAI /v1/images/edits ─────────────────────────────────────────────────

async fn generate_openai_edit(
    spec: &ModelSpec,
    api_key: &str,
    prompt: &str,
    output_dir: &str,
    ref_image: &RefImage,
) -> Result<GenerationResult, String> {
    let (b64, mime) = ref_image;
    let model = spec.i2i_id.ok_or("Modelo sin identificador de edición.")?;

    let image_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
        .map_err(|e| format!("Error decodificando imagen de referencia: {}", e))?;

    let fname = match mime.as_str() {
        "image/jpeg" => "reference.jpg",
        "image/webp" => "reference.webp",
        "image/gif" => "reference.gif",
        _ => "reference.png",
    };

    let image_part = reqwest::multipart::Part::bytes(image_bytes)
        .file_name(fname)
        .mime_str(mime)
        .map_err(|e| format!("Error MIME: {}", e))?;

    let form = reqwest::multipart::Form::new()
        .text("model", model.to_string())
        .text("prompt", prompt.to_string())
        .text("n", "1")
        .part("image", image_part);

    let resp = http()
        .post("https://api.openai.com/v1/images/edits")
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .timeout(std::time::Duration::from_secs(180))
        .send()
        .await
        .map_err(|e| format_reqwest_error(ImageProvider::OpenAi, e))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!(
            "OpenAI edits devolvió HTTP {}: {}",
            status.as_u16(),
            extract_error(&body)
        ));
    }

    let data: OpenAiResponse = resp
        .json()
        .await
        .map_err(|e| format!("Error parseando respuesta de OpenAI edits: {}", e))?;

    save_openai_payload(ImageProvider::OpenAi, data, output_dir, "edits").await
}

// ─── xAI /v1/images/edits ────────────────────────────────────────────────────

#[derive(Serialize)]
struct XaiImageItem {
    #[serde(rename = "type")]
    item_type: &'static str,
    url: String,
}

#[derive(Serialize)]
struct XaiEditsRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    images: Vec<XaiImageItem>,
}

/// Edición en xAI.
///
/// La 2.4.0 llamaba a `xai_edit_model()`, que ignoraba el modelo elegido y
/// devolvía siempre la constante `"grok-imagine-image-quality"` — un
/// identificador que ni siquiera figuraba en el catálogo. El desplegable
/// mostraba una cosa y se ejecutaba otra. Ahora se envía el modelo declarado
/// en la tabla.
async fn generate_xai_edit(
    spec: &ModelSpec,
    api_key: &str,
    prompt: &str,
    output_dir: &str,
    ref_images: &[RefImage],
) -> Result<GenerationResult, String> {
    let model = spec.i2i_id.ok_or("Modelo sin identificador de edición.")?;

    let images: Vec<XaiImageItem> = ref_images
        .iter()
        .map(|(b64, mime)| XaiImageItem {
            item_type: "image_url",
            url: format!("data:{};base64,{}", mime, b64),
        })
        .collect();

    if images.is_empty() {
        return Err("xAI Grok edits necesita al menos una imagen de referencia.".to_string());
    }
    let num_images = images.len();

    let request = XaiEditsRequest {
        model,
        prompt,
        images,
    };

    let resp = http()
        .post("https://api.x.ai/v1/images/edits")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .timeout(std::time::Duration::from_secs(180))
        .send()
        .await
        .map_err(|e| format_reqwest_error(ImageProvider::Xai, e))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!(
            "xAI Grok edits ({} ref{}) devolvió HTTP {}: {}",
            num_images,
            if num_images == 1 { "" } else { "s" },
            status.as_u16(),
            extract_error(&body)
        ));
    }

    let data: OpenAiResponse = resp
        .json()
        .await
        .map_err(|e| format!("Error parseando respuesta de xAI Grok edits: {}", e))?;

    save_openai_payload(ImageProvider::Xai, data, output_dir, "edits").await
}

// ─── WaveSpeed ───────────────────────────────────────────────────────────────

const WAVESPEED_BASE: &str = "https://api.wavespeed.ai/api/v3";

#[derive(Deserialize)]
struct WaveSpeedResponse {
    message: Option<String>,
    data: Option<WaveSpeedData>,
}

#[derive(Deserialize)]
struct WaveSpeedData {
    id: Option<String>,
    status: Option<String>,
    outputs: Option<Vec<String>>,
    error: Option<String>,
}

fn wavespeed_size(spec: &ModelSpec, output_resolution: &str) -> &'static str {
    if !output_resolution.is_empty() {
        return match output_resolution {
            "1k" => "1024*1024",
            "2k" => "2048*2048",
            "4k" => "4096*4096",
            _ => "1024*1024",
        };
    }
    let id = spec.t2i_id.or(spec.i2i_id).unwrap_or("");
    if id.contains("seedream") || id.contains("dreamina") {
        "1920*1920"
    } else {
        "1024*1024"
    }
}

/// Construye el cuerpo JSON de una petición a WaveSpeed.
///
/// Está separado de la llamada de red **para poder probarlo**. El fallo
/// «Invalid request body: field "image" is required» de WAN 2.2 y Qwen Image
/// Edit sólo se veía enviando la petición de verdad y gastando créditos; con
/// esto se comprueba en un test de medio milisegundo.
fn wavespeed_body(
    spec: &ModelSpec,
    model: &str,
    prompt: &str,
    ref_images: &[RefImage],
    i2i_mode: I2iMode,
    output_resolution: &str,
) -> Result<serde_json::Value, String> {
    let has_refs = !ref_images.is_empty();

    let mut body = serde_json::json!({
        "prompt": prompt,
        "seed": -1_i64,
        "enable_sync_mode": true,
    });

    // GPT Image 2 vía WaveSpeed no acepta `size`.
    if spec.size_style == SizeStyle::WsSize && !model.contains("gpt-image-2") {
        body["size"] =
            serde_json::Value::String(wavespeed_size(spec, output_resolution).to_string());
    }

    if has_refs {
        // `strength` sólo tiene sentido en edición; enviarlo en texto→imagen
        // provocaba rechazos en algunos modelos.
        body["strength"] = serde_json::json!(match i2i_mode {
            I2iMode::DirectEdit => 0.85_f32,
            I2iMode::StyleReference => 0.55_f32,
        });

        let data_uris: Vec<String> = ref_images
            .iter()
            .map(|(b64, mime)| format!("data:{};base64,{}", mime, b64))
            .collect();

        // El nombre del campo sale de la tabla, no de adivinar por subcadenas
        // del identificador. Cada valor está verificado contra la
        // documentación del modelo (URL junto a su entrada en models.rs).
        match spec.ref_field {
            RefField::WsImage => {
                body["image"] = serde_json::Value::String(data_uris[0].clone());
            }
            RefField::WsImages => {
                body["images"] = serde_json::json!(data_uris);
            }
            other => {
                return Err(format!(
                    "Configuración incoherente: «{}» es de WaveSpeed pero declara {:?}.",
                    spec.label, other
                ));
            }
        }
    }

    Ok(body)
}

#[allow(clippy::too_many_arguments)]
async fn generate_wavespeed(
    spec: &ModelSpec,
    api_key: &str,
    prompt: &str,
    output_dir: &str,
    ref_images: &[RefImage],
    i2i_mode: I2iMode,
    output_resolution: &str,
    progress: &ProgressFn,
) -> Result<GenerationResult, String> {
    let has_refs = !ref_images.is_empty();
    let model = spec
        .wire_id(has_refs)
        .ok_or("Modelo sin identificador para este modo.")?;
    let url = format!("{}/{}", WAVESPEED_BASE, model);

    let body = wavespeed_body(
        spec,
        model,
        prompt,
        ref_images,
        i2i_mode,
        output_resolution,
    )?;

    let resp = http()
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .timeout(std::time::Duration::from_secs(180))
        .send()
        .await
        .map_err(|e| format_reqwest_error(ImageProvider::WaveSpeed, e))?;

    let status = resp.status();
    let body_str = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        let msg = serde_json::from_str::<WaveSpeedResponse>(&body_str)
            .ok()
            .and_then(|r| r.message)
            .unwrap_or_else(|| take_chars(&body_str, 400).to_string());

        if status.as_u16() == 400 && msg.contains("model not found") {
            return Err(format!(
                "❌ WaveSpeed no reconoce «{}» ({}).\n\
                 El identificador de la tabla de modelos puede haber cambiado en la API.",
                spec.label, model
            ));
        }
        return Err(format!(
            "WaveSpeed devolvió HTTP {}: {}",
            status.as_u16(),
            msg
        ));
    }

    handle_wavespeed_response(api_key, &body_str, output_dir, progress).await
}

async fn handle_wavespeed_response(
    api_key: &str,
    body: &str,
    output_dir: &str,
    progress: &ProgressFn,
) -> Result<GenerationResult, String> {
    let ws: WaveSpeedResponse = serde_json::from_str(body).map_err(|e| {
        format!(
            "Error parseando respuesta de WaveSpeed: {} — cuerpo: {}",
            e,
            take_chars(body, 200)
        )
    })?;

    let data = ws.data.ok_or("WaveSpeed no devolvió datos.")?;

    if let Some(err) = data.error.as_deref() {
        if !err.is_empty() {
            return Err(format!("WaveSpeed error: {}", err));
        }
    }
    if data.status.as_deref() == Some("failed") {
        return Err(format!(
            "WaveSpeed falló: {}",
            data.error.as_deref().unwrap_or("sin detalles")
        ));
    }
    if data.status.as_deref() == Some("completed") {
        let outputs = data.outputs.ok_or("WaveSpeed completó pero sin outputs.")?;
        let url = outputs.first().ok_or("Lista de outputs vacía.")?;
        return download_and_save_for(ImageProvider::WaveSpeed, url, output_dir, Some(progress)).await;
    }

    let task_id = data
        .id
        .ok_or_else(|| format!("WaveSpeed devolvió estado inesperado: {:?}", data.status))?;
    progress(format!("WaveSpeed: tarea {} en cola, esperando…", take_chars(&task_id, 16)));
    poll_wavespeed(api_key, &task_id, output_dir, progress).await
}

async fn poll_wavespeed(
    api_key: &str,
    task_id: &str,
    output_dir: &str,
    progress: &ProgressFn,
) -> Result<GenerationResult, String> {
    let poll_url = format!("{}/predictions/{}/result", WAVESPEED_BASE, task_id);

    for attempt in 0..180 {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let resp = http()
            .get(&poll_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Error consultando WaveSpeed: {}", e))?;

        let status = resp.status().as_u16();

        // Antes cualquier error HTTP se ignoraba con `continue`, hasta 180
        // veces: una key inválida costaba tres minutos de espera y terminaba
        // en un «timeout» que apuntaba en la dirección equivocada.
        if is_fatal_status(status) {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!(
                "WaveSpeed rechazó la consulta con HTTP {}: {}",
                status,
                extract_error(&body)
            ));
        }
        if !(200..300).contains(&status) {
            continue; // 5xx o 429: transitorio, se reintenta
        }

        let body = resp.text().await.unwrap_or_default();
        let Ok(ws) = serde_json::from_str::<WaveSpeedResponse>(&body) else {
            continue;
        };
        let Some(data) = ws.data else { continue };

        match data.status.as_deref() {
            Some("completed") => {
                let url = data
                    .outputs
                    .as_ref()
                    .and_then(|o| o.first())
                    .ok_or("WaveSpeed completó pero sin URLs de output.")?;
                return download_and_save_for(ImageProvider::WaveSpeed, url, output_dir, Some(progress)).await;
            }
            Some("failed") => {
                return Err(format!(
                    "WaveSpeed falló tras {}s: {}",
                    attempt + 1,
                    data.error.as_deref().unwrap_or("sin detalles")
                ));
            }
            other => {
                if attempt > 0 && attempt % 15 == 0 {
                    progress(format!(
                        "WaveSpeed: {} — {}s esperando…",
                        other.unwrap_or("en curso"),
                        attempt + 1
                    ));
                }
                continue;
            }
        }
    }

    Err("WaveSpeed: se agotó la espera del resultado (>3 min).".to_string())
}

// ─── Kie.AI ──────────────────────────────────────────────────────────────────
//
// El flujo es distinto al del resto de proveedores:
//
//   1. Las imágenes de referencia deben ser **URL públicas**: Kie.AI no
//      acepta data URIs. Se suben antes con la API de subida en base64, que
//      devuelve una URL temporal (se borra a los 3 días).
//   2. `POST /api/v1/jobs/createTask` devuelve un `taskId`.
//   3. `GET /api/v1/jobs/recordInfo?taskId=…` hasta `state == "success"`.
//   4. Las URL del resultado llegan dentro de `data.resultJson`, que es una
//      **cadena JSON anidada** con un array `resultUrls`.

const KIE_BASE: &str = "https://api.kie.ai/api/v1/jobs";
const KIE_UPLOAD_URL: &str = "https://kieai.redpandaai.co/api/file-base64-upload";

#[derive(Serialize)]
struct KieUploadRequest<'a> {
    #[serde(rename = "base64Data")]
    base64_data: String,
    #[serde(rename = "uploadPath")]
    upload_path: &'a str,
    #[serde(rename = "fileName")]
    file_name: String,
}

#[derive(Deserialize)]
struct KieUploadResponse {
    success: Option<bool>,
    msg: Option<String>,
    data: Option<KieUploadData>,
}

#[derive(Deserialize)]
struct KieUploadData {
    #[serde(rename = "downloadUrl")]
    download_url: Option<String>,
}

#[derive(Deserialize)]
struct KieCreateResponse {
    code: Option<i64>,
    msg: Option<String>,
    data: Option<KieCreateData>,
}

#[derive(Deserialize)]
struct KieCreateData {
    #[serde(rename = "taskId")]
    task_id: Option<String>,
}

#[derive(Deserialize)]
struct KieRecordResponse {
    data: Option<KieRecordData>,
}

#[derive(Deserialize)]
struct KieRecordData {
    state: Option<String>,
    #[serde(rename = "resultJson")]
    result_json: Option<String>,
    #[serde(rename = "failMsg")]
    fail_msg: Option<String>,
    #[serde(rename = "failCode")]
    fail_code: Option<serde_json::Value>,
}

/// `resultJson` es una cadena que contiene *otro* JSON. Se extrae la primera
/// URL del array `resultUrls`.
fn kie_first_result_url(result_json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(result_json).ok()?;
    v.get("resultUrls")?
        .as_array()?
        .first()?
        .as_str()
        .map(|s| s.to_string())
}

/// Sube una imagen y devuelve su URL pública.
async fn kie_upload_image(
    api_key: &str,
    index: usize,
    total: usize,
    (b64, mime): &RefImage,
    progress: &ProgressFn,
) -> Result<String, String> {
    // Una imagen de 4 MB son ~5,5 MB en base64: la subida puede tardar
    // bastante y conviene decirlo.
    progress(format!(
        "Kie.AI: subiendo imagen de referencia {}/{} ({} KB)…",
        index + 1,
        total,
        b64.len() / 1024
    ));
    let ext = match mime.as_str() {
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "png",
    };

    let request = KieUploadRequest {
        // La API acepta base64 puro o con prefijo data URI.
        base64_data: format!("data:{};base64,{}", mime, b64),
        upload_path: "images/batch-generator",
        file_name: format!("ref{}.{}", index + 1, ext),
    };

    let resp = http()
        .post(KIE_UPLOAD_URL)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| format_reqwest_error(ImageProvider::KieAi, e))?;

    let status = resp.status().as_u16();
    let body = resp.text().await.unwrap_or_default();

    if !(200..300).contains(&status) {
        return Err(format!(
            "Kie.AI rechazó la subida de la imagen {} (HTTP {}): {}",
            index + 1,
            status,
            extract_error(&body)
        ));
    }

    let parsed: KieUploadResponse = serde_json::from_str(&body).map_err(|e| {
        format!(
            "Respuesta de subida de Kie.AI ilegible: {} — {}",
            e,
            take_chars(&body, 200)
        )
    })?;

    if parsed.success == Some(false) {
        return Err(format!(
            "Kie.AI no pudo subir la imagen {}: {}",
            index + 1,
            parsed.msg.unwrap_or_else(|| "sin detalles".to_string())
        ));
    }

    parsed
        .data
        .and_then(|d| d.download_url)
        .filter(|u| !u.is_empty())
        .ok_or_else(|| {
            format!(
                "Kie.AI no devolvió URL para la imagen {} (respuesta: {}).",
                index + 1,
                take_chars(&body, 200)
            )
        })
}
/// Rellena los campos de tamaño y formato de la petición a Kie.AI.
///
/// Historia de dos correcciones seguidas, porque la primera se pasó de
/// frenada:
///
/// Primero: se enviaba `aspect_ratio: "auto"` a todos los modelos, pero sólo
/// la familia Nano Banana lo admite. Grok y Flux.2 devolvían HTTP 500 con
/// «aspect_ratio is not within the range of allowed options». De ahí que el
/// valor lo declare cada modelo en `spec.aspect`.
///
/// Después: al corregir aquello dejé de enviar también `quality`,
/// `output_format` y `resolution` cuando la resolución estaba en «Auto». Pero
/// todos los ejemplos de la documentación los incluyen, y Seedream 5.0 Pro
/// pasó a responder «This field is required». La lección: el problema era el
/// valor de un campo, no la presencia de los campos.
///
/// Ahora cada familia envía exactamente el juego de campos que aparece en su
/// ejemplo oficial, y «Auto» significa usar el valor por defecto de ese
/// ejemplo, no omitir el campo.
fn kie_apply_size(input: &mut serde_json::Value, spec: &ModelSpec, output_resolution: &str) {
    let resolution = match output_resolution {
        "2k" => "2K",
        "4k" => "4K",
        // «Auto» → el valor por defecto de los ejemplos oficiales.
        _ => "1K",
    };

    match spec.size_style {
        // Nano Banana 2 / 2 Lite / Pro y Flux.2:
        //   { aspect_ratio, resolution, output_format }
        SizeStyle::KieAspectResolution => {
            if !spec.aspect.is_empty() {
                input["aspect_ratio"] = serde_json::json!(spec.aspect);
            }
            input["resolution"] = serde_json::json!(resolution);
            input["output_format"] = serde_json::json!("png");
        }

        // Seedream 5.x:
        //   { aspect_ratio, quality, output_format }
        // No expone campo de resolución; la calidad va por `quality`.
        SizeStyle::KieAspectQuality => {
            if !spec.aspect.is_empty() {
                input["aspect_ratio"] = serde_json::json!(spec.aspect);
            }
            input["quality"] = serde_json::json!("basic");
            input["output_format"] = serde_json::json!("png");
        }

        // Seedream 4.0: aquí `image_size` es un preajuste («square_hd»),
        // no una relación de aspecto.
        SizeStyle::KieImageSize => {
            if !spec.aspect.is_empty() {
                input["image_size"] = serde_json::json!(spec.aspect);
            }
            input["image_resolution"] = serde_json::json!(resolution);
            input["max_images"] = serde_json::json!(1);
        }

        // Qwen 3: llama `image_size` a la relación de aspecto —el mismo
        // nombre que Seedream 4.0 usa para otra cosa— y sólo admite 1K o 2K.
        SizeStyle::KieQwen => {
            if !spec.aspect.is_empty() {
                input["image_size"] = serde_json::json!(spec.aspect);
            }
            input["resolution"] =
                serde_json::json!(if resolution == "1K" { "1K" } else { "2K" });
            input["output_format"] = serde_json::json!("png");
        }

        // Grok y GPT Image 2: sus ejemplos sólo llevan `aspect_ratio`.
        SizeStyle::KieAspect => {
            if !spec.aspect.is_empty() {
                input["aspect_ratio"] = serde_json::json!(spec.aspect);
            }
        }

        SizeStyle::None | SizeStyle::WsSize => {}
    }
}

async fn generate_kie(
    spec: &ModelSpec,
    api_key: &str,
    prompt: &str,
    output_dir: &str,
    ref_images: &[RefImage],
    output_resolution: &str,
    progress: &ProgressFn,
) -> Result<GenerationResult, String> {
    let has_refs = !ref_images.is_empty();
    let model = spec
        .wire_id(has_refs)
        .ok_or("Modelo sin identificador para este modo.")?;

    let mut input = serde_json::json!({ "prompt": prompt });
    kie_apply_size(&mut input, spec, output_resolution);

    if has_refs {
        // Kie.AI necesita URL públicas: hay que subir antes cada imagen.
        let mut urls = Vec::with_capacity(ref_images.len());
        for (i, img) in ref_images.iter().enumerate() {
            urls.push(kie_upload_image(api_key, i, ref_images.len(), img, progress).await?);
        }
        progress(format!(
            "Kie.AI: {} imagen{} subida{}.",
            urls.len(),
            if urls.len() == 1 { "" } else { "es" },
            if urls.len() == 1 { "" } else { "s" }
        ));

        let field = match spec.ref_field {
            RefField::KieInputUrls => "input_urls",
            RefField::KieImageUrls => "image_urls",
            RefField::KieImageInput => "image_input",
            other => {
                return Err(format!(
                    "Configuración incoherente: «{}» es de Kie.AI pero declara {:?}.",
                    spec.label, other
                ));
            }
        };
        input[field] = serde_json::json!(urls);
    }

    let body = serde_json::json!({ "model": model, "input": input });
    progress(format!("Kie.AI: creando tarea para «{}»…", model));

    let resp = http()
        .post(format!("{}/createTask", KIE_BASE))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| format_reqwest_error(ImageProvider::KieAi, e))?;

    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();

    if !(200..300).contains(&status) {
        return Err(format!(
            "Kie.AI devolvió HTTP {} al crear la tarea: {}",
            status,
            extract_error(&text)
        ));
    }

    let created: KieCreateResponse = serde_json::from_str(&text).map_err(|e| {
        format!(
            "Respuesta de Kie.AI ilegible: {} — {}",
            e,
            take_chars(&text, 200)
        )
    })?;

    // Kie.AI puede devolver HTTP 200 con un código de error en el cuerpo.
    if let Some(code) = created.code {
        if code != 200 {
            return Err(format!(
                "Kie.AI rechazó la tarea (código {}): {}",
                code,
                created.msg.unwrap_or_else(|| "sin detalles".to_string())
            ));
        }
    }

    let task_id = created
        .data
        .and_then(|d| d.task_id)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| format!("Kie.AI no devolvió taskId: {}", take_chars(&text, 200)))?;

    progress(format!(
        "Kie.AI: tarea {} aceptada; generando…",
        take_chars(&task_id, 16)
    ));
    poll_kie(api_key, &task_id, output_dir, progress).await
}

async fn poll_kie(
    api_key: &str,
    task_id: &str,
    output_dir: &str,
    progress: &ProgressFn,
) -> Result<GenerationResult, String> {
    let url = format!("{}/recordInfo", KIE_BASE);
    let mut esperado = 0u64;

    for attempt in 0..140 {
        // Espera escalonada: los primeros segundos se consulta a menudo
        // porque algunos modelos responden en 5-10 s; después se espacia
        // para no castigar la API en generaciones largas.
        let espera = if attempt < 10 {
            1
        } else if attempt < 40 {
            2
        } else {
            5
        };
        tokio::time::sleep(std::time::Duration::from_secs(espera)).await;
        esperado += espera;

        let resp = http()
            .get(&url)
            .query(&[("taskId", task_id)])
            .header("Authorization", format!("Bearer {}", api_key))
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Error consultando Kie.AI: {}", e))?;

        let status = resp.status().as_u16();
        if is_fatal_status(status) {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!(
                "Kie.AI rechazó la consulta con HTTP {}: {}",
                status,
                extract_error(&body)
            ));
        }
        if !(200..300).contains(&status) {
            continue;
        }

        let body = resp.text().await.unwrap_or_default();
        let Ok(record) = serde_json::from_str::<KieRecordResponse>(&body) else {
            continue;
        };
        let Some(data) = record.data else { continue };

        match data.state.as_deref() {
            Some("success") => {
                progress(format!("Kie.AI: listo en {}s, descargando…", esperado));
                let result_json = data
                    .result_json
                    .ok_or("Kie.AI marcó éxito pero no devolvió resultJson.")?;
                let image_url = kie_first_result_url(&result_json).ok_or_else(|| {
                    format!(
                        "Kie.AI devolvió un resultJson sin resultUrls: {}",
                        take_chars(&result_json, 200)
                    )
                })?;
                return download_and_save_for(ImageProvider::KieAi, &image_url, output_dir, Some(progress)).await;
            }
            Some("fail") => {
                let code = data
                    .fail_code
                    .map(|c| c.to_string().trim_matches('"').to_string())
                    .unwrap_or_else(|| "?".to_string());
                let msg = data.fail_msg.unwrap_or_else(|| "sin detalles".to_string());

                // El 524 lo emite Kie.AI cuando su propio worker agota el
                // tiempo, no cuando la petición es incorrecta: conviene
                // distinguirlo para no buscar el fallo donde no está.
                if code == "524" || msg.contains("task timeout") {
                    return Err(format!(
                        "⏱ Kie.AI agotó su propio tiempo de generación tras {}s (código {}).\n\
                         No es un problema de la petición: la tarea se aceptó y su servidor \
                         no la terminó a tiempo. Suele pasar en horas de carga o con modelos \
                         pesados en imagen→imagen. Reintenta, baja la resolución o prueba con \
                         un modelo más ligero.",
                        esperado, code
                    ));
                }

                return Err(format!(
                    "Kie.AI falló tras {}s (código {}): {}",
                    esperado, code, msg
                ));
            }
            // waiting · queuing · generating
            other => {
                // Un aviso cada ~15 s: suficiente para saber que sigue vivo
                // sin inundar el log.
                if esperado > 0 && esperado % 15 < espera {
                    progress(format!(
                        "Kie.AI: {} — {}s transcurridos…",
                        match other {
                            Some("waiting") => "en espera",
                            Some("queuing") => "en cola",
                            Some("generating") => "generando",
                            Some(o) => o,
                            None => "sin estado",
                        },
                        esperado
                    ));
                }
                continue;
            }
        }
    }

    Err(format!(
        "Kie.AI: se agotó la espera del resultado tras {}s. La tarea {} puede seguir \
         viva en su panel; si esto se repite, prueba con otro modelo o revisa el \
         saldo de tu cuenta.",
        esperado,
        take_chars(task_id, 24)
    ))
}

// ─── Guardado ────────────────────────────────────────────────────────────────

static SAVE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Guarda la imagen sin sobrescribir nunca un archivo existente.
///
/// La 2.4.0 nombraba con resolución de **un segundo** y escribía con
/// `fs::write`, que trunca. En modo Burst dos generaciones del mismo segundo
/// producían un único archivo: la segunda pisaba a la primera en silencio.
fn save_image(
    provider: ImageProvider,
    bytes: &[u8],
    output_dir: &str,
    ext: &str,
) -> Result<GenerationResult, String> {
    use std::io::Write;

    let dir = std::path::Path::new(output_dir);

    for _ in 0..64 {
        let ts = chrono::Local::now().format("%Y%m%d_%H%M%S%.3f").to_string();
        let seq = SAVE_COUNTER.fetch_add(1, Ordering::Relaxed) % 10_000;
        let filename = format!(
            "{}_{}_{:04}.{}",
            provider.file_prefix(),
            ts.replace('.', ""),
            seq,
            ext
        );
        let filepath = dir.join(&filename);

        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&filepath)
        {
            Ok(mut f) => {
                f.write_all(bytes)
                    .map_err(|e| format!("Error guardando {}: {}", filepath.display(), e))?;
                return Ok(GenerationResult {
                    filepath: filepath.to_string_lossy().to_string(),
                    filename,
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("Error guardando en {}: {}", dir.display(), e)),
        }
    }

    Err("No se pudo encontrar un nombre de archivo libre tras 64 intentos.".to_string())
}

/// Deduce la extensión real a partir de los primeros bytes.
///
/// La 2.4.0 guardaba siempre `.png`, aunque varios proveedores devuelven
/// JPEG o WebP: los archivos mentían sobre su formato.
fn ext_from_magic(bytes: &[u8], content_type: Option<&str>) -> &'static str {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return "jpg";
    }
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        return "png";
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return "webp";
    }
    if bytes.starts_with(b"GIF8") {
        return "gif";
    }
    match content_type.unwrap_or("") {
        t if t.contains("jpeg") || t.contains("jpg") => "jpg",
        t if t.contains("webp") => "webp",
        t if t.contains("gif") => "gif",
        _ => "png",
    }
}

/// Intentos de descarga de la imagen ya generada.
///
/// POR QUÉ HAY REINTENTOS
/// ----------------------
/// Cuando la descarga falla, el trabajo **ya está hecho y ya está pagado**:
/// el proveedor generó la imagen y nos dio su URL. Un corte de red de un
/// segundo contra la CDN tiraba a la basura una generación entera. Se observó
/// en uso real con WAN 2.7 Edit: la API respondió correctamente y la descarga
/// desde CloudFront murió con «error sending request for url».
///
/// Es la asimetría lo que justifica reintentar aquí y no en la petición de
/// generación: repetir una descarga es gratis, repetir una generación no.
const DESCARGA_INTENTOS: u32 = 4;

/// Resultado de un intento: el mensaje y si merece la pena repetir.
struct FalloDescarga {
    mensaje: String,
    reintentable: bool,
}

/// Devuelve los bytes y el `Content-Type` de un intento.
///
/// Se convierte a `Vec<u8>` en vez de exponer `bytes::Bytes` para no añadir
/// una dependencia directa sólo por un tipo de retorno interno.
async fn intento_de_descarga(url: &str) -> Result<(Vec<u8>, Option<String>), FalloDescarga> {
    let resp = http()
        .get(url)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| FalloDescarga {
            mensaje: if e.is_timeout() {
                "la CDN tardó más de 120 s".to_string()
            } else if e.is_connect() {
                format!("no se pudo conectar con la CDN: {e}")
            } else {
                format!("{e}")
            },
            // Timeouts y cortes de conexión son transitorios por definición.
            reintentable: true,
        })?;

    let status = resp.status();
    if !status.is_success() {
        return Err(FalloDescarga {
            mensaje: format!("HTTP {}", status.as_u16()),
            // 403/404 en una URL firmada significa caducada o inexistente:
            // repetir sólo hace perder tiempo. 5xx y 429 sí se reintentan.
            reintentable: status.is_server_error() || status.as_u16() == 429,
        });
    }

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_ascii_lowercase());

    // El corte puede llegar a mitad del cuerpo, no sólo al conectar.
    let bytes = resp.bytes().await.map_err(|e| FalloDescarga {
        mensaje: format!("la descarga se cortó a medias: {e}"),
        reintentable: true,
    })?;

    Ok((bytes.to_vec(), content_type))
}

async fn download_and_save_for(
    provider: ImageProvider,
    url: &str,
    output_dir: &str,
    progress: Option<&ProgressFn>,
) -> Result<GenerationResult, String> {
    let aviso = |msg: String| {
        if let Some(p) = progress {
            p(msg);
        }
    };

    let mut ultimo = String::new();
    for intento in 1..=DESCARGA_INTENTOS {
        match intento_de_descarga(url).await {
            Ok((bytes, content_type)) => {
                if intento > 1 {
                    aviso(format!("Descarga recuperada en el intento {intento}."));
                }
                let ext = ext_from_magic(&bytes, content_type.as_deref());
                return save_image(provider, &bytes, output_dir, ext);
            }
            Err(fallo) => {
                ultimo = fallo.mensaje;
                if !fallo.reintentable || intento == DESCARGA_INTENTOS {
                    break;
                }
                // 1 s, 2 s, 4 s: suficiente para un corte pasajero sin dejar
                // la aplicación colgada un minuto.
                let espera = 1u64 << (intento - 1);
                aviso(format!(
                    "Fallo al descargar la imagen ({ultimo}). Reintento {}/{} en {espera}s…",
                    intento + 1,
                    DESCARGA_INTENTOS
                ));
                tokio::time::sleep(std::time::Duration::from_secs(espera)).await;
            }
        }
    }

    // La imagen existe y está pagada: se da la URL para poder rescatarla a
    // mano antes de que caduque.
    aviso(format!("URL de la imagen no descargada: {url}"));
    Err(format!(
        "La imagen se generó pero no se pudo descargar tras {DESCARGA_INTENTOS} intentos ({ultimo}). \
         La URL está en el log; cópiala al navegador antes de que caduque."
    ))
}

fn format_reqwest_error(provider: ImageProvider, e: reqwest::Error) -> String {
    if e.is_timeout() {
        format!("Timeout: {} tardó demasiado.", provider.display_name())
    } else if e.is_connect() {
        format!(
            "Error de conexión con {}. Verifica tu red.",
            provider.display_name()
        )
    } else {
        format!("Error HTTP con {}: {}", provider.display_name(), e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Servidor mínimo que corta la conexión las `fallos` primeras veces y
    /// después devuelve un PNG válido. Sirve para comprobar que una descarga
    /// interrumpida se recupera sola en vez de tirar la generación.
    fn servidor_que_falla(fallos: usize) -> String {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let puerto = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let mut n = 0;
            for flujo in listener.incoming() {
                let Ok(mut flujo) = flujo else { continue };
                if n < fallos {
                    n += 1;
                    // Cerrar sin responder: el cliente ve un corte de red.
                    drop(flujo);
                    continue;
                }
                let mut buf = [0u8; 1024];
                let _ = flujo.read(&mut buf);
                let png: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
                let cabecera = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    png.len()
                );
                let _ = flujo.write_all(cabecera.as_bytes());
                let _ = flujo.write_all(&png);
                let _ = flujo.flush();
                return;
            }
        });
        format!("http://127.0.0.1:{puerto}/imagen.png")
    }

    #[tokio::test]
    async fn una_descarga_cortada_se_reintenta_y_se_recupera() {
        let dir = std::env::temp_dir().join(format!(
            "big-test-descarga-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let url = servidor_que_falla(2);

        let avisos = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let capturados = avisos.clone();
        let progress: ProgressFn =
            std::sync::Arc::new(move |m: String| capturados.lock().unwrap().push(m));

        let res = download_and_save_for(
            ImageProvider::WaveSpeed,
            &url,
            dir.to_str().unwrap(),
            Some(&progress),
        )
        .await;

        assert!(res.is_ok(), "debería recuperarse: {res:?}");
        let dichos = avisos.lock().unwrap().clone();
        assert!(
            dichos.iter().any(|m| m.contains("Reintento")),
            "el usuario tiene que ver que se está reintentando: {dichos:?}"
        );
        assert!(
            dichos.iter().any(|m| m.contains("recuperada")),
            "y que se recuperó: {dichos:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Un 404 no se reintenta: la URL no va a aparecer por esperar, y cada
    /// intento son segundos perdidos.
    #[tokio::test]
    async fn un_404_no_se_reintenta() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let puerto = listener.local_addr().unwrap().port();
        let intentos = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let contador = intentos.clone();
        std::thread::spawn(move || {
            for flujo in listener.incoming().take(4) {
                let Ok(mut flujo) = flujo else { continue };
                contador.fetch_add(1, Ordering::SeqCst);
                // Hay que leer la petición antes de contestar: cerrar sin
                // leerla provoca un RST y el cliente ve un corte de red en
                // lugar del 404, que es justo lo que este test distingue.
                let mut buf = [0u8; 1024];
                let _ = flujo.read(&mut buf);
                let _ = flujo.write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
                let _ = flujo.flush();
            }
        });

        let dir = std::env::temp_dir().join("big-test-404");
        let res = download_and_save_for(
            ImageProvider::WaveSpeed,
            &format!("http://127.0.0.1:{puerto}/no-existe.png"),
            dir.to_str().unwrap(),
            None,
        )
        .await;

        assert!(res.is_err());
        assert_eq!(
            intentos.load(Ordering::SeqCst),
            1,
            "un 404 sólo debe intentarse una vez"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// El bug que Eric vio en uso real: WAN 2.2 y Qwen Image Edit contestaban
    /// `Invalid request body: field "image" is required` porque se les enviaba
    /// `images` (array). Este test comprueba el cuerpo exacto que sale por el
    /// cable, sin gastar una sola llamada a la API.
    #[test]
    fn el_cuerpo_de_wavespeed_usa_el_campo_documentado_de_cada_modelo() {
        let refs: Vec<RefImage> = vec![("QUJD".into(), "image/png".into())];

        let casos: &[(&str, &str)] = &[
            ("WaveSpeed — WAN 2.2", "image"),
            ("WaveSpeed — Qwen Image Edit", "image"),
            ("WaveSpeed — Grok Imagine Edit", "image"),
            ("WaveSpeed — WAN 2.7 Edit", "images"),
            ("WaveSpeed — UNO", "images"),
        ];

        for (label, campo) in casos {
            let spec = crate::models::CATALOG
                .iter()
                .find(|m| m.label == *label)
                .unwrap_or_else(|| panic!("«{label}» ya no está en la tabla"));
            let model = spec.wire_id(true).expect("debe tener endpoint de I2I");
            let body =
                wavespeed_body(spec, model, "un prompt", &refs, I2iMode::DirectEdit, "").unwrap();

            assert!(
                body.get(campo).is_some(),
                "«{label}» debe enviar el campo `{campo}`; envió: {}",
                body
            );
            let otro = if *campo == "image" { "images" } else { "image" };
            assert!(
                body.get(otro).is_none(),
                "«{label}» no debe enviar además `{otro}`"
            );

            // Tipo correcto: cadena para `image`, array para `images`.
            if *campo == "image" {
                assert!(body[campo].is_string(), "`image` debe ser una cadena");
            } else {
                assert!(body[campo].is_array(), "`images` debe ser un array");
            }
        }
    }

    /// Sin imágenes de referencia no debe colarse ni el campo de imagen ni
    /// `strength`: en texto→imagen provocaban rechazos.
    #[test]
    fn en_texto_a_imagen_no_se_envia_ni_imagen_ni_strength() {
        let spec = crate::models::CATALOG
            .iter()
            .find(|m| m.label == "WaveSpeed — Flux 2 Max")
            .expect("modelo de referencia para el test");
        let model = spec.wire_id(false).unwrap();
        let body = wavespeed_body(spec, model, "un prompt", &[], I2iMode::DirectEdit, "").unwrap();
        assert!(body.get("image").is_none());
        assert!(body.get("images").is_none());
        assert!(body.get("strength").is_none());
        assert_eq!(body["prompt"], "un prompt");
    }

    #[test]
    fn extension_por_magic_bytes() {
        assert_eq!(ext_from_magic(&[0xFF, 0xD8, 0xFF, 0xE0], None), "jpg");
        assert_eq!(ext_from_magic(&[0x89, b'P', b'N', b'G'], None), "png");
        assert_eq!(ext_from_magic(b"GIF89a", None), "gif");
        assert_eq!(ext_from_magic(b"RIFF\x00\x00\x00\x00WEBPVP8 ", None), "webp");
    }

    #[test]
    fn extension_por_content_type_si_no_hay_magic() {
        assert_eq!(ext_from_magic(b"basura", Some("image/jpeg")), "jpg");
        assert_eq!(ext_from_magic(b"basura", Some("image/webp")), "webp");
        assert_eq!(ext_from_magic(b"", None), "png");
    }

    /// Algunos proveedores anuncian `image/png` y envían JPEG.
    #[test]
    fn magic_bytes_manda_sobre_content_type_erroneo() {
        assert_eq!(
            ext_from_magic(&[0xFF, 0xD8, 0xFF, 0xE0], Some("image/png")),
            "jpg"
        );
    }

    #[test]
    fn ext_from_magic_no_desborda_con_entradas_cortas() {
        for n in 0..12 {
            let _ = ext_from_magic(&vec![b'R'; n], None);
        }
    }

    #[test]
    fn mime_desde_extension() {
        assert_eq!(mime_from_ext("JPG"), "image/jpeg");
        assert_eq!(mime_from_ext(".jpeg"), "image/jpeg");
        assert_eq!(mime_from_ext("webp"), "image/webp");
        assert_eq!(mime_from_ext("desconocido"), "image/png");
    }

    #[test]
    fn los_4xx_son_definitivos_menos_el_429() {
        assert!(is_fatal_status(400));
        assert!(is_fatal_status(401));
        assert!(is_fatal_status(404));
        assert!(!is_fatal_status(429), "el 429 debe reintentarse");
        assert!(!is_fatal_status(500));
        assert!(!is_fatal_status(200));
    }

    #[test]
    fn se_extrae_la_url_del_resultjson_anidado() {
        let rj = r#"{"resultUrls":["https://cdn.kie.ai/a.png","https://cdn.kie.ai/b.png"]}"#;
        assert_eq!(
            kie_first_result_url(rj).as_deref(),
            Some("https://cdn.kie.ai/a.png")
        );
    }

    #[test]
    fn resultjson_invalido_no_entra_en_panico() {
        assert!(kie_first_result_url("").is_none());
        assert!(kie_first_result_url("no es json").is_none());
        assert!(kie_first_result_url("{}").is_none());
        assert!(kie_first_result_url(r#"{"resultUrls":[]}"#).is_none());
        assert!(kie_first_result_url(r#"{"resultUrls":"no es array"}"#).is_none());
        assert!(kie_first_result_url(r#"{"resultUrls":[123]}"#).is_none());
    }

    fn spec_de(label: &str) -> &'static ModelSpec {
        crate::models::CATALOG
            .iter()
            .find(|m| m.label == label)
            .unwrap_or_else(|| panic!("no existe «{}»", label))
    }

    /// Regresión 1: se enviaba `aspect_ratio: "auto"` a todos los modelos y
    /// Grok y Flux.2 devolvían HTTP 500 con
    /// «aspect_ratio is not within the range of allowed options».
    #[test]
    fn nunca_se_envia_un_aspecto_que_el_modelo_no_declara() {
        for m in crate::models::CATALOG {
            if m.provider != ImageProvider::KieAi {
                continue;
            }
            let mut v = serde_json::json!({});
            kie_apply_size(&mut v, m, "");
            for campo in ["aspect_ratio", "image_size"] {
                if let Some(val) = v.get(campo).and_then(|x| x.as_str()) {
                    assert_eq!(
                        val, m.aspect,
                        "«{}» envía un {} distinto del que declara",
                        m.label, campo
                    );
                }
            }
        }
    }

    #[test]
    fn solo_nano_banana_recibe_auto() {
        let mut v = serde_json::json!({});
        kie_apply_size(&mut v, spec_de("Kie.AI — Nano Banana Pro"), "");
        assert_eq!(v["aspect_ratio"], "auto");

        let mut v = serde_json::json!({});
        kie_apply_size(&mut v, spec_de("Kie.AI — Grok Imagine"), "");
        assert_eq!(v["aspect_ratio"], "3:2", "Grok usa el valor de su ejemplo oficial");

        let mut v = serde_json::json!({});
        kie_apply_size(&mut v, spec_de("Kie.AI — Flux.2 Pro"), "");
        assert_eq!(v["aspect_ratio"], "1:1");
    }

    /// Regresión 2: al corregir lo anterior dejé de enviar `quality`,
    /// `output_format` y `resolution` con la resolución en «Auto», y
    /// Seedream 5.0 Pro respondió «This field is required». Los ejemplos
    /// oficiales incluyen siempre esos campos.
    #[test]
    fn seedream_5_envia_el_juego_completo_de_campos() {
        for etiqueta in ["Kie.AI — Seedream 5.0 Pro", "Kie.AI — Seedream 5.0 Lite"] {
            let mut v = serde_json::json!({});
            kie_apply_size(&mut v, spec_de(etiqueta), "");
            assert_eq!(v["aspect_ratio"], "1:1", "{etiqueta}");
            assert_eq!(v["quality"], "basic", "{etiqueta}: falta `quality`");
            assert_eq!(v["output_format"], "png", "{etiqueta}: falta `output_format`");
        }
    }

    #[test]
    fn nano_banana_envia_resolucion_y_formato_incluso_en_auto() {
        for etiqueta in ["Kie.AI — Nano Banana 2", "Kie.AI — Nano Banana Pro"] {
            let mut v = serde_json::json!({});
            kie_apply_size(&mut v, spec_de(etiqueta), "");
            assert_eq!(v["resolution"], "1K", "{etiqueta}: «Auto» debe ser 1K, no omitir");
            assert_eq!(v["output_format"], "png", "{etiqueta}");
        }
    }

    /// Dentro de la misma familia los modelos no coinciden: Nano Banana 2 y
    /// Pro usan `image_input` y llevan `resolution` y `output_format`;
    /// **Nano Banana 2 Lite usa `image_urls` y sólo `aspect_ratio`**. Es el
    /// mejor argumento a favor de declarar cada modelo por separado en vez de
    /// agrupar por nombre.
    #[test]
    fn nano_banana_2_lite_no_es_como_sus_hermanos() {
        let lite = spec_de("Kie.AI — Nano Banana 2 Lite");
        let dos = spec_de("Kie.AI — Nano Banana 2");

        assert_eq!(lite.ref_field, RefField::KieImageUrls);
        assert_eq!(dos.ref_field, RefField::KieImageInput);

        let mut v = serde_json::json!({});
        kie_apply_size(&mut v, lite, "2k");
        assert_eq!(v["aspect_ratio"], "auto");
        assert!(
            v.get("resolution").is_none(),
            "Lite no acepta `resolution`: su ejemplo oficial no lo incluye"
        );
    }

    #[test]
    fn una_resolucion_explicita_llega_con_el_nombre_de_cada_familia() {
        let mut v = serde_json::json!({});
        kie_apply_size(&mut v, spec_de("Kie.AI — Nano Banana Pro"), "2k");
        assert_eq!(v["resolution"], "2K");

        let mut v = serde_json::json!({});
        kie_apply_size(&mut v, spec_de("Kie.AI — Seedream 4.0"), "4k");
        assert_eq!(v["image_resolution"], "4K");
        assert_eq!(v["max_images"], 1);

        // Qwen 3 no llega a 4K: se limita en vez de ser rechazado.
        let mut v = serde_json::json!({});
        kie_apply_size(&mut v, spec_de("Kie.AI — Qwen Image 3.0"), "4k");
        assert_eq!(v["resolution"], "2K", "Qwen 3 sólo admite 1K o 2K");
    }

    /// `image_size` significa cosas distintas según el modelo: preajuste en
    /// Seedream 4.0, relación de aspecto en Qwen 3.
    #[test]
    fn image_size_significa_cosas_distintas_segun_el_modelo() {
        let mut a = serde_json::json!({});
        kie_apply_size(&mut a, spec_de("Kie.AI — Seedream 4.0"), "1k");
        let mut b = serde_json::json!({});
        kie_apply_size(&mut b, spec_de("Kie.AI — Qwen Image 3.0"), "1k");
        assert_eq!(a["image_size"], "square_hd");
        assert_eq!(b["image_size"], "1:1");
    }

    #[test]
    fn el_tamano_de_wavespeed_respeta_la_resolucion_elegida() {
        let spec = &crate::models::CATALOG[0];
        assert_eq!(wavespeed_size(spec, "1k"), "1024*1024");
        assert_eq!(wavespeed_size(spec, "2k"), "2048*2048");
        assert_eq!(wavespeed_size(spec, "4k"), "4096*4096");
    }

    #[test]
    fn extract_error_tolera_cuerpos_no_json_y_multibyte() {
        assert_eq!(
            extract_error(r#"{"error":{"message":"clave inválida"}}"#),
            "clave inválida"
        );
        // No debe entrar en pánico recortando un cuerpo con multibyte.
        let cuerpo = "ñ".repeat(500);
        let _ = extract_error(&cuerpo);
    }

    #[test]
    fn cada_proveedor_usa_su_campo_de_clave() {
        assert_eq!(ImageProvider::WaveSpeed.key_slot(), KeySlot::WaveSpeed);
        assert_eq!(ImageProvider::KieAi.key_slot(), KeySlot::KieAi);
        assert_eq!(ImageProvider::Xai.key_slot(), KeySlot::General);
        assert_eq!(ImageProvider::OpenAi.key_slot(), KeySlot::General);
        assert_eq!(ImageProvider::Google.key_slot(), KeySlot::General);
    }

    // ── Comprobaciones de compatibilidad ─────────────────────────────────
    //
    // Se ejecutan antes de cualquier llamada de red, así que estos tests
    // verifican el enrutado sin tocar Internet.

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    fn spec_por_etiqueta(label: &str) -> &'static ModelSpec {
        crate::models::CATALOG
            .iter()
            .find(|m| m.label == label)
            .unwrap_or_else(|| panic!("no existe el modelo «{}»", label))
    }

    /// Reportero de progreso mudo para los tests.
    fn sin_progreso() -> ProgressFn {
        std::sync::Arc::new(|_| {})
    }

    fn una_referencia() -> Vec<RefImage> {
        vec![("aGVsbG8=".to_string(), "image/png".to_string())]
    }

    /// H-04: un modelo texto→imagen con imagen cargada debe explicar el
    /// problema, no construir un endpoint inventado y devolver un 400.
    #[test]
    fn un_modelo_solo_t2i_rechaza_las_referencias_con_un_mensaje_util() {
        let spec = spec_por_etiqueta("WaveSpeed — Flux 2 Max");
        assert!(!spec.supports_i2i());

        let dir = std::env::temp_dir().join("big-test-t2i");
        let err = rt()
            .block_on(generate_image(
                spec,
                "clave-de-prueba",
                "un prompt",
                dir.to_str().unwrap(),
                &una_referencia(),
                I2iMode::StyleReference,
                "",
                &sin_progreso(),
            ))
            .unwrap_err();

        assert!(err.contains("texto→imagen"), "mensaje poco claro: {err}");
        assert!(err.contains("Flux Kontext"), "no sugiere alternativas: {err}");
    }

    /// Un endpoint de edición sin imagen debe avisar en vez de enviarse.
    #[test]
    fn un_modelo_solo_de_edicion_exige_imagen() {
        let spec = spec_por_etiqueta("Kie.AI — Seedream 4.0 Edit");
        assert!(!spec.supports_t2i());

        let dir = std::env::temp_dir().join("big-test-edit");
        let err = rt()
            .block_on(generate_image(
                spec,
                "clave-de-prueba",
                "un prompt",
                dir.to_str().unwrap(),
                &[],
                I2iMode::DirectEdit,
                "",
                &sin_progreso(),
            ))
            .unwrap_err();

        assert!(err.contains("necesita al menos una imagen"), "mensaje: {err}");
    }

    #[test]
    fn sin_clave_o_sin_prompt_se_avisa_antes_de_salir_a_la_red() {
        let spec = &crate::models::CATALOG[0];
        let dir = std::env::temp_dir().join("big-test-vacios");

        let err = rt()
            .block_on(generate_image(spec, "   ", "hola", dir.to_str().unwrap(), &[], I2iMode::StyleReference, "", &sin_progreso()))
            .unwrap_err();
        assert!(err.contains("Falta la API key"), "mensaje: {err}");

        let err = rt()
            .block_on(generate_image(spec, "k", "   ", dir.to_str().unwrap(), &[], I2iMode::StyleReference, "", &sin_progreso()))
            .unwrap_err();
        assert!(err.contains("prompt está vacío"), "mensaje: {err}");
    }

    /// H-05: en edición se envía el modelo elegido, no una constante fija.
    #[test]
    fn xai_conserva_el_modelo_elegido_en_edicion() {
        let pro = spec_por_etiqueta("xAI — Grok Imagine Image Pro");
        assert_eq!(pro.i2i_id, Some("grok-imagine-image-pro"));
        assert_eq!(pro.wire_id(true), Some("grok-imagine-image-pro"));

        let base = spec_por_etiqueta("xAI — Grok Imagine Image");
        assert_ne!(
            base.i2i_id, pro.i2i_id,
            "los dos modelos de xAI deben enviar identificadores distintos"
        );
    }

    /// Cada familia de Kie.AI usa un nombre distinto para el mismo concepto.
    #[test]
    fn cada_modelo_de_kie_declara_su_propio_campo_de_imagen() {
        assert_eq!(
            spec_por_etiqueta("Kie.AI — GPT Image 2").ref_field,
            RefField::KieInputUrls
        );
        assert_eq!(
            spec_por_etiqueta("Kie.AI — Seedream 4.0 Edit").ref_field,
            RefField::KieImageUrls
        );
        assert_eq!(
            spec_por_etiqueta("Kie.AI — Nano Banana Pro").ref_field,
            RefField::KieImageInput
        );
    }

    /// Kie.AI GPT Image 2 cambia de identificador entre T2I e I2I.
    #[test]
    fn kie_gpt_image_2_usa_endpoints_distintos_por_modo() {
        let spec = spec_por_etiqueta("Kie.AI — GPT Image 2");
        assert_eq!(spec.wire_id(false), Some("gpt-image-2-text-to-image"));
        assert_eq!(spec.wire_id(true), Some("gpt-image-2-image-to-image"));
    }
}
