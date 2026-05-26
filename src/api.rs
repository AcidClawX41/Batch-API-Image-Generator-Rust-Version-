//! api.rs — Multi-provider image generation client (v2.4.0).
//!
//! Supports:
//!   • OpenAI-compatible text-to-image (xAI, Google, OpenAI) — single POST, b64_json.
//!   • xAI Grok Imagine /v1/images/edits — JSON body with `images` array (up to 2 refs).
//!   • OpenAI /v1/images/edits — multipart form for image-to-image editing.
//!   • WaveSpeed.ai text-to-image — POST submit (sync mode), download URL.
//!   • WaveSpeed.ai image-to-image — dynamic field routing per model family.
//!
//! I2I compatibility matrix:
//!   xAI grok-imagine-image / quality ✅  (/v1/images/edits JSON, up to 2 images)
//!   WaveSpeed Flux Kontext Max/Pro/Dev ✅  (`image` singular field)
//!   WaveSpeed Flux Kontext Max/Pro Multi ✅  (`images` array, up to 5)
//!   WaveSpeed WAN 2.x                ✅  (`images` array, /image-to-image endpoint)
//!   WaveSpeed UNO                    ✅  (`images` array)
//!   WaveSpeed Flux 2 / Kling / etc   ❌  (T2I only)
//!   OpenAI gpt-image-1 / gpt-image-1.5  ✅  (/v1/images/edits multipart, 1 image)
//!   OpenAI dall-e-3                  ❌  (edits endpoint not supported)
//!   Google                           ❌  (no I2I API via this endpoint)

use serde::{Deserialize, Serialize};

// ─── Provider enum ───────────────────────────────────────────────────────────

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
            Self::KieAi => "Kie.ai",
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

// ─── xAI Grok Imagine image-edits types ──────────────────────────────────────

/// One entry in the `images` array for POST /v1/images/edits.
#[derive(Serialize)]
struct XaiImageItem {
    #[serde(rename = "type")]
    item_type: String,  // always "image_url"
    url: String,        // public URL or "data:<mime>;base64,<b64>"
}

/// Body for xAI /v1/images/edits (JSON, not multipart).
#[derive(Serialize)]
struct XaiEditsRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    images: Vec<XaiImageItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    aspect_ratio: Option<&'a str>,
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

#[derive(Deserialize)]
struct WaveSpeedResponse {
    #[allow(dead_code)]
    code: Option<i32>,
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

fn wavespeed_supports_i2i(_model: &str) -> bool {
    true
}

fn openai_supports_i2i(model: &str) -> bool {
    model.starts_with("gpt-image-")
}

fn xai_supports_i2i(model: &str) -> bool {
    model.starts_with("grok-imagine")
}

fn xai_edit_model(_model: &str) -> &'static str {
    "grok-imagine-image-quality"
}

