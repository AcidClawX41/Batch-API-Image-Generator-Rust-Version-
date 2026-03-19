#![windows_subsystem = "windows"]
//! main.rs — xAI Imagine Batch Generator v1 (Rust + Slint)
//!
//! Entry point. Wires up the Slint UI with the async API client,
//! randomizer, and countdown timer logic.

mod api;
mod pools;
mod randomizer;

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
}

fn main() {
    // On macOS, prefer femtovg backend to work around click issues on Tahoe+.
    // Only sets the hint if the user hasn't already overridden via env var.
    #[cfg(target_os = "macos")]
    {
        if std::env::var("SLINT_BACKEND").is_err() {
            std::env::set_var("SLINT_BACKEND", "winit-femtovg");
        }
    }

    let app = MainWindow::new().unwrap();

    // Set default output folder
    if let Some(home) = dirs::home_dir() {
        let default_folder = home.join("xai_images").to_string_lossy().to_string();
        app.set_output_folder(default_folder.into());
    }

    let state = Arc::new(Mutex::new(AppState {
        running: false,
        seconds_left: 0,
        interval: 60,
    }));

    // Tokio runtime for async HTTP requests
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
                // Mode A
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
                    };
                    let result = randomizer::modify_prompt(&base, &opts);
                    app.set_preview_text(result.clone().into());
                    result
                } else {
                    base
                }
            } else {
                // Mode B
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
            let api_key = app.get_api_key().to_string();
            let model_idx = app.get_model_index() as usize;
            let models = ["grok-imagine-image", "grok-imagine-image-pro"];
            let model = models.get(model_idx).unwrap_or(&models[0]).to_string();
            let output_dir = app.get_output_folder().to_string();

            app.set_progress_indeterminate(true);
            app.set_progress_label("Generando imagen...".into());

            let app_weak2 = app_weak.clone();
            let log2 = log.clone();
            let state2 = state.clone();

            // ── Detailed debug log ──
            log(&format!("Modelo: {}", model), "INFO");
            log(&format!("Carpeta: {}", output_dir), "INFO");
            let prompt_preview = if prompt.len() > 120 {
                format!("{}...", &prompt[..120])
            } else {
                prompt.clone()
            };
            log(&format!("Prompt: {}", prompt_preview), "INFO");
            log("Enviando petición a xAI API...", "INFO");

            rt.spawn(async move {
                let result =
                    api::generate_image(&api_key, &prompt, &model, &output_dir).await;

                slint::invoke_from_event_loop(move || {
                    if let Some(app) = app_weak2.upgrade() {
                        app.set_progress_indeterminate(false);

                        match result {
                            Ok(gen) => {
                                let count = app.get_generation_count() + 1;
                                app.set_generation_count(count);
                                app.set_progress_value(1.0);
                                app.set_progress_label("Completado".into());
                                log2(
                                    &format!(
                                        "✅ Imagen #{} guardada: {}",
                                        count, gen.filepath
                                    ),
                                    "OK",
                                );

                                // Kick off countdown for next generation
                                let mut st = state2.lock().unwrap();
                                if st.running {
                                    st.seconds_left = st.interval;
                                    let secs = st.interval;
                                    drop(st);
                                    let mins = secs / 60;
                                    let s = secs % 60;
                                    app.set_countdown_text(
                                        format!("{:02}:{:02}", mins, s).into(),
                                    );
                                    app.set_progress_value(1.0);
                                    app.set_progress_label(
                                        format!("Siguiente en {}s", secs).into(),
                                    );
                                    log2(
                                        &format!(
                                            "⏱ Cuenta atrás: {}s hasta la siguiente.",
                                            secs
                                        ),
                                        "INFO",
                                    );
                                }
                            }
                            Err(e) => {
                                app.set_progress_value(0.0);
                                app.set_progress_label("Error".into());
                                log2(&format!("❌ {}", e), "ERROR");

                                // Even on error, restart countdown if loop is running
                                let mut st = state2.lock().unwrap();
                                if st.running {
                                    st.seconds_left = st.interval;
                                    let secs = st.interval;
                                    drop(st);
                                    log2(
                                        &format!(
                                            "⏱ Reintentando en {}s...",
                                            secs
                                        ),
                                        "WARN",
                                    );
                                }
                            }
                        }
                    }
                })
                .ok();
            });
        }
    };

    // ── Callbacks ──

    // Toggle randomizer (Mode A)
    {
        let app_weak = app.as_weak();
        let log = append_log.clone();
        app.on_toggle_randomizer(move || {
            if let Some(app) = app_weak.upgrade() {
                let active = !app.get_rand_active();
                app.set_rand_active(active);
                if active {
                    log("🎲 Randomizer ACTIVADO.", "RAND");
                    // Generate a preview
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

    // Generate preview (Mode B)
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

    // Browse folder
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
                if let Some(folder) = rfd::FileDialog::new()
                    .set_directory(&start)
                    .pick_folder()
                {
                    app.set_output_folder(folder.to_string_lossy().to_string().into());
                }
            }
        });
    }

    // Single generate
    {
        let fire = fire_generation.clone();
        let log = append_log.clone();
        app.on_single_generate(move || {
            log("⚡ Generación única solicitada.", "INFO");
            fire();
        });
    }

    // Start loop
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
                st.seconds_left = 0; // Fire immediately, countdown starts after
                drop(st);

                app.set_is_running(true);
                app.set_status_text("● GENERANDO".into());
                app.set_status_color(slint::Color::from_rgb_u8(111, 207, 111));
                log("▶ Iniciando loop de generación.", "INFO");
                fire();
            }
        });
    }

    // Stop loop
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let log = append_log.clone();

        app.on_stop_loop(move || {
            if let Some(app) = app_weak.upgrade() {
                let mut st = state.lock().unwrap();
                st.running = false;
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

        timer.start(
            TimerMode::Repeated,
            Duration::from_secs(1),
            move || {
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
                    } else {
                        // Countdown hit 0 — fire next generation
                        // seconds_left will be reset in fire_generation's success callback
                        drop(st);
                        fire();
                    }
                }
            },
        );
        timer // keep alive
    };

    app.run().unwrap();
}
