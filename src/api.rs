//! api.rs — Multi-provider image generation client (v2.2.0).
//!
//! Supports:
//!   • OpenAI-compatible text-to-image (xAI, Google, OpenAI) — single POST, b64_json.
//!   • OpenAI /v1/images/edits — multipart form for image-to-image editing.
//!   • WaveSpeed.ai text-to-image — POST submit (sync mode), download URL.
//!   • WaveSpeed.ai image-to-image — Flux Kontext only; `image` field as data URI.
//!
//! I2I compatibility matrix:
//!   WaveSpeed Flux Kontext Max/Pro  ✅  (image editing by design)
//!   WaveSpeed Flux 2 / Kling / etc  ❌  (T2I only, silently ignore image field)
//!   OpenAI gpt-image-1 / gpt-image-1.5  ✅  (/v1/images/edits multipart)
//!   OpenAI dall-e-3                 ❌  (edits endpoint not supported)
//!   xAI / Google                    ❌  (no I2I API)

use serde::{Deserialize, Serialize};

// ─── Provider enum ───────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageProvider {
    Xai,
    Google,
    OpenAi,
    WaveSpeed,
}

impl ImageProvider {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Xai => "xAI",
            Self::Google => "Google",
            Self::OpenAi => "OpenAI",
            Self::WaveSpeed => "WaveSpeed",
        }
    }

    fn file_prefix(self) -> &'static str {
        match self {
            Self::Xai => "xai",
            Self::Google => "google",
            Self::OpenAi => "openai",
            Self::WaveSpeed => "wavespeed",
        }
    }
}

// ─── I2I mode ────────────────────────────────────────────────────────────────

/// How the reference image should be used.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum I2iMode {
    /// Use the reference image as a visual style guide; the prompt drives content.
    StyleReference,
    /// Directly edit / transform the reference image following the prompt.
    DirectEdit,
}

// ─── OpenAI-compatible text-to-image types ───────────────────────────────────

fn openai_api_url(provider: ImageProvider) -> &'static str {
    match provider {
        ImageProvider::Xai => "https://api.x.ai/v1/images/generations",
        ImageProvider::Google => {
            "https://generativelanguage.googleapis.com/v1beta/openai/images/generations"
        }
        ImageProvider::OpenAi => "https://api.openai.com/v1/images/generations",
        _ => unreachable!(),
    }
}

fn openai_response_format(provider: ImageProvider, model: &str) -> Option<&'static str> {
    match provider {
        ImageProvider::OpenAi if model.starts_with("gpt-image-") => None,
        _ => Some("b64_json"),
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

#[derive(Deserialize)]
struct ErrorResponse {
    error: Option<ErrorDetail>,
}

#[derive(Deserialize)]
struct ErrorDetail {
    message: Option<String>,
}

// ─── WaveSpeed types ─────────────────────────────────────────────────────────

const WAVESPEED_BASE: &str = "https://api.wavespeed.ai/api/v3";

/// Text-to-image request (no reference image).
#[derive(Serialize)]
struct WaveSpeedRequest<'a> {
    prompt: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<&'a str>,
    seed: i64,
    enable_sync_mode: bool,
}

/// Image-to-image request (includes `image` or `images` as data URI).
#[derive(Serialize)]
struct WaveSpeedI2IRequest<'a> {
    prompt: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    images: Option<Vec<&'a str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<&'a str>,
    seed: i64,
    enable_sync_mode: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    strength: Option<f32>,
}