/// Derive MIME type from file extension (lowercase).
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
/// * `ref_images` — Slice of (base64_string, mime_string) pairs.
///   Index 0 = persona/primary, 1 = escena/secondary, 2-4 = extra refs.
///   - xAI Grok Imagine uses up to 2.
///   - WaveSpeed Flux Kontext single uses 1 (`image` field).
///   - WaveSpeed Flux Kontext Multi / UNO / WAN use all provided (`images` array).
///   - OpenAI uses only the first.
///   Pass an empty slice for text-to-image mode.
/// * `i2i_mode` — `StyleReference` or `DirectEdit`; ignored in T2I mode.
pub async fn generate_image(
    provider: ImageProvider,
    api_key: &str,
    prompt: &str,
    model: &str,
    output_dir: &str,
    ref_images: &[(String, String)],
    i2i_mode: I2iMode,
    output_resolution: &str,
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

    let has_refs = !ref_images.is_empty();

    match (provider, has_refs) {
        // ── WaveSpeed with reference image(s) ──────────────────────────────
        (ImageProvider::WaveSpeed, true) => {
            if !wavespeed_supports_i2i(model) {
                return Err(format!(
                    "❌ El modelo «{}» es texto→imagen puro y no soporta imagen de referencia.\n\
                     Para Image-to-Image en WaveSpeed usa:\n\
                     • Flux Kontext Max/Pro/Dev      (edición precisa, 1 img)\n\
                     • Flux Kontext Max/Pro Multi    (hasta 5 imgs)\n\
                     • WAN 2.2 / WAN 2.6             (imagen→imagen)\n\
                     • UNO                           (multi-referencia)",
                    model
                ));
            }
            generate_wavespeed_i2i(api_key, prompt, model, output_dir, ref_images, i2i_mode, output_resolution).await
        }
        // ── WaveSpeed text-to-image ─────────────────────────────────────────
        (ImageProvider::WaveSpeed, false) => {
            generate_wavespeed(api_key, prompt, model, output_dir, output_resolution).await
        }
        // ── OpenAI with reference image (uses first only) ───────────────────
        (ImageProvider::OpenAi, true) => {
            if !openai_supports_i2i(model) {
                return Err(format!(
                    "❌ El modelo «{}» no soporta edición de imagen.\n\
                     Para Image-to-Image en OpenAI usa: gpt-image-1 o gpt-image-1.5",
                    model
                ));
            }
            let (b64, mime) = &ref_images[0];
            generate_openai_edit(api_key, prompt, model, output_dir, b64, mime).await
        }
        // ── xAI with reference image(s) → Grok Imagine /v1/images/edits ───
        (ImageProvider::Xai, true) => {
            if !xai_supports_i2i(model) {
                return Err(format!(
                    "❌ El modelo «{}» no soporta edición de imagen.\n\
                     Para Image-to-Image en xAI usa: grok-imagine-image o grok-imagine-image-quality",
                    model
                ));
            }
            let (b64_1, mime_1) = &ref_images[0];
            let img2 = ref_images.get(1);
            generate_xai_edit(
                api_key, prompt, model, output_dir,
                b64_1, mime_1,
                img2.map(|(b, _)| b.as_str()),
                img2.map(|(_, m)| m.as_str()).unwrap_or("image/png"),
            ).await
        }
        // ── Google with reference image (not supported) ─────────────────────
        (ImageProvider::Google, true) => {
            Err("❌ Google Gemini Image no soporta Image-to-Image por esta API.\n\
                 Usa xAI Grok Imagine, WaveSpeed (Flux Kontext) o OpenAI (gpt-image-1)."
                .to_string())
        }
        // ── Kie.ai with reference image (not supported) ─────────────────────
        (ImageProvider::KieAi, true) => {
            Err("❌ Kie.ai no soporta Image-to-Image.\n\
                 Usa xAI Grok Imagine, WaveSpeed (Flux Kontext) o OpenAI (gpt-image-1)."
                .to_string())
        }
        // ── All other providers: standard text-to-image ─────────────────────
        (_, false) => {
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
    let image_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        ref_image_b64,
    )
    .map_err(|e| format!("Error decodificando imagen de referencia: {}", e))?;

    let client = reqwest::Client::new();

    let fname = match ref_mime {
        "image/jpeg" => "reference.jpg",
        "image/webp" => "reference.webp",
        "image/gif"  => "reference.gif",
        _            => "reference.png",
    };

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

// ─── xAI Grok Imagine /v1/images/edits (image-to-image, 1–2 refs) ────────────

async fn generate_xai_edit(
    api_key: &str,
    prompt: &str,
    model: &str,
    output_dir: &str,
    ref_image_b64: &str,
    ref_mime: &str,
    ref_image2_b64: Option<&str>,
    ref_mime2: &str,
) -> Result<GenerationResult, String> {
    let client = reqwest::Client::new();
    let edit_model = xai_edit_model(model);

    let data_uri1 = format!("data:{};base64,{}", ref_mime, ref_image_b64);
    let mut images: Vec<XaiImageItem> = vec![
        XaiImageItem { item_type: "image_url".to_string(), url: data_uri1 },
    ];

    if let Some(b64_2) = ref_image2_b64 {
        let data_uri2 = format!("data:{};base64,{}", ref_mime2, b64_2);
        images.push(XaiImageItem { item_type: "image_url".to_string(), url: data_uri2 });
    }

    let num_images = images.len();
    let request = XaiEditsRequest {
        model: edit_model,
        prompt,
        images,
        aspect_ratio: Some("16:9"),
    };

    let resp = client
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
        let msg = serde_json::from_str::<ErrorResponse>(&body)
            .ok()
            .and_then(|e| e.error)
            .and_then(|e| e.message)
            .unwrap_or(body);
        return Err(format!(
            "xAI Grok Imagine edits ({} ref{}) devolvió HTTP {}: {}",
            num_images,
            if num_images == 1 { "" } else { "s" },
            status.as_u16(),
            msg
        ));
    }

    let data: OpenAiResponse = resp.json().await.map_err(|e| {
        format!("Error parseando respuesta de xAI Grok edits: {}", e)
    })?;

    let images_resp = data.data.ok_or("xAI Grok no devolvió imágenes.")?;
    let first = images_resp.first().ok_or("Lista de imágenes vacía.")?;

    if let Some(b64) = first.b64_json.as_deref() {
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
            .map_err(|e| format!("Error decodificando base64 de xAI edits: {}", e))?;
        save_image(ImageProvider::Xai, &bytes, output_dir, "png")
    } else if let Some(url) = first.url.as_deref() {
        download_and_save_for(ImageProvider::Xai, url, output_dir).await
    } else {
        Err("Sin datos base64 ni URL en la respuesta de xAI Grok edits.".to_string())
    }
}

// ─── WaveSpeed text-to-image flow ────────────────────────────────────────────

async fn generate_wavespeed(
    api_key: &str,
    prompt: &str,
    model: &str,
    output_dir: &str,
    output_resolution: &str,
) -> Result<GenerationResult, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/{}", WAVESPEED_BASE, model);

    // GPT Image 2 T2I uses a different schema: no `size`, just prompt + aspect_ratio
    let body_str;
    let body_json;
    let resp = if model.contains("gpt-image-2") {
        body_json = serde_json::json!({
            "prompt": prompt,
            "seed": -1_i64,
            "enable_sync_mode": true,
        });
        client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body_json)
            .timeout(std::time::Duration::from_secs(180))
            .send()
            .await
            .map_err(|e| format_reqwest_error(ImageProvider::WaveSpeed, e))?
    } else {
        let size = if !output_resolution.is_empty() {
            resolution_to_size(output_resolution)
        } else {
            wavespeed_default_size(model)
        };
        let request = WaveSpeedRequest {
            prompt,
            size: Some(size),
            seed: -1,
            enable_sync_mode: true,
        };
        client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .timeout(std::time::Duration::from_secs(180))
            .send()
            .await
            .map_err(|e| format_reqwest_error(ImageProvider::WaveSpeed, e))?
    };

    let status = resp.status();
    body_str = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        let msg = serde_json::from_str::<WaveSpeedResponse>(&body_str)
            .ok()
            .and_then(|r| r.message)
            .unwrap_or(body_str);
        return Err(format!("WaveSpeed devolvió HTTP {}: {}", status.as_u16(), msg));
    }

    handle_wavespeed_response(&client, api_key, &body_str, output_dir).await
}

