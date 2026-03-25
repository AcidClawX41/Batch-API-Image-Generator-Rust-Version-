//! api.rs — api.rs — Multi-provider image generation client (v2).
//!
//! Supports two API flavours:
//!   • OpenAI-compatible (xAI, Google, OpenAI) — single POST, b64_json response.
//!   • WaveSpeed.ai — POST to submit, then poll GET until completed, download URL.

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

// ─── OpenAI-compatible types ─────────────────────────────────────────────────

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

// ─── Main entry point ────────────────────────────────────────────────────────

/// Generate an image and save it to disk.
/// Routes to the appropriate backend based on the provider.
pub async fn generate_image(
    provider: ImageProvider,
    api_key: &str,
    prompt: &str,
    model: &str,
    output_dir: &str,
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

    match provider {
        ImageProvider::WaveSpeed => {
            generate_wavespeed(api_key, prompt, model, output_dir).await
        }
        _ => {
            generate_openai_compat(provider, api_key, prompt, model, output_dir).await
        }
    }
}

// ─── OpenAI-compatible flow ──────────────────────────────────────────────────

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
    let b64 = first.b64_json.as_deref().ok_or("Sin datos base64.")?;

    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
        .map_err(|e| format!("Error decodificando base64: {}", e))?;

    save_image(provider, &bytes, output_dir, "png")
}

// ─── WaveSpeed flow (sync mode + URL download) ──────────────────────────────

async fn generate_wavespeed(
    api_key: &str,
    prompt: &str,
    model: &str,
    output_dir: &str,
) -> Result<GenerationResult, String> {
    let client = reqwest::Client::new();

    // Build the endpoint URL: WAVESPEED_BASE / {model}
    // model is the full slug, e.g. "wavespeed-ai/flux-2-max/text-to-image"
    let url = format!("{}/{}", WAVESPEED_BASE, model);

    // Some models (Seedream, Dreamina) require minimum 3686400 pixels (≈1920x1920).
    // Others work fine at 1024x1024.  Pick the right default.
    let size = if model.contains("seedream") || model.contains("dreamina") {
        "1920*1920"
    } else {
        "1024*1024"
    };

    let request = WaveSpeedRequest {
        prompt,
        size: Some(size),
        seed: -1,
        enable_sync_mode: true, // wait for completion in one call
    };

    // WaveSpeed sync mode can take a while (up to ~120s for some models)
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
        // Try to parse structured error
        let msg = serde_json::from_str::<WaveSpeedResponse>(&body)
            .ok()
            .and_then(|r| r.message)
            .unwrap_or(body);
        return Err(format!("WaveSpeed devolvió HTTP {}: {}", status.as_u16(), msg));
    }

    let ws_resp: WaveSpeedResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Error parseando respuesta de WaveSpeed: {} — body: {}", e, &body[..body.len().min(200)]))?;

    // Check for API-level errors
    if let Some(ref data) = ws_resp.data {
        if let Some(ref err) = data.error {
            if !err.is_empty() {
                return Err(format!("WaveSpeed error: {}", err));
            }
        }
        if data.status.as_deref() == Some("failed") {
            let msg = data.error.as_deref().unwrap_or("Generación fallida sin detalles.");
            return Err(format!("WaveSpeed falló: {}", msg));
        }
    }

    // If sync mode returned completed, extract the output URL
    let data = ws_resp.data.ok_or("WaveSpeed no devolvió datos.")?;

    // If status is not completed yet (shouldn't happen in sync mode, but just in case),
    // fall back to polling.
    if data.status.as_deref() != Some("completed") {
        if let Some(task_id) = data.id.as_deref() {
            return poll_wavespeed(&client, api_key, task_id, output_dir).await;
        }
        return Err(format!(
            "WaveSpeed devolvió estado inesperado: {:?}",
            data.status
        ));
    }

    let outputs = data.outputs.ok_or("WaveSpeed completó pero sin outputs.")?;
    let image_url = outputs.first().ok_or("Lista de outputs vacía.")?;

    download_and_save(image_url, output_dir).await
}

/// Poll WaveSpeed until the task completes (fallback if sync mode doesn't
/// return immediately).
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
            continue; // Retry on transient errors
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
                            return download_and_save(url, output_dir).await;
                        }
                    }
                    return Err("WaveSpeed completó pero sin URLs de output.".to_string());
                }
                Some("failed") => {
                    let msg = data.error.as_deref().unwrap_or("Sin detalles.");
                    return Err(format!("WaveSpeed falló: {}", msg));
                }
                _ => continue, // Still processing
            }
        }
    }

    Err("WaveSpeed: timeout esperando resultado (>3 min).".to_string())
}

/// Download an image from a URL and save it to disk.
async fn download_and_save(
    url: &str,
    output_dir: &str,
) -> Result<GenerationResult, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| format!("Error descargando imagen de WaveSpeed: {}", e))?;

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

    save_image(ImageProvider::WaveSpeed, &bytes, output_dir, "png")
}

// ─── Shared helpers ──────────────────────────────────────────────────────────

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
