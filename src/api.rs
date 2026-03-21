//! api.rs — Multi-provider image generation client.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageProvider {
    Xai,
    Google,
    OpenAi,
}

impl ImageProvider {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Xai => "xAI",
            Self::Google => "Google",
            Self::OpenAi => "OpenAI",
        }
    }

    fn api_url(self) -> &'static str {
        match self {
            Self::Xai => "https://api.x.ai/v1/images/generations",
            Self::Google => {
                "https://generativelanguage.googleapis.com/v1beta/openai/images/generations"
            }
            Self::OpenAi => "https://api.openai.com/v1/images/generations",
        }
    }

    fn file_prefix(self) -> &'static str {
        match self {
            Self::Xai => "xai",
            Self::Google => "google",
            Self::OpenAi => "openai",
        }
    }

    fn response_format(self, model: &str) -> Option<&'static str> {
        match self {
            Self::OpenAi if model.starts_with("gpt-image-") => None,
            _ => Some("b64_json"),
        }
    }
}

#[derive(Serialize)]
struct ImageRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    n: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<&'a str>,
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

    let client = reqwest::Client::new();
    let request = ImageRequest {
        model,
        prompt,
        n: 1,
        response_format: provider.response_format(model),
    };

    let resp = client
        .post(provider.api_url())
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                format!("Timeout: {} tardó más de 120s.", provider.display_name())
            } else if e.is_connect() {
                format!(
                    "Error de conexión con {}. Verifica tu red.",
                    provider.display_name()
                )
            } else {
                format!("Error HTTP con {}: {}", provider.display_name(), e)
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
        return Err(format!(
            "{} devolvió HTTP {}: {}",
            provider.display_name(),
            status.as_u16(),
            msg
        ));
    }

    let data: ImageResponse = resp.json().await.map_err(|e| {
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

    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("{}_{}.png", provider.file_prefix(), ts);
    let filepath = std::path::Path::new(output_dir).join(&filename);

    std::fs::write(&filepath, &bytes).map_err(|e| format!("Error guardando: {}", e))?;

    Ok(GenerationResult {
        filepath: filepath.to_string_lossy().to_string(),
        filename,
    })
}