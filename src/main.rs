#![windows_subsystem = "windows"]
//! main.rs — Batch Image Generator v2.4.0 (Rust + Slint)
//!
//! Entry point. Wires up the Slint UI with the async API client,
//! randomizer, countdown timer logic, and Image-to-Image conditioning.
//! Supports up to 5 reference images for multi-ref models
//! (Flux Kontext Multi, UNO).

mod api;
mod pools;
mod randomizer;

use api::{I2iMode, ImageProvider};
use randomizer::ModifyOptions;
use slint::{Timer, TimerMode};
use std::sync::{Arc, Mutex};
use std::time::Duration;

slint::include_modules!();

/// Shared mutable state for the countdown and generation loop.
struct AppState {
    running: bool,
    seconds_left: i32,
    interval: i32,
    /// True when burst mode is active (generate continuously with no delay).
    burst_mode: bool,
    /// Reference images: Vec of (base64_string, mime_string).
    /// Index 0 = persona/primary, 1 = escena/secondary, 2-4 = extra refs.
    /// An empty base64 string means that slot is not loaded.
    ref_images: Vec<(String, String)>,
}

impl AppState {
    /// Returns only the images that are actually loaded (non-empty b64).
    fn active_ref_images(&self) -> Vec<(String, String)> {
        self.ref_images
            .iter()
            .filter(|(b64, _)| !b64.is_empty())
            .cloned()
            .collect()
    }

    /// Set a reference image at a given slot index.
    fn set_ref_image(&mut self, index: usize, b64: String, mime: String) {
        while self.ref_images.len() <= index {
            self.ref_images.push((String::new(), "image/png".to_string()));
        }
        self.ref_images[index] = (b64, mime);
    }

    /// Clear a reference image at a given slot index.
    fn clear_ref_image(&mut self, index: usize) {
        if index < self.ref_images.len() {
            self.ref_images[index] = (String::new(), "image/png".to_string());
        }
    }
}

struct ModelCatalogEntry {
    provider: ImageProvider,
    model: &'static str,
}

const MODEL_CATALOG: &[ModelCatalogEntry] = &[
    // ── xAI ──
    ModelCatalogEntry { provider: ImageProvider::Xai,       model: "grok-imagine-image" },
    ModelCatalogEntry { provider: ImageProvider::Xai,       model: "grok-imagine-image-pro" },
    // ── Google ──
    ModelCatalogEntry { provider: ImageProvider::Google,    model: "gemini-2.5-flash-image" },
    ModelCatalogEntry { provider: ImageProvider::Google,    model: "gemini-3-pro-image-preview" },
    // ── OpenAI ──
    ModelCatalogEntry { provider: ImageProvider::OpenAi,    model: "gpt-image-1.5" },
    ModelCatalogEntry { provider: ImageProvider::OpenAi,    model: "gpt-image-1" },
    ModelCatalogEntry { provider: ImageProvider::OpenAi,    model: "gpt-image-1-mini" },
    ModelCatalogEntry { provider: ImageProvider::OpenAi,    model: "dall-e-3" },
    // ── WaveSpeed: Flux 2 family (T2I only) ──
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "wavespeed-ai/flux-2-max/text-to-image" },
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "wavespeed-ai/flux-2-dev/text-to-image" },
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "wavespeed-ai/flux-2-flash/text-to-image" },
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "wavespeed-ai/flux-2-flex/text-to-image" },
    // ── WaveSpeed: Flux Kontext single (1 img ref) ──
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "wavespeed-ai/flux-kontext-max/text-to-image" },
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "wavespeed-ai/flux-kontext-pro/text-to-image" },
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "wavespeed-ai/flux-kontext-dev" },
    // ── WaveSpeed: Flux Kontext Multi (up to 5 imgs ref) ──
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "wavespeed-ai/flux-kontext-max/multi" },
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "wavespeed-ai/flux-kontext-pro/multi" },
    // ── WaveSpeed: WAN (I2I) ──
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "wavespeed-ai/wan-2.2/image-to-image" },
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "alibaba/wan-2.6/text-to-image" },
    // ── WaveSpeed: UNO (multi-ref) ──
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "wavespeed-ai/uno" },
    // ── WaveSpeed: Seedream (ByteDance) ──
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "bytedance/seedream-v5.0-lite" },
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "bytedance/seedream-v4.5" },
    // ── WaveSpeed: Nano Banana (Google via WaveSpeed) ──
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "google/nano-banana-2/text-to-image" },
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "google/nano-banana-pro/text-to-image" },
    // ── WaveSpeed: Dreamina (ByteDance) ──
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "bytedance/dreamina-v3.1/text-to-image" },
    // ── WaveSpeed: Qwen Image (Alibaba) ──
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "wavespeed-ai/qwen-image-2.0-pro/text-to-image" },
    // ── WaveSpeed: Kling (Kuaishou) ──
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "kwaivgi/kling-image-o3/text-to-image" },
    // ── WaveSpeed: Grok (xAI via WaveSpeed) ──
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "x-ai/grok-2-image" },
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "x-ai/grok-imagine-image-text-to-image" },
    // ── WaveSpeed: Grok Imagine Edit (I2I nativo) ──
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "x-ai/grok-imagine-image/edit" },
    // ── WaveSpeed: Nano Banana Edit (hasta 14 imgs) ──
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "google/nano-banana-2/edit" },
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "google/nano-banana-2/edit-fast" },
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "google/nano-banana-pro/edit" },
    // ── WaveSpeed: Seedream Edit ──
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "bytedance/seedream-v5.0-lite/edit" },
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "bytedance/seedream-v4.5/edit" },
    // ── WaveSpeed: Qwen Image Edit ──
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "wavespeed-ai/qwen-image/edit" },
    // ── WaveSpeed: Flux 2 Klein Edit ──
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "wavespeed-ai/flux-2-klein-4b/edit" },
    // ── WaveSpeed: WAN 2.7 Image Edit ──
    ModelCatalogEntry { provider: ImageProvider::WaveSpeed, model: "alibaba/wan-2.7/image-edit" },
];