#[derive(Deserialize)]
struct WaveSpeedResponse {
    #[allow(dead_code)]
    code: Option<i32>,      // HTTP-level status code in the body; not used directly
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

// ─── Public result type ──────────────────────────────────────────────────────

/// Result of a generation attempt.
pub struct GenerationResult {
    pub filepath: String,
    pub filename: String,
}

// ─── I2I model compatibility ─────────────────────────────────────────────────

/// Returns true only for WaveSpeed models that genuinely support image conditioning.
/// All other WaveSpeed endpoints are text-to-image only and silently ignore `image`.
///
/// Confirmed I2I support:
///   • flux-kontext — designed for image editing (Flux Kontext Max/Pro)
///   • nano-banana  — Google Imagen via WaveSpeed; accepts image conditioning
/// Returns true for OpenAI models that support /v1/images/edits.
// Permitimos que todos los modelos intenten la generación con imagen de referencia,
// en lugar de bloquear artificialmente en el cliente. Si un modelo no existe o falla,
// la API de WaveSpeed devolverá el error apropiadamente.
fn wavespeed_supports_i2i(_model: &str) -> bool {
    true
}

/// Returns true for OpenAI models that support /v1/images/edits.
fn openai_supports_i2i(model: &str) -> bool {
    model.starts_with("gpt-image-")
}

/// Derive MIME type from file extension (lowercase).
/// Falls back to "image/png" for unknown types.
pub fn mime_from_ext(ext: &str) -> &'static str {
    match ext.to_ascii_lowercase().trim_start_matches('.') {
        "jpg" | "jpeg" => "image/jpeg",
        "webp"         => "image/webp",
        "gif"          => "image/gif",
        _              => "image/png",
    }
}

// ─── Main entry point ────────────────────────────────────────────────────────

/// Generate an image and save it to disk.
///
/// * `ref_image_b64` — raw Base64-encoded reference image bytes (no data-URI header).
///   Pass `None` for text-to-image mode.
/// * `ref_mime` — MIME type of the reference image (e.g. `"image/webp"`).
///   Ignored when `ref_image_b64` is `None`.
/// * `i2i_mode` — `StyleReference` or `DirectEdit`; ignored when `ref_image_b64` is `None`.
pub async fn generate_image(
    provider: ImageProvider,
    api_key: &str,
    prompt: &str,
    model: &str,
    output_dir: &str,
    ref_image_b64: Option<&str>,
    ref_mime: &str,
    i2i_mode: I2iMode,
) -> Result<GenerationResult, String> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err(format!("Falta la API key de {}.", provider.display_name()));
    }

    let prompt = prompt.trim();
    if prompt.is_empty() {
        return Err("El prompt está vacío.".to_string());
    }

    std::fs::create_dir_all(output_dir).map_err(|e| format!("Error creando carpeta: {}", e))?;

    match (provider, ref_image_b64) {
        // ── WaveSpeed with reference image ──────────────────────────────────
        (ImageProvider::WaveSpeed, Some(b64)) => {
            if !wavespeed_supports_i2i(model) {
                return Err(format!(
                    "❌ El modelo «{}» es texto→imagen puro y no soporta imagen de referencia.\n\
                     Para Image-to-Image en WaveSpeed usa:\n\
                     • Flux Kontext Max  (edición precisa)\n\
                     • Flux Kontext Pro  (edición precisa)\n\
                     • Nano Banana 2     (estilo + composición)\n\
                     • Nano Banana Pro   (estilo + composición)",
                    model
                ));
            }
            generate_wavespeed_i2i(api_key, prompt, model, output_dir, b64, ref_mime, i2i_mode).await
        }
        // ── WaveSpeed text-to-image ─────────────────────────────────────────
        (ImageProvider::WaveSpeed, None) => {
            generate_wavespeed(api_key, prompt, model, output_dir).await
        }
        // ── OpenAI with reference image ─────────────────────────────────────
        (ImageProvider::OpenAi, Some(b64)) => {
            if !openai_supports_i2i(model) {
                return Err(format!(
                    "❌ El modelo «{}» no soporta edición de imagen.\n\
                     Para Image-to-Image en OpenAI usa: gpt-image-1 o gpt-image-1.5",
                    model
                ));
            }
            generate_openai_edit(api_key, prompt, model, output_dir, b64, ref_mime).await
        }
        // ── xAI with reference image (not supported natively) ───────────────
        (ImageProvider::Xai, Some(_)) => {
            Err("❌ xAI no soporta Image-to-Image por API directa.\n\
                 Usa WaveSpeed (Flux Kontext) o OpenAI (gpt-image-1) para edición de imagen."
                .to_string())
        }
        // ── Google with reference image (not supported in this endpoint) ─────
        (ImageProvider::Google, Some(_)) => {
            Err("❌ Google Gemini Image no soporta Image-to-Image por esta API.\n\
                 Usa WaveSpeed (Flux Kontext) o OpenAI (gpt-image-1) para edición de imagen."
                .to_string())
        }
        // ── All other providers: standard text-to-image ─────────────────────
        (_, None) => {
            generate_openai_compat(provider, api_key, prompt, model, output_dir).await
        }
    }
}

