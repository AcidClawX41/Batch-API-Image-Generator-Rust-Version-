//! api.rs — xAI Imagine API client.

use serde::{Deserialize, Serialize};

const API_URL: &str = "https://api.x.ai/v1/images/generations";

#[derive(Serialize)]
struct ImageRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    n: u32,
    response_format: &'a str,
}

#[derive(Deserialize)]
struct ImageResponse {
    data: Option<Vec<ImageData>>,
}

#[derive(Deserialize)]
struct ImageData {
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

/// Result of a generation attempt.
pub struct GenerationResult {
    pub filepath: String,
    pub filename: String,
}

/// Generate an image and save it to disk.
/// Returns the filepath on success, or an error message.
pub async fn generate_image(
    api_key: &str,
    prompt: &str,
    model: &str,
    output_dir: &str,
) -> Result<GenerationResult, String> {
    // Ensure output directory exists
    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Error creando carpeta: {}", e))?;

    let client = reqwest::Client::new();

    let request = ImageRequest {
        model,
        prompt,
        n: 1,
        response_format: "b64_json",
    };

    let resp = client
        .post(API_URL)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                "Timeout: la API tardó más de 120s.".to_string()
            } else if e.is_connect() {
                "Error de conexión. Verifica tu red.".to_string()
            } else {
                format!("Error HTTP: {}", e)
            }
        })?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        let msg = serde_json::from_str::<ErrorResponse>(&body)
            .ok()
            .and_then(|e| e.error)
            .and_then(|e| e.message)
            .unwrap_or(body);
        return Err(format!("HTTP {}: {}", status.as_u16(), msg));
    }

    let data: ImageResponse = resp.json().await
        .map_err(|e| format!("Error parseando respuesta: {}", e))?;

    let images = data.data.ok_or("La API no devolvió imágenes.")?;
    let first = images.first().ok_or("Lista de imágenes vacía.")?;
    let b64 = first.b64_json.as_deref().ok_or("Sin datos base64.")?;

    let bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD, b64
    ).map_err(|e| format!("Error decodificando base64: {}", e))?;

    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("xai_{}.png", ts);
    let filepath = std::path::Path::new(output_dir).join(&filename);

    std::fs::write(&filepath, &bytes)
        .map_err(|e| format!("Error guardando: {}", e))?;

    Ok(GenerationResult {
        filepath: filepath.to_string_lossy().to_string(),
        filename,
    })
}