fn main() {
    #[cfg(target_os = "macos")]
    {
        if std::env::var("SLINT_BACKEND").is_err() {
            std::env::set_var("SLINT_BACKEND", "winit-femtovg");
        }
    }

    let app = MainWindow::new().unwrap();

    if let Some(home) = dirs::home_dir() {
        let default_folder = home.join("batch_images").to_string_lossy().to_string();
        app.set_output_folder(default_folder.into());
    }

    let state = Arc::new(Mutex::new(AppState {
        running: false,
        burst_mode: false,
        seconds_left: 0,
        interval: 60,
        ref_images: Vec::new(),
    }));

    let rt = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap(),
    );

    // ── Helper: append to log ──
    let append_log = {
        let app_weak = app.as_weak();
        move |msg: &str, level: &str| {
            let app_weak = app_weak.clone();
            let ts = chrono::Local::now().format("%H:%M:%S").to_string();
            let line = format!("[{}] [{}] {}\n", ts, level, msg);
            slint::invoke_from_event_loop(move || {
                if let Some(app) = app_weak.upgrade() {
                    let current = app.get_log_text().to_string();
                    app.set_log_text(format!("{}{}", current, line).into());
                }
            })
            .ok();
        }
    };

    // ── Resolve prompt (Mode A or B) ──
    let resolve_prompt = {
        let app_weak = app.as_weak();
        move || -> String {
            let app = app_weak.upgrade().unwrap();
            let mode = app.get_current_mode();

            if mode == 0 {
                let base = app.get_prompt_base().to_string();
                if app.get_rand_active() {
                    let opts = ModifyOptions {
                        do_nails: app.get_chk_nails(),
                        do_orientation: app.get_chk_orient(),
                        do_expression: app.get_chk_expression(),
                        do_outfit: app.get_chk_outfit(),
                        do_legwear: app.get_chk_legwear(),
                        do_environment: app.get_chk_environment(),
                        do_atmosphere: app.get_chk_atmosphere(),
                        do_pose: app.get_chk_pose(),
                        do_lighting: app.get_chk_lighting(),
                        do_camera: app.get_chk_camera(),
                        do_rare: app.get_chk_rare(),
                        do_accessories: app.get_chk_accessories(),
                        do_makeup: app.get_chk_makeup(),
                        do_body_type: app.get_chk_body_type(),
                        do_age_vibe: app.get_chk_age_vibe(),
                        do_color_palette: app.get_chk_color_palette(),
                        do_time_of_day: app.get_chk_time_of_day(),
                        do_weather: app.get_chk_weather(),
                        do_bg_props: app.get_chk_bg_props(),
                        do_material: app.get_chk_material(),
                        do_motion: app.get_chk_motion(),
                    };
                    let result = randomizer::modify_prompt(&base, &opts);
                    app.set_preview_text(result.clone().into());
                    result
                } else {
                    base
                }
            } else {
                let theme_idx = app.get_theme_index() as usize;
                let curated = app.get_chk_curated();
                if app.get_chk_auto_b() {
                    let result = randomizer::generate_full_prompt(theme_idx, curated);
                    app.set_preview_text(result.clone().into());
                    result
                } else {
                    let txt = app.get_preview_text().to_string();
                    if txt.trim().is_empty() {
                        let result = randomizer::generate_full_prompt(theme_idx, curated);
                        app.set_preview_text(result.clone().into());
                        result
                    } else {
                        txt
                    }
                }
            }
        }
    };

    // ── Fire generation ──
    let fire_generation = {
        let app_weak = app.as_weak();
        let rt = rt.clone();
        let state = state.clone();
        let log = append_log.clone();
        let resolve = resolve_prompt.clone();

        move || {
            let app = app_weak.upgrade().unwrap();
            let prompt = resolve();
            let model_idx = app.get_model_index() as usize;
            let selected_model = MODEL_CATALOG.get(model_idx).unwrap_or(&MODEL_CATALOG[0]);

            let api_key = if selected_model.provider == ImageProvider::WaveSpeed {
                app.get_wavespeed_api_key().to_string()
            } else {
                app.get_api_key().to_string()
            };
            let provider = selected_model.provider;
            let model = selected_model.model.to_string();
            let output_dir = app.get_output_folder().to_string();

            // Collect active reference images from shared state
            let ref_images: Vec<(String, String)> = {
                let st = state.lock().unwrap();
                st.active_ref_images()
            };

            let i2i_mode = if app.get_i2i_mode_index() == 1 {
                I2iMode::DirectEdit
            } else {
                I2iMode::StyleReference
            };

            app.set_progress_indeterminate(true);
            app.set_progress_label("Generando imagen...".into());

            let app_weak2 = app_weak.clone();
            let log2 = log.clone();
            let state2 = state.clone();

            log(&format!("Proveedor: {}", provider.display_name()), "INFO");
            log(&format!("Modelo: {}", model), "INFO");
            log(&format!("Carpeta: {}", output_dir), "INFO");

            if !ref_images.is_empty() {
                let mode_str = match i2i_mode {
                    I2iMode::StyleReference => "Referencia de Estilo",
                    I2iMode::DirectEdit     => "Edición Directa",
                };
                log(&format!(
                    "🖼 Image-to-Image: ACTIVO ({} imagen{} · {})",
                    ref_images.len(),
                    if ref_images.len() == 1 { "" } else { "es" },
                    mode_str
                ), "I2I");
            }

            let base_prompt = app.get_prompt_base().to_string();
            if !base_prompt.trim().is_empty() {
                let base_preview = if base_prompt.len() > 100 {
                    format!("{}...", &base_prompt[..100])
                } else {
                    base_prompt.clone()
                };
                log(&format!("🎯 Base prompt: \"{}\"", base_preview), "INFO");
            }

            let prompt_preview = if prompt.len() > 200 {
                format!("{}...", &prompt[..200])
            } else {
                prompt.clone()
            };
            log(&format!("📝 Prompt final ({}ch): {}", prompt.len(), prompt_preview), "INFO");

            if app.get_rand_active() {
                log("🎲 Randomizer: ACTIVO", "RAND");
            }

            log(&format!("Enviando petición a {} API...", provider.display_name()), "INFO");

            rt.spawn(async move {
                let result = api::generate_image(
                    provider,
                    &api_key,
                    &prompt,
                    &model,
                    &output_dir,
                    &ref_images,
                    i2i_mode,
                )
                .await;

                slint::invoke_from_event_loop(move || {
                    if let Some(app) = app_weak2.upgrade() {
                        app.set_progress_indeterminate(false);

                        match result {
                            Ok(gen) => {
                                let count = app.get_generation_count() + 1;
                                app.set_generation_count(count);
                                app.set_progress_value(1.0);
                                app.set_progress_label("Completado".into());
                                log2(&format!(
                                    "✅ Imagen #{} guardada: {} ({})",
                                    count, gen.filepath, gen.filename
                                ), "OK");

                                let mut st = state2.lock().unwrap();
                                if st.running {
                                    if st.burst_mode {
                                        // Burst: fire next generation immediately (next timer tick)
                                        st.seconds_left = 0;
                                        drop(st);
                                        app.set_countdown_text("⚡".into());
                                        app.set_progress_value(1.0);
                                        app.set_progress_label("Burst — siguiente...".into());
                                        log2(&format!("⚡ Burst #{} completado — disparando siguiente.", count), "BURST");
                                    } else {
                                        st.seconds_left = st.interval;
                                        let secs = st.interval;
                                        drop(st);
                                        let mins = secs / 60;
                                        let s = secs % 60;
                                        app.set_countdown_text(format!("{:02}:{:02}", mins, s).into());
                                        app.set_progress_value(1.0);
                                        app.set_progress_label(format!("Siguiente en {}s", secs).into());
                                        log2(&format!("⏱ Cuenta atrás: {}s hasta la siguiente.", secs), "INFO");
                                    }
                                }
                            }
                            Err(e) => {
                                app.set_progress_value(0.0);
                                app.set_progress_label("Error".into());
                                log2(&format!("❌ {}", e), "ERROR");

                                let mut st = state2.lock().unwrap();
                                if st.running {
                                    if st.burst_mode {
                                        // Burst: retry immediately even on error
                                        st.seconds_left = 0;
                                        drop(st);
                                        log2("⚡ Burst: error — reintentando inmediatamente...", "WARN");
                                    } else {
                                        st.seconds_left = st.interval;
                                        let secs = st.interval;
                                        drop(st);
                                        log2(&format!("⏱ Reintentando en {}s...", secs), "WARN");
                                    }
                                }
                            }
                        }
                    }
                })
                .ok();
            });
        }
    };

    // ── Helper macro / closure for browse image by index ──
    // (inline per slot to avoid complex Rust closure captures)

    // ── Toggle randomizer (Mode A) ──
    {
        let app_weak = app.as_weak();
        let log = append_log.clone();
        app.on_toggle_randomizer(move || {
            if let Some(app) = app_weak.upgrade() {
                let active = !app.get_rand_active();
                app.set_rand_active(active);
                if active {
                    log("🎲 Randomizer ACTIVADO.", "RAND");
                    let base = app.get_prompt_base().to_string();
                    let opts = ModifyOptions {
                        do_nails: app.get_chk_nails(),
                        do_orientation: app.get_chk_orient(),
                        do_expression: app.get_chk_expression(),
                        do_outfit: app.get_chk_outfit(),
                        do_legwear: app.get_chk_legwear(),
                        do_environment: app.get_chk_environment(),
                        do_atmosphere: app.get_chk_atmosphere(),
                        do_pose: app.get_chk_pose(),
                        do_lighting: app.get_chk_lighting(),
                        do_camera: app.get_chk_camera(),
                        do_rare: app.get_chk_rare(),
                        do_accessories: app.get_chk_accessories(),
                        do_makeup: app.get_chk_makeup(),
                        do_body_type: app.get_chk_body_type(),
                        do_age_vibe: app.get_chk_age_vibe(),
                        do_color_palette: app.get_chk_color_palette(),
                        do_time_of_day: app.get_chk_time_of_day(),
                        do_weather: app.get_chk_weather(),
                        do_bg_props: app.get_chk_bg_props(),
                        do_material: app.get_chk_material(),
                        do_motion: app.get_chk_motion(),
                    };
                    let preview = randomizer::modify_prompt(&base, &opts);
                    app.set_preview_text(preview.into());
                } else {
                    log("🎲 Randomizer DESACTIVADO.", "RAND");
                    app.set_preview_text("".into());
                }
            }
        });
    }

    // ── Generate preview (Mode B) ──
    {
        let app_weak = app.as_weak();
        let log = append_log.clone();
        app.on_gen_preview_b(move || {
            if let Some(app) = app_weak.upgrade() {
                let idx = app.get_theme_index() as usize;
                let curated = app.get_chk_curated();
                let prompt = randomizer::generate_full_prompt(idx, curated);
                app.set_preview_text(prompt.into());
                log("🎰 Prompt auto-generado (preview).", "GEN");
            }
        });
    }

    // ── Browse output folder ──
    {
        let app_weak = app.as_weak();
        app.on_browse_folder(move || {
            if let Some(app) = app_weak.upgrade() {
                let current = app.get_output_folder().to_string();
                let start = if current.is_empty() {
                    dirs::home_dir().unwrap_or_default()
                } else {
                    std::path::PathBuf::from(&current)
                };
                if let Some(folder) = rfd::FileDialog::new().set_directory(&start).pick_folder() {
                    app.set_output_folder(folder.to_string_lossy().to_string().into());
                }
            }
        });
    }

    // ── Browse reference image #1 ──
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let log = append_log.clone();
        app.on_browse_ref_image(move || {
            if let Some(app) = app_weak.upgrade() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Seleccionar imagen de referencia 1 (persona)")
                    .add_filter("Imágenes", &["png", "jpg", "jpeg", "webp"])
                    .set_directory(dirs::home_dir().unwrap_or_default())
                    .pick_file()
                {
                    match std::fs::read(&path) {
                        Ok(bytes) => {
                            use base64::Engine;
                            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                            let ext = path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
                            let mime = api::mime_from_ext(&ext).to_string();
                            let size_kb = bytes.len() / 1024;
                            state.lock().unwrap().set_ref_image(0, b64, mime.clone());
                            let filename = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "imagen1".to_string());
                            app.set_ref_image_path(filename.clone().into());
                            app.set_ref_image_loaded(true);
                            log(&format!("🖼 Img 1 cargada: {} ({}KB, {})", filename, size_kb, mime), "I2I");
                        }
                        Err(e) => log(&format!("❌ Error leyendo Img 1: {}", e), "ERROR"),
                    }
                }
            }
        });
    }

    // ── Clear reference image #1 ──
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let log = append_log.clone();
        app.on_clear_ref_image(move || {
            state.lock().unwrap().clear_ref_image(0);
            if let Some(app) = app_weak.upgrade() {
                app.set_ref_image_path("".into());
                app.set_ref_image_loaded(false);
            }
            log("🗑 Img 1 eliminada.", "I2I");
        });
    }

    // ── Browse reference image #2 ──
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let log = append_log.clone();
        app.on_browse_ref_image2(move || {
            if let Some(app) = app_weak.upgrade() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Seleccionar imagen de referencia 2 (escena / vehículo)")
                    .add_filter("Imágenes", &["png", "jpg", "jpeg", "webp"])
                    .set_directory(dirs::home_dir().unwrap_or_default())
                    .pick_file()
                {
                    match std::fs::read(&path) {
                        Ok(bytes) => {
                            use base64::Engine;
                            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                            let ext = path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
                            let mime = api::mime_from_ext(&ext).to_string();
                            let size_kb = bytes.len() / 1024;
                            state.lock().unwrap().set_ref_image(1, b64, mime.clone());
                            let filename = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "imagen2".to_string());
                            app.set_ref_image2_path(filename.clone().into());
                            app.set_ref_image2_loaded(true);
                            log(&format!("🖼 Img 2 cargada: {} ({}KB, {})", filename, size_kb, mime), "I2I");
                        }
                        Err(e) => log(&format!("❌ Error leyendo Img 2: {}", e), "ERROR"),
                    }
                }
            }
        });
    }

    // ── Clear reference image #2 ──
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let log = append_log.clone();
        app.on_clear_ref_image2(move || {
            state.lock().unwrap().clear_ref_image(1);
            if let Some(app) = app_weak.upgrade() {
                app.set_ref_image2_path("".into());
                app.set_ref_image2_loaded(false);
            }
            log("🗑 Img 2 eliminada.", "I2I");
        });
    }

    // ── Browse reference image #3 ──
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let log = append_log.clone();
        app.on_browse_ref_image3(move || {
            if let Some(app) = app_weak.upgrade() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Seleccionar imagen de referencia 3 (extra — Flux Kontext Multi / UNO)")
                    .add_filter("Imágenes", &["png", "jpg", "jpeg", "webp"])
                    .set_directory(dirs::home_dir().unwrap_or_default())
                    .pick_file()
                {
                    match std::fs::read(&path) {
                        Ok(bytes) => {
                            use base64::Engine;
                            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                            let ext = path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
                            let mime = api::mime_from_ext(&ext).to_string();
                            let size_kb = bytes.len() / 1024;
                            state.lock().unwrap().set_ref_image(2, b64, mime.clone());
                            let filename = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "imagen3".to_string());
                            app.set_ref_image3_path(filename.clone().into());
                            app.set_ref_image3_loaded(true);
                            log(&format!("🖼 Img 3 cargada: {} ({}KB, {})", filename, size_kb, mime), "I2I");
                        }
                        Err(e) => log(&format!("❌ Error leyendo Img 3: {}", e), "ERROR"),
                    }
                }
            }
        });
    }

    // ── Clear reference image #3 ──
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let log = append_log.clone();
        app.on_clear_ref_image3(move || {
            state.lock().unwrap().clear_ref_image(2);
            if let Some(app) = app_weak.upgrade() {
                app.set_ref_image3_path("".into());
                app.set_ref_image3_loaded(false);
            }
            log("🗑 Img 3 eliminada.", "I2I");
        });
    }

    // ── Browse reference image #4 ──
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let log = append_log.clone();
        app.on_browse_ref_image4(move || {
            if let Some(app) = app_weak.upgrade() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Seleccionar imagen de referencia 4 (extra — Flux Kontext Multi / UNO)")
                    .add_filter("Imágenes", &["png", "jpg", "jpeg", "webp"])
                    .set_directory(dirs::home_dir().unwrap_or_default())
                    .pick_file()
                {
                    match std::fs::read(&path) {
                        Ok(bytes) => {
                            use base64::Engine;
                            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                            let ext = path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
                            let mime = api::mime_from_ext(&ext).to_string();
                            let size_kb = bytes.len() / 1024;
                            state.lock().unwrap().set_ref_image(3, b64, mime.clone());
                            let filename = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "imagen4".to_string());
                            app.set_ref_image4_path(filename.clone().into());
                            app.set_ref_image4_loaded(true);
                            log(&format!("🖼 Img 4 cargada: {} ({}KB, {})", filename, size_kb, mime), "I2I");
                        }
                        Err(e) => log(&format!("❌ Error leyendo Img 4: {}", e), "ERROR"),
                    }
                }
            }
        });
    }

    // ── Clear reference image #4 ──
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let log = append_log.clone();
        app.on_clear_ref_image4(move || {
            state.lock().unwrap().clear_ref_image(3);
            if let Some(app) = app_weak.upgrade() {
                app.set_ref_image4_path("".into());
                app.set_ref_image4_loaded(false);
            }
            log("🗑 Img 4 eliminada.", "I2I");
        });
    }

    // ── Browse reference image #5 ──
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let log = append_log.clone();
        app.on_browse_ref_image5(move || {
            if let Some(app) = app_weak.upgrade() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Seleccionar imagen de referencia 5 (extra — Flux Kontext Multi / UNO)")
                    .add_filter("Imágenes", &["png", "jpg", "jpeg", "webp"])
                    .set_directory(dirs::home_dir().unwrap_or_default())
                    .pick_file()
                {
                    match std::fs::read(&path) {
                        Ok(bytes) => {
                            use base64::Engine;
                            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                            let ext = path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
                            let mime = api::mime_from_ext(&ext).to_string();
                            let size_kb = bytes.len() / 1024;
                            state.lock().unwrap().set_ref_image(4, b64, mime.clone());
                            let filename = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "imagen5".to_string());
                            app.set_ref_image5_path(filename.clone().into());
                            app.set_ref_image5_loaded(true);
                            log(&format!("🖼 Img 5 cargada: {} ({}KB, {})", filename, size_kb, mime), "I2I");
                        }
                        Err(e) => log(&format!("❌ Error leyendo Img 5: {}", e), "ERROR"),
                    }
                }
            }
        });
    }

    // ── Clear reference image #5 ──
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let log = append_log.clone();
        app.on_clear_ref_image5(move || {
            state.lock().unwrap().clear_ref_image(4);
            if let Some(app) = app_weak.upgrade() {
                app.set_ref_image5_path("".into());
                app.set_ref_image5_loaded(false);
            }
            log("🗑 Img 5 eliminada.", "I2I");
        });
    }

    // ── Single generate ──
    {
        let fire = fire_generation.clone();
        let log = append_log.clone();
        app.on_single_generate(move || {
            log("⚡ Generación única solicitada.", "INFO");
            fire();
        });
    }

    // ── Start loop ──
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let fire = fire_generation.clone();
        let log = append_log.clone();

        app.on_start_loop(move || {
            if let Some(app) = app_weak.upgrade() {
                let mut st = state.lock().unwrap();
                st.running = true;
                st.interval = app.get_interval_secs();
                st.seconds_left = -1;
                drop(st);

                app.set_is_running(true);
                app.set_status_text("● GENERANDO".into());
                app.set_status_color(slint::Color::from_rgb_u8(111, 207, 111));
                log("▶ Iniciando loop de generación.", "INFO");
                fire();
            }
        });
    }

    // ── Start burst (continuous, no interval) ──
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let fire = fire_generation.clone();
        let log = append_log.clone();

        app.on_start_burst(move || {
            if let Some(app) = app_weak.upgrade() {
                let mut st = state.lock().unwrap();
                st.running = true;
                st.burst_mode = true;
                st.seconds_left = -1;
                drop(st);

                app.set_is_running(true);
                app.set_status_text("⚡ BURST".into());
                app.set_status_color(slint::Color::from_rgb_u8(240, 180, 40));
                log("⚡ Burst Generation iniciado — sin espera entre generaciones.", "BURST");
                fire();
            }
        });
    }

    // ── Stop loop ──
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let log = append_log.clone();

        app.on_stop_loop(move || {
            if let Some(app) = app_weak.upgrade() {
                let mut st = state.lock().unwrap();
                st.running = false;
                st.burst_mode = false;
                st.seconds_left = 0;
                drop(st);

                app.set_is_running(false);
                app.set_status_text("● DETENIDO".into());
                app.set_status_color(slint::Color::from_rgb_u8(136, 136, 136));
                app.set_countdown_text("--:--".into());
                app.set_progress_value(0.0);
                app.set_progress_label("Detenido".into());
                log("■ Loop detenido.", "WARN");
            }
        });
    }

    // ── Countdown timer ──
    let _countdown_timer = {
        let timer = Timer::default();
        let app_weak = app.as_weak();
        let state = state.clone();
        let fire = fire_generation.clone();

        timer.start(TimerMode::Repeated, Duration::from_secs(1), move || {
            if let Some(app) = app_weak.upgrade() {
                let mut st = state.lock().unwrap();
                if !st.running {
                    return;
                }

                if st.seconds_left > 0 {
                    st.seconds_left -= 1;
                    let secs = st.seconds_left;
                    let total = st.interval;
                    drop(st);

                    let mins = secs / 60;
                    let s = secs % 60;
                    app.set_countdown_text(format!("{:02}:{:02}", mins, s).into());
                    if total > 0 {
                        app.set_progress_value(secs as f32 / total as f32);
                    }
                    app.set_progress_label(format!("Siguiente en {}s", secs).into());
                } else if st.seconds_left == 0 {
                    st.seconds_left = -1;
                    drop(st);
                    fire();
                } else {
                    // < 0 means currently generating
                }
            }
        });
        timer
    };

    app.run().unwrap();
}