// ─── OpenAI-compatible text-to-image flow ────────────────────────────────────

async fn generate_openai_compat(
    provider: ImageProvider,
    api_key: &str,
    prompt: &str,
    model: &str,
    output_dir: &str,
) -> Result<GenerationResult, String> {
    let client = reqwest::Client::new();
    let request = OpenAiRequest {
        model,
        prompt,
        n: 1,
        response_format: openai_response_format(provider, model),
    };

    let resp = client
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
        let msg = serde_json::from_str::<ErrorResponse>(&body)
            .ok()
            .and_then(|e| e.error)
            .and_then(|e| e.message)
            .unwrap_or(body);
        return Err(format!(
            "{} devolvió HTTP {}: {}",
            provider.display_name(),
            status.as_u16(),
            msg
        ));
    }

    let data: OpenAiResponse = resp.json().await.map_err(|e| {
        format!(
            "Error parseando respuesta de {}: {}",
            provider.display_name(),
            e
        )
    })?;

    let images = data
        .data
        .ok_or_else(|| format!("{} no devolvió imágenes.", provider.display_name()))?;
    let first = images.first().ok_or("Lista de imágenes vacía.")?;

    // Prefer b64_json, fall back to URL download
    if let Some(b64) = first.b64_json.as_deref() {
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
            .map_err(|e| format!("Error decodificando base64: {}", e))?;
        save_image(provider, &bytes, output_dir, "png")
    } else if let Some(url) = first.url.as_deref() {
        download_and_save_for(provider, url, output_dir).await
    } else {
        Err("Sin datos base64 ni URL en la respuesta.".to_string())
    }
}

// ─── OpenAI /v1/images/edits (image-to-image) ────────────────────────────────

async fn generate_openai_edit(
    api_key: &str,
    prompt: &str,
    model: &str,
    output_dir: &str,
    ref_image_b64: &str,
    ref_mime: &str,
) -> Result<GenerationResult, String> {
    // Decode b64 → raw bytes for the multipart upload
    let image_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        ref_image_b64,
    )
    .map_err(|e| format!("Error decodificando imagen de referencia: {}", e))?;

    let client = reqwest::Client::new();

    // Derive a sensible filename from the MIME type
    let fname = match ref_mime {
        "image/jpeg" => "reference.jpg",
        "image/webp" => "reference.webp",
        "image/gif"  => "reference.gif",
        _            => "reference.png",
    };

    // Build multipart/form-data body
    let image_part = reqwest::multipart::Part::bytes(image_bytes)
        .file_name(fname)
        .mime_str(ref_mime)
        .map_err(|e| format!("Error MIME: {}", e))?;

    let form = reqwest::multipart::Form::new()
        .text("model", model.to_string())
        .text("prompt", prompt.to_string())
        .text("n", "1")
        .part("image", image_part);

    let resp = client
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
        let msg = serde_json::from_str::<ErrorResponse>(&body)
            .ok()
            .and_then(|e| e.error)
            .and_then(|e| e.message)
            .unwrap_or(body);
        return Err(format!("OpenAI edits devolvió HTTP {}: {}", status.as_u16(), msg));
    }

    let data: OpenAiResponse = resp.json().await.map_err(|e| {
        format!("Error parseando respuesta de OpenAI edits: {}", e)
    })?;

    let images = data.data.ok_or("OpenAI no devolvió imágenes en edits.")?;
    let first = images.first().ok_or("Lista de imágenes vacía.")?;

    if let Some(b64) = first.b64_json.as_deref() {
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
            .map_err(|e| format!("Error decodificando base64 de edits: {}", e))?;
        save_image(ImageProvider::OpenAi, &bytes, output_dir, "png")
    } else if let Some(url) = first.url.as_deref() {
        download_and_save_for(ImageProvider::OpenAi, url, output_dir).await
    } else {
        Err("Sin datos base64 ni URL en la respuesta de edits.".to_string())
    }
}

// ─── WaveSpeed text-to-image flow ────────────────────────────────────────────