// ─── WaveSpeed image-to-image flow ───────────────────────────────────────────
//
// Field routing per model family:
//
//   Flux Kontext single (max/pro/dev)  → `image`  (singular data URI)
//   Flux Kontext /multi                → `images` (array, up to 5)
//   WAN family                         → `images` (array, /image-to-image endpoint)
//   UNO                                → `images` (array)
//   /edit, /image-edit endpoints       → `images` (array)
//   /image-to-image endpoints          → `images` (array)
//   Everything else                    → `image`  (singular)

async fn generate_wavespeed_i2i(
    api_key: &str,
    prompt: &str,
    model: &str,
    output_dir: &str,
    ref_images: &[(String, String)],
    i2i_mode: I2iMode,
    output_resolution: &str,
) -> Result<GenerationResult, String> {
    let client = reqwest::Client::new();

    let base_model = model.strip_suffix("/text-to-image").unwrap_or(model);

    // ── Derive the actual I2I endpoint ──────────────────────────────────────
    let actual_model = if base_model.ends_with("/image-to-image")
        || base_model.ends_with("/image-edit")
        || base_model.ends_with("/multi")
        || base_model.ends_with("/edit")
        || base_model.ends_with("/edit-fast")
    {
        // Already an I2I / edit / multi endpoint — use as-is.
        // e.g. wan-2.2/image-to-image, flux-kontext-max/multi,
        //      nano-banana-2/edit, seedream-v5.0-lite/edit, wan-2.7/image-edit
        base_model.to_string()
    } else if base_model.contains("wan-2.6") {
        // WAN 2.6 has NO I2I endpoint — tell the user to use WAN 2.7
        return Err(format!(
            "El modelo 'alibaba/wan-2.6' no tiene endpoint Image-to-Image.              Usa 'alibaba/wan-2.7/image-edit' para I2I."
        ));
    } else if base_model.contains("wan") {
        // WAN 2.7+ text-to-image base → I2I endpoint
        format!("{}/image-to-image", base_model)
    } else if base_model.contains("flux-kontext") || base_model.contains("uno") {
        // Flux Kontext and UNO are natively editing models — no suffix needed
        base_model.to_string()
    } else if base_model.contains("flux") {
        format!("{}/image-to-image", base_model)
    } else {
        format!("{}/edit", base_model)
    };

    let url = format!("{}/{}", WAVESPEED_BASE, actual_model);
    let size = if !output_resolution.is_empty() {
        resolution_to_size(output_resolution)
    } else {
        wavespeed_default_size(model)
    };

    // Build data URIs for all reference images
    let data_uris: Vec<String> = ref_images
        .iter()
        .map(|(b64, mime)| format!("data:{};base64,{}", mime, b64))
        .collect();

    let strength = match i2i_mode {
        I2iMode::DirectEdit     => 0.85_f32,
        I2iMode::StyleReference => 0.55_f32,
    };

    // ── Field routing ───────────────────────────────────────────────────────
    // Flux Kontext single: `image` (singular)
    // Everything with /multi, /wan, /edit, /image-edit, /image-to-image, or UNO: `images` (array)
    let is_kontext_single = actual_model.contains("kontext") && !actual_model.ends_with("/multi");

    // Grok Edit uses xAI's images/edits format → `image` singular (not an array)
    let is_grok_edit = actual_model.contains("grok");

    let needs_images_array = !is_kontext_single
        && !is_grok_edit
        && (actual_model.ends_with("/multi")
            || actual_model.contains("/wan")
            || actual_model.contains("uno")
            || actual_model.ends_with("/edit")
            || actual_model.ends_with("/edit-fast")   // ← was missing
            || actual_model.ends_with("/image-edit")
            || actual_model.ends_with("/image-to-image"));

    // ── Build JSON body dynamically ─────────────────────────────────────────
    let mut body = serde_json::json!({
        "prompt": prompt,
        "size": size,
        "seed": -1_i64,
        "enable_sync_mode": true,
        "strength": strength,
    });

    // GPT Image 2 Edit uses `resolution` (1k/2k/4k) instead of (or in addition to) `size`
    if actual_model.contains("gpt-image-2") && !output_resolution.is_empty() {
        body["resolution"] = serde_json::Value::String(output_resolution.to_string());
    }

    if is_kontext_single || is_grok_edit {
        // `image`: single data URI (first reference only)
        // - Flux Kontext single: expects `image` not `images`
        // - Grok Edit: mirrors xAI images/edits API, expects `image` singular
        if let Some(uri) = data_uris.first() {
            body["image"] = serde_json::Value::String(uri.clone());
        }
    } else if needs_images_array {
        // `images`: array of data URIs (all references, up to model's limit)
        body["images"] = serde_json::json!(data_uris);
    } else {
        // Fallback: treat as `image` singular
        if let Some(uri) = data_uris.first() {
            body["image"] = serde_json::Value::String(uri.clone());
        }
    }

    let resp = client
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
            .unwrap_or_else(|| body_str.clone());

        if status.as_u16() == 400 && msg.contains("model not found") {
            return Err(format!(
                "❌ WaveSpeed no encuentra el modelo I2I '{}'.\n\
                 Usa Flux Kontext (Max/Pro/Dev) o WAN 2.2 para Image-to-Image.",
                actual_model
            ));
        }

        return Err(format!(
            "WaveSpeed I2I devolvió HTTP {}: {}",
            status.as_u16(),
            msg
        ));
    }

    handle_wavespeed_response(&client, api_key, &body_str, output_dir).await
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

    if data.status.as_deref() == Some("completed") {
        let outputs = data.outputs.ok_or("WaveSpeed completó pero sin outputs.")?;
        let image_url = outputs.first().ok_or("Lista de outputs vacía.")?;
        return download_and_save_for(ImageProvider::WaveSpeed, image_url, output_dir).await;
    }

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
    let max_polls = 180;

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

fn wavespeed_default_size(model: &str) -> &'static str {
    if model.contains("seedream") || model.contains("dreamina") {
        "1920*1920"
    } else {
        "1024*1024"
    }
}

/// Maps a user-selected resolution string to a WaveSpeed `size` field value.
fn resolution_to_size(res: &str) -> &'static str {
    match res {
        "1k" => "1024*1024",
        "2k" => "2048*2048",
        "4k" => "4096*4096",
        _    => "1024*1024",
    }
}

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