async fn generate_wavespeed(
    api_key: &str,
    prompt: &str,
    model: &str,
    output_dir: &str,
) -> Result<GenerationResult, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/{}", WAVESPEED_BASE, model);

    let size = wavespeed_default_size(model);

    let request = WaveSpeedRequest {
        prompt,
        size: Some(size),
        seed: -1,
        enable_sync_mode: true,
    };

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .timeout(std::time::Duration::from_secs(180))
        .send()
        .await
        .map_err(|e| format_reqwest_error(ImageProvider::WaveSpeed, e))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        let msg = serde_json::from_str::<WaveSpeedResponse>(&body)
            .ok()
            .and_then(|r| r.message)
            .unwrap_or(body);
        return Err(format!("WaveSpeed devolvió HTTP {}: {}", status.as_u16(), msg));
    }

    handle_wavespeed_response(&client, api_key, &body, output_dir).await
}

// ─── WaveSpeed image-to-image flow ───────────────────────────────────────────

async fn generate_wavespeed_i2i(
    api_key: &str,
    prompt: &str,
    model: &str,
    output_dir: &str,
    ref_image_b64: &str,
    ref_mime: &str,
    i2i_mode: I2iMode,
) -> Result<GenerationResult, String> {
    let client = reqwest::Client::new();
    
    // Cambiar la terminación del modelo para apuntar al endpoint de Image-to-Image / Edit
    let base_model = model.strip_suffix("/text-to-image").unwrap_or(model);
    
    let actual_model = if base_model.contains("wan") {
        format!("{}/image-edit", base_model)
    } else if base_model.contains("flux-kontext") {
        // Flux Kontext is natively an editing model, no suffix needed
        base_model.to_string()
    } else if base_model.contains("flux") {
        format!("{}/image-to-image", base_model)
    } else {
        // Modelos como Seedream, Nano-Banana y Qwen usan el endpoint /edit
        format!("{}/edit", base_model)
    };
    
    let url = format!("{}/{}", WAVESPEED_BASE, actual_model);

    let size = wavespeed_default_size(model);

    // Build the data URI with the actual MIME type of the uploaded file
    let data_uri = format!("data:{};base64,{}", ref_mime, ref_image_b64);

    // strength: how much the reference image influences the output.
    // DirectEdit → 0.85 (strong adherence to reference image structure)
    // StyleReference → 0.55 (looser, prompt drives content more)
    let strength = match i2i_mode {
        I2iMode::DirectEdit => 0.85_f32,
        I2iMode::StyleReference => 0.55_f32,
    };

    let (image_val, images_val) = if actual_model.ends_with("/edit") || actual_model.contains("kontext") {
        (None, Some(vec![data_uri.as_str()]))
    } else {
        (Some(data_uri.as_str()), None)
    };

    let request = WaveSpeedI2IRequest {
        prompt,
        image: image_val,
        images: images_val,
        size: Some(size),
        seed: -1,
        enable_sync_mode: true,
        strength: Some(strength),
    };

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .timeout(std::time::Duration::from_secs(180))
        .send()
        .await
        .map_err(|e| format_reqwest_error(ImageProvider::WaveSpeed, e))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        let msg = serde_json::from_str::<WaveSpeedResponse>(&body)
            .ok()
            .and_then(|r| r.message)
            .unwrap_or(body);
            
        if status.as_u16() == 400 && msg.contains("model not found") {
            return Err(format!(
                "❌ WaveSpeed no ofrece Endpoint Image-to-Image para '{}'.\nLa API no encuentra el modelo {}. (Si usas el base ignorará la imagen). Usa Flux Kontext o Wan.",
                base_model, actual_model
            ));
        }
            
        return Err(format!(
            "WaveSpeed I2I devolvió HTTP {}: {}",
            status.as_u16(),
            msg
        ));
    }

    handle_wavespeed_response(&client, api_key, &body, output_dir).await
}

// ─── WaveSpeed shared response handler ───────────────────────────────────────

async fn handle_wavespeed_response(
    client: &reqwest::Client,
    api_key: &str,
    body: &str,
    output_dir: &str,
) -> Result<GenerationResult, String> {
    let ws_resp: WaveSpeedResponse = serde_json::from_str(body).map_err(|e| {
        format!(
            "Error parseando respuesta de WaveSpeed: {} — body: {}",
            e,
            &body[..body.len().min(200)]
        )
    })?;

    // Check for API-level errors
    if let Some(ref data) = ws_resp.data {
        if let Some(ref err) = data.error {
            if !err.is_empty() {
                return Err(format!("WaveSpeed error: {}", err));
            }
        }
        if data.status.as_deref() == Some("failed") {
            let msg = data
                .error
                .as_deref()
                .unwrap_or("Generación fallida sin detalles.");
            return Err(format!("WaveSpeed falló: {}", msg));
        }
    }

    let data = ws_resp.data.ok_or("WaveSpeed no devolvió datos.")?;

    // Sync mode completed immediately
    if data.status.as_deref() == Some("completed") {
        let outputs = data.outputs.ok_or("WaveSpeed completó pero sin outputs.")?;
        let image_url = outputs.first().ok_or("Lista de outputs vacía.")?;
        return download_and_save_for(ImageProvider::WaveSpeed, image_url, output_dir).await;
    }

    // Not yet done → fall back to polling
    if let Some(task_id) = data.id.as_deref() {
        return poll_wavespeed(client, api_key, task_id, output_dir).await;
    }

    Err(format!(
        "WaveSpeed devolvió estado inesperado: {:?}",
        data.status
    ))
}

// ─── WaveSpeed polling fallback ───────────────────────────────────────────────

async fn poll_wavespeed(
    client: &reqwest::Client,
    api_key: &str,
    task_id: &str,
    output_dir: &str,
) -> Result<GenerationResult, String> {
    let poll_url = format!("{}/predictions/{}/result", WAVESPEED_BASE, task_id);
    let max_polls = 180; // ~3 minutes at 1s intervals

    for _ in 0..max_polls {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let resp = client
            .get(&poll_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Error polling WaveSpeed: {}", e))?;

        if !resp.status().is_success() {
            continue;
        }

        let body = resp.text().await.unwrap_or_default();
        let ws: WaveSpeedResponse = match serde_json::from_str(&body) {
            Ok(r) => r,
            Err(_) => continue,
        };

        if let Some(ref data) = ws.data {
            match data.status.as_deref() {
                Some("completed") => {
                    if let Some(ref outputs) = data.outputs {
                        if let Some(url) = outputs.first() {
                            return download_and_save_for(
                                ImageProvider::WaveSpeed,
                                url,
                                output_dir,
                            )
                            .await;
                        }
                    }
                    return Err("WaveSpeed completó pero sin URLs de output.".to_string());
                }
                Some("failed") => {
                    let msg = data.error.as_deref().unwrap_or("Sin detalles.");
                    return Err(format!("WaveSpeed falló: {}", msg));
                }
                _ => continue,
            }
        }
    }

    Err("WaveSpeed: timeout esperando resultado (>3 min).".to_string())
}

// ─── Shared helpers ───────────────────────────────────────────────────────────

/// Choose the right output resolution for WaveSpeed models.
fn wavespeed_default_size(model: &str) -> &'static str {
    // ByteDance models require ≥ 3 686 400 px (1920×1920 = 3 686 400)
    if model.contains("seedream") || model.contains("dreamina") {
        "1920*1920"
    } else {
        "1024*1024"
    }
}

/// Download an image from a CDN URL and save it to disk under the given provider prefix.
async fn download_and_save_for(
    provider: ImageProvider,
    url: &str,
    output_dir: &str,
) -> Result<GenerationResult, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| format!("Error descargando imagen: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!(
            "Error descargando imagen: HTTP {}",
            resp.status().as_u16()
        ));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Error leyendo bytes de imagen: {}", e))?;

    save_image(provider, &bytes, output_dir, "png")
}

fn save_image(
    provider: ImageProvider,
    bytes: &[u8],
    output_dir: &str,
    ext: &str,
) -> Result<GenerationResult, String> {
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("{}_{}.{}", provider.file_prefix(), ts, ext);
    let filepath = std::path::Path::new(output_dir).join(&filename);

    std::fs::write(&filepath, bytes).map_err(|e| format!("Error guardando: {}", e))?;

    Ok(GenerationResult {
        filepath: filepath.to_string_lossy().to_string(),
        filename,
    })
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
