#![windows_subsystem = "windows"]
//! main.rs — Batch Image Generator v2.5.0 (Rust + Slint)
//!
//! Entry point. Wires up the Slint UI with the async API client,
//! randomizer, countdown timer logic, and Image-to-Image conditioning.
//! Supports up to 5 reference images for multi-ref models
//! (Flux Kontext Multi, UNO).

mod api;
mod config;
mod models;
mod notify;
mod pools;
mod randomizer;
mod util;

use api::{I2iMode, KeySlot};
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
    /// Banco de prompts: 5 ranuras. Los textos viven aquí, no en la
    /// interfaz, para no llenarla de propiedades y poder sortearlos.
    prompt_bank: Vec<String>,
    /// Selección manual de casillas guardada mientras el Super Randomizer
    /// está activo.
    ///
    /// Con Super activo las casillas se sobrescriben en cada generación para
    /// que se vea el sorteo. Sin esta copia, al apagarlo el usuario habría
    /// perdido la combinación que tenía puesta —y al cerrar la aplicación se
    /// habría guardado la última tirada aleatoria como si fuera su elección.
    manual_checks: Option<config::Checks>,
    /// Generación en curso, para poder abortarla al pulsar «Detener».
    ///
    /// Antes «Detener» sólo ponía `running = false`: la petición ya lanzada
    /// seguía viva hasta su timeout (180 s), así que el estado decía DETENIDO
    /// y minutos después aparecía otra imagen en la carpeta — facturada.
    current_task: Option<tokio::task::JoinHandle<()>>,
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

// El catálogo de modelos vivía aquí, duplicado a mano y en el mismo orden
// que la lista de `ui/main.slint`. Ahora es `src/models.rs`, y de ahí salen
// tanto el enrutado como las etiquetas de la interfaz.

/// Tamaño máximo del log en memoria.
///
/// La 2.4.0 concatenaba cada línea sobre el `String` completo
/// (`format!("{}{}", todo_el_log, linea)`), copiándolo entero en cada
/// escritura: coste O(n²) en tiempo y memoria que en sesiones Burst largas
/// degradaba la interfaz progresivamente y no liberaba nunca. Aquí el log se
/// acota y se recorta por la cabecera, siempre en frontera de línea.
const LOG_MAX_BYTES: usize = 200_000;

fn trim_log(s: &str) -> String {
    if s.len() <= LOG_MAX_BYTES {
        return s.to_string();
    }
    let target = s.len().saturating_sub(LOG_MAX_BYTES / 2);

    // Se corta preferentemente en un salto de línea para no partir un
    // mensaje por la mitad. Si no hay ninguno a partir de `target` —una
    // única línea gigantesca—, se corta igualmente en la primera frontera
    // de carácter válida: lo que no puede hacerse es devolver la cadena
    // entera, porque entonces el log nunca dejaría de crecer.
    let start = s
        .char_indices()
        .skip_while(|(i, _)| *i < target)
        .find(|(_, c)| *c == '\n')
        .map(|(i, _)| i + 1)
        .unwrap_or_else(|| {
            s.char_indices()
                .map(|(i, _)| i)
                .find(|i| *i >= target)
                .unwrap_or(s.len())
        });

    format!("[… log recortado …]\n{}", &s[start..])
}

fn main() {
    // Modo diagnóstico sin interfaz: `./xai-imagine-generator --test-notificacion`
    //
    // Existe porque «no veo ninguna notificación» tiene dos causas muy
    // distintas —la aplicación no consigue enviarla, o el escritorio la
    // recibe y no la muestra— y desde la ventana no se distinguen. Aquí el
    // resultado real de `show()` sale por la terminal, sin ventana de por
    // medio y sin depender de que el log se lea bien.
    if std::env::args().any(|a| a == "--test-notificacion" || a == "--test-notification") {
        let salida: notify::LogFn = Arc::new(|msg: String| println!("{msg}"));
        println!("{}", notify::diagnostico());
        notify::test(&salida);
        // `test` envía en su propio hilo; se le da margen antes de salir.
        std::thread::sleep(std::time::Duration::from_secs(2));
        return;
    }

    #[cfg(target_os = "macos")]
    {
        if std::env::var("SLINT_BACKEND").is_err() {
            std::env::set_var("SLINT_BACKEND", "winit-femtovg");
        }
    }

    let app = match MainWindow::new() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("No se pudo crear la ventana: {e}");
            eprintln!("En Linux sin servidor gráfico, exporta DISPLAY o usa una sesión con escritorio.");
            std::process::exit(1);
        }
    };

    // ── Preferencias guardadas ──
    // Se restauran skin, carpeta, modelo, intervalo y randomizer. Las API
    // keys NO se persisten (ver src/config.rs).
    // La lista del desplegable se genera desde la tabla de modelos: es
    // imposible que muestre un nombre y se envíe otro.
    app.set_model_list(slint::ModelRc::new(slint::VecModel::from(
        models::labels()
            .into_iter()
            .map(slint::SharedString::from)
            .collect::<Vec<_>>(),
    )));
    // Mismo orden que la lista de nombres: la interfaz lee el máximo de
    // referencias del modelo seleccionado y avisa antes de generar si hay
    // imágenes cargadas que ese modelo va a descartar.
    app.set_model_max_refs_list(slint::ModelRc::new(slint::VecModel::from(
        models::CATALOG
            .iter()
            .map(|m| m.max_refs as i32)
            .collect::<Vec<_>>(),
    )));

    let cfg = config::Config::load();
    apply_config(&app, &cfg);

    let state = Arc::new(Mutex::new(AppState {
        running: false,
        burst_mode: false,
        seconds_left: 0,
        interval: 60,
        ref_images: Vec::new(),
        prompt_bank: cfg.prompts.clone(),
        manual_checks: None,
        current_task: None,
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
                    let mut next = current;
                    next.push_str(&line);
                    app.set_log_text(trim_log(&next).into());
                }
            })
            .ok();
        }
    };

    // Versión de una sola pieza del log, para los sitios donde sólo hace
    // falta escribir una línea con nivel RAND.
    let append_log_plain = {
        let log = append_log.clone();
        move |msg: &str| log(msg, "RAND")
    };

    // ── Resolve prompt (Mode A or B) ──
    let resolve_prompt = {
        let app_weak = app.as_weak();
        let log_super = append_log_plain.clone();
        let state = state.clone();
        move || -> String {
            let app = app_weak.upgrade().unwrap();
            let mode = app.get_current_mode();

            if mode == 0 {
                // Banco de prompts: si el sorteo está activo, la base de esta
                // generación sale de una ranura guardada elegida al azar.
                //
                // No se sobrescribe el cuadro de texto: lo que hay escrito ahí
                // es del usuario y se conserva. El prompt elegido se ve en el
                // preview, ya montado con el randomizer encima.
                let base = if app.get_prompt_random_active() {
                    let guardados: Vec<(usize, String)> = state
                        .lock()
                        .map(|st| {
                            st.prompt_bank
                                .iter()
                                .enumerate()
                                .filter(|(_, p)| !p.trim().is_empty())
                                .map(|(i, p)| (i, p.clone()))
                                .collect()
                        })
                        .unwrap_or_default();

                    if guardados.is_empty() {
                        log_super(
                            "⚠ Prompt aleatorio activo pero el banco está vacío:                              se usa el prompt escrito.",
                        );
                        app.get_prompt_base().to_string()
                    } else {
                        let n = {
                            use rand::Rng;
                            rand::thread_rng().gen_range(0..guardados.len())
                        };
                        let (idx, texto) = guardados[n].clone();
                        log_super(&format!(
                            "🎲 Prompt del banco: ranura {} de {} guardadas — «{}»",
                            idx + 1,
                            guardados.len(),
                            util::truncate_chars(&texto, 60)
                        ));
                        texto
                    }
                } else {
                    app.get_prompt_base().to_string()
                };

                // Super Randomizer: se sortea la combinación en cada
                // generación y se vuelca sobre las casillas para que se vea
                // qué ha tocado esta vez.
                if app.get_super_rand_active() {
                    let (opts, nombres) = randomizer::random_options();
                    apply_checks_to_ui(&app, &randomizer::options_as_flags(&opts));
                    log_super(&format!(
                        "🎰 Super Randomizer: {} de {} categorías — {}",
                        nombres.len(),
                        randomizer::CATEGORY_NAMES.len(),
                        nombres.join(", ")
                    ));
                    let result = randomizer::modify_prompt(&base, &opts);
                    app.set_preview_text(result.clone().into());
                    return result;
                }

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
            let model_idx   = app.get_model_index() as usize;
            let res_idx = app.get_resolution_idx() as usize;
            let output_resolution = match res_idx {
                1 => "1k",
                2 => "2k",
                3 => "4k",
                _ => "",
            }.to_string();
            let spec = models::get(model_idx);
            let provider = spec.provider;

            // Cada proveedor toma su clave del campo que le corresponde.
            let api_key = match provider.key_slot() {
                KeySlot::WaveSpeed => app.get_wavespeed_api_key().to_string(),
                KeySlot::KieAi => app.get_kie_api_key().to_string(),
                KeySlot::General => app.get_api_key().to_string(),
            };
            let output_dir = app.get_output_folder().to_string();

            // Collect active reference images from shared state
            let mut ref_images: Vec<(String, String)> = {
                let st = state.lock().unwrap();
                st.active_ref_images()
            };

            // El modelo puede aceptar menos referencias de las cargadas.
            // Se avisa en vez de descartarlas en silencio o provocar un 400.
            if ref_images.len() > spec.max_refs {
                log(&format!(
                    "⚠ «{}» acepta {} imagen{} de referencia; se ignoran las {} restantes.",
                    spec.label,
                    spec.max_refs,
                    if spec.max_refs == 1 { "" } else { "es" },
                    ref_images.len() - spec.max_refs
                ), "WARN");
                ref_images.truncate(spec.max_refs);
            }

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

            // Canal de avance para la capa de red. `slint::Weak` es Send +
            // Sync, así que la tarea asíncrona puede escribir en el log a
            // través del bucle de eventos.
            let progress: api::ProgressFn = {
                let app_weak = app_weak.clone();
                Arc::new(move |msg: String| {
                    let app_weak = app_weak.clone();
                    let ts = chrono::Local::now().format("%H:%M:%S").to_string();
                    let line = format!("[{}] [API] {}\n", ts, msg);
                    slint::invoke_from_event_loop(move || {
                        if let Some(app) = app_weak.upgrade() {
                            let mut next = app.get_log_text().to_string();
                            next.push_str(&line);
                            app.set_log_text(trim_log(&next).into());
                        }
                    })
                    .ok();
                })
            };

            // Canal de log para los avisos de escritorio. Antes, si el
            // escritorio rechazaba la notificación, el error se iba a stderr:
            // invisible para quien arranca la aplicación desde el lanzador.
            // Ahora el fallo —y el diagnóstico del entorno— acaban en el log
            // que el usuario sí ve.
            let notify_log: notify::LogFn = {
                let app_weak = app_weak.clone();
                Arc::new(move |msg: String| {
                    let app_weak = app_weak.clone();
                    let ts = chrono::Local::now().format("%H:%M:%S").to_string();
                    let line = format!("[{}] [AVISO] {}\n", ts, msg);
                    slint::invoke_from_event_loop(move || {
                        if let Some(app) = app_weak.upgrade() {
                            let mut next = app.get_log_text().to_string();
                            next.push_str(&line);
                            app.set_log_text(trim_log(&next).into());
                        }
                    })
                    .ok();
                })
            };

            log(&format!("Proveedor: {}", provider.display_name()), "INFO");
            log(&format!("Modelo: {}", spec.label), "INFO");
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
                // Recorte por caracteres: `&base_prompt[..100]` entraba en
                // pánico si el byte 100 caía dentro de un carácter multibyte.
                let base_preview = util::truncate_chars(&base_prompt, 100);
                log(&format!("🎯 Base prompt: \"{}\"", base_preview), "INFO");
            }

            // Ídem: recorte por caracteres, no por bytes.
            let prompt_preview = util::truncate_chars(&prompt, 200);
            log(&format!("📝 Prompt final ({} caracteres): {}", prompt.chars().count(), prompt_preview), "INFO");

            if app.get_rand_active() {
                log("🎲 Randomizer: ACTIVO", "RAND");
            }

            log(&format!("Enviando petición a {} API...", provider.display_name()), "INFO");

            let handle = rt.spawn(async move {
                let result = api::generate_image(
                    spec,
                    &api_key,
                    &prompt,
                    &output_dir,
                    &ref_images,
                    i2i_mode,
                    &output_resolution,
                    &progress,
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

                                notify::notify(
                                    notify_settings(&app),
                                    notify::Event::Success,
                                    &format!("Imagen #{} — {}", count, gen.filename),
                                    &notify_log,
                                );

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

                                // El tipo de fallo sale del texto del
                                // proveedor: no hay un código uniforme para
                                // «rechazado por contenido» ni para «se me
                                // acabó el tiempo».
                                notify::notify(
                                    notify_settings(&app),
                                    notify::classify(&e),
                                    &e,
                                    &notify_log,
                                );

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

            // Se guarda para poder abortarla desde «Detener».
            if let Ok(mut st) = state.lock() {
                st.current_task = Some(handle);
            }
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

    // ── Super Randomizer ──
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let log = append_log.clone();
        app.on_toggle_super_randomizer(move || {
            if let Some(app) = app_weak.upgrade() {
                let activar = !app.get_super_rand_active();
                app.set_super_rand_active(activar);

                let Ok(mut st) = state.lock() else { return };
                if activar {
                    // Guardar la combinación manual para devolverla intacta
                    // al apagar. Sin esto, el sorteo la pisaría para siempre.
                    st.manual_checks = Some(checks_from_ui(&app));
                    drop(st);

                    // El Super Randomizer no tiene sentido con el randomizer
                    // apagado: se enciende solo.
                    if !app.get_rand_active() {
                        app.set_rand_active(true);
                        log("🎲 Randomizer activado por el Super Randomizer.", "RAND");
                    }
                    log(
                        "🎰 Super Randomizer ACTIVADO — cada generación sorteará cuántas \
                         categorías entran (de 1 a 21) y cuáles.",
                        "RAND",
                    );
                } else {
                    let manual = st.manual_checks.take();
                    drop(st);
                    if let Some(c) = manual {
                        checks_to_ui(&app, &c);
                        log(
                            "🎰 Super Randomizer DESACTIVADO — restaurada tu selección manual.",
                            "RAND",
                        );
                    } else {
                        log("🎰 Super Randomizer DESACTIVADO.", "RAND");
                    }
                }
            }
        });
    }

    // ── Banco de prompts ──
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let log = append_log.clone();
        app.on_save_prompt_slot(move || {
            if let Some(app) = app_weak.upgrade() {
                let texto = app.get_prompt_base().to_string();
                if texto.trim().is_empty() {
                    log("⚠ El prompt está vacío: no hay nada que guardar.", "WARN");
                    return;
                }
                let i = app.get_prompt_slot_index().clamp(0, config::PROMPT_SLOTS as i32 - 1) as usize;
                if let Ok(mut st) = state.lock() {
                    st.prompt_bank.resize(config::PROMPT_SLOTS, String::new());
                    st.prompt_bank[i] = texto.clone();
                    app.set_prompt_slots_summary(slots_summary(&st.prompt_bank).into());
                }
                log(
                    &format!(
                        "💾 Guardado en la ranura {}: «{}»",
                        i + 1,
                        util::truncate_chars(&texto, 60)
                    ),
                    "INFO",
                );
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let log = append_log.clone();
        app.on_load_prompt_slot(move || {
            if let Some(app) = app_weak.upgrade() {
                let i = app.get_prompt_slot_index().clamp(0, config::PROMPT_SLOTS as i32 - 1) as usize;
                let texto = state
                    .lock()
                    .ok()
                    .and_then(|st| st.prompt_bank.get(i).cloned())
                    .unwrap_or_default();
                if texto.trim().is_empty() {
                    log(&format!("⚠ La ranura {} está vacía.", i + 1), "WARN");
                } else {
                    app.set_prompt_base(texto.into());
                    log(&format!("📂 Cargada la ranura {}.", i + 1), "INFO");
                }
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let log = append_log.clone();
        app.on_clear_prompt_slot(move || {
            if let Some(app) = app_weak.upgrade() {
                let i = app.get_prompt_slot_index().clamp(0, config::PROMPT_SLOTS as i32 - 1) as usize;
                if let Ok(mut st) = state.lock() {
                    st.prompt_bank.resize(config::PROMPT_SLOTS, String::new());
                    st.prompt_bank[i] = String::new();
                    app.set_prompt_slots_summary(slots_summary(&st.prompt_bank).into());
                }
                log(&format!("🗑 Vaciada la ranura {}.", i + 1), "INFO");
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        let log = append_log.clone();
        app.on_toggle_prompt_random(move || {
            if let Some(app) = app_weak.upgrade() {
                let activar = !app.get_prompt_random_active();
                app.set_prompt_random_active(activar);
                if activar {
                    let guardados = state
                        .lock()
                        .map(|st| st.prompt_bank.iter().filter(|p| !p.trim().is_empty()).count())
                        .unwrap_or(0);
                    if guardados == 0 {
                        log(
                            "⚠ Prompt aleatorio ACTIVADO, pero el banco está vacío.                              Guarda al menos un prompt o se usará el que esté escrito.",
                            "WARN",
                        );
                    } else {
                        log(
                            &format!(
                                "🎲 Prompt aleatorio ACTIVADO — se sorteará entre {} prompt{} guardado{}.",
                                guardados,
                                if guardados == 1 { "" } else { "s" },
                                if guardados == 1 { "" } else { "s" }
                            ),
                            "RAND",
                        );
                    }
                } else {
                    log("🎲 Prompt aleatorio DESACTIVADO.", "RAND");
                }
            }
        });
    }

    // ── Notificaciones ──
    {
        let app_weak = app.as_weak();
        let log = append_log.clone();
        app.on_toggle_notifications(move || {
            if let Some(app) = app_weak.upgrade() {
                let activar = !app.get_notify_enabled();
                app.set_notify_enabled(activar);
                log(
                    if activar {
                        "🔔 Notificaciones de escritorio ACTIVADAS."
                    } else {
                        "🔕 Notificaciones de escritorio DESACTIVADAS."
                    },
                    "INFO",
                );
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let log = append_log.clone();
        app.on_test_notification(move || {
            if let Some(app) = app_weak.upgrade() {
                // El aviso de prueba se manda saltándose los interruptores por
                // tipo: el usuario quiere ver si su escritorio los muestra, no
                // comprobar sus preferencias. El interruptor general sí se
                // respeta, porque apagarlo significa «no quiero avisos».
                if !app.get_notify_enabled() {
                    log("🔕 No se envió el aviso de prueba (notificaciones desactivadas).", "INFO");
                    return;
                }
                // `notify::test` cuenta el resultado **real** de `show()`, no
                // el hecho de haber lanzado el hilo. Si el escritorio lo
                // rechaza, el error y el diagnóstico del entorno acaban aquí,
                // en el log que el usuario ve.
                let sink: notify::LogFn = {
                    let log = log.clone();
                    Arc::new(move |msg: String| log(&msg, "AVISO"))
                };
                notify::test(&sink);
                log(&notify::diagnostico(), "AVISO");
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
                // Sin esto, arrancar Burst y después «Iniciar Loop» sin pasar
                // por «Detener» dejaba `burst_mode` en true: el loop con
                // intervalo generaba sin pausa e ignoraba el temporizador.
                st.burst_mode = false;
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
                // Cancelar de verdad la generación en curso, no sólo dejar de
                // programar la siguiente.
                let cancelada = st.current_task.take().map(|h| {
                    h.abort();
                }).is_some();
                drop(st);

                app.set_is_running(false);
                app.set_status_text("● DETENIDO".into());
                app.set_status_color(slint::Color::from_rgb_u8(136, 136, 136));
                app.set_countdown_text("--:--".into());
                app.set_progress_value(0.0);
                app.set_progress_label("Detenido".into());
                app.set_progress_indeterminate(false);
                if cancelada {
                    log("■ Loop detenido — generación en curso cancelada.", "WARN");
                } else {
                    log("■ Loop detenido.", "WARN");
                }
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

    // ── Persistir la skin en cuanto se cambia ──
    {
        let app_weak = app.as_weak();
        let state = state.clone();
        app.on_skin_changed(move || {
            if let Some(app) = app_weak.upgrade() {
                let (manual, bank) = state
                    .lock()
                    .map(|st| (st.manual_checks.clone(), st.prompt_bank.clone()))
                    .unwrap_or((None, Vec::new()));
                if let Err(e) = collect_config(&app, manual.as_ref(), &bank).save() {
                    eprintln!("[config] no se pudo guardar: {e}");
                }
            }
        });
    }

    if let Err(e) = app.run() {
        eprintln!("Error en el bucle de la interfaz: {e}");
    }

    // ── Guardar preferencias al salir ──
    let (manual, bank) = state
        .lock()
        .map(|st| (st.manual_checks.clone(), st.prompt_bank.clone()))
        .unwrap_or((None, Vec::new()));
    if let Err(e) = collect_config(&app, manual.as_ref(), &bank).save() {
        eprintln!("[config] no se pudo guardar al salir: {e}");
    }
}

/// Resumen legible de qué ranuras del banco están ocupadas.
fn slots_summary(bank: &[String]) -> String {
    bank.iter()
        .enumerate()
        .map(|(i, p)| format!("{} {}", i + 1, if p.trim().is_empty() { "—" } else { "✓" }))
        .collect::<Vec<_>>()
        .join("  ·  ")
}

/// Lee las preferencias de notificación de la interfaz.
fn notify_settings(app: &MainWindow) -> notify::Settings {
    notify::Settings {
        enabled: app.get_notify_enabled(),
        on_success: app.get_notify_success(),
        on_timeout: app.get_notify_timeout(),
        on_policy: app.get_notify_policy(),
        on_other_error: app.get_notify_other(),
    }
}

/// Vuelca 21 interruptores sobre las casillas, en el orden de la interfaz.
///
/// Se usa para que el sorteo del Super Randomizer sea visible: en cada
/// generación las casillas muestran qué categorías han entrado.
fn apply_checks_to_ui(app: &MainWindow, f: &[bool; 21]) {
    app.set_chk_nails(f[0]);
    app.set_chk_orient(f[1]);
    app.set_chk_expression(f[2]);
    app.set_chk_outfit(f[3]);
    app.set_chk_legwear(f[4]);
    app.set_chk_environment(f[5]);
    app.set_chk_atmosphere(f[6]);
    app.set_chk_pose(f[7]);
    app.set_chk_lighting(f[8]);
    app.set_chk_camera(f[9]);
    app.set_chk_rare(f[10]);
    app.set_chk_accessories(f[11]);
    app.set_chk_makeup(f[12]);
    app.set_chk_body_type(f[13]);
    app.set_chk_age_vibe(f[14]);
    app.set_chk_color_palette(f[15]);
    app.set_chk_time_of_day(f[16]);
    app.set_chk_weather(f[17]);
    app.set_chk_bg_props(f[18]);
    app.set_chk_material(f[19]);
    app.set_chk_motion(f[20]);
}

/// Lee las casillas actuales de la interfaz.
fn checks_from_ui(app: &MainWindow) -> config::Checks {
    config::Checks {
        nails: app.get_chk_nails(),
        orient: app.get_chk_orient(),
        expression: app.get_chk_expression(),
        outfit: app.get_chk_outfit(),
        legwear: app.get_chk_legwear(),
        environment: app.get_chk_environment(),
        atmosphere: app.get_chk_atmosphere(),
        pose: app.get_chk_pose(),
        lighting: app.get_chk_lighting(),
        camera: app.get_chk_camera(),
        rare: app.get_chk_rare(),
        accessories: app.get_chk_accessories(),
        makeup: app.get_chk_makeup(),
        body_type: app.get_chk_body_type(),
        age_vibe: app.get_chk_age_vibe(),
        color_palette: app.get_chk_color_palette(),
        time_of_day: app.get_chk_time_of_day(),
        weather: app.get_chk_weather(),
        bg_props: app.get_chk_bg_props(),
        material: app.get_chk_material(),
        motion: app.get_chk_motion(),
        curated: app.get_chk_curated(),
        auto_b: app.get_chk_auto_b(),
    }
}

/// Escribe una estructura de casillas sobre la interfaz.
fn checks_to_ui(app: &MainWindow, c: &config::Checks) {
    apply_checks_to_ui(
        app,
        &[
            c.nails, c.orient, c.expression, c.outfit, c.legwear,
            c.environment, c.atmosphere, c.pose, c.lighting, c.camera,
            c.rare, c.accessories, c.makeup, c.body_type, c.age_vibe,
            c.color_palette, c.time_of_day, c.weather, c.bg_props,
            c.material, c.motion,
        ],
    );
    app.set_chk_curated(c.curated);
    app.set_chk_auto_b(c.auto_b);
}

/// Vuelca la configuración guardada sobre las propiedades de la interfaz.
fn apply_config(app: &MainWindow, cfg: &config::Config) {
    app.set_skin_index(cfg.skin);
    app.set_output_folder(cfg.output_folder.clone().into());
    app.set_model_index(cfg.model_index.clamp(0, models::CATALOG.len() as i32 - 1));
    app.set_resolution_idx(cfg.resolution_idx.clamp(0, 3));
    app.set_interval_secs(cfg.interval_secs.clamp(10, 600));
    app.set_current_mode(cfg.current_mode.clamp(0, 1));
    app.set_theme_index(cfg.theme_index.max(0));
    app.set_i2i_mode_index(cfg.i2i_mode_index.clamp(0, 1));
    app.set_rand_active(cfg.rand_active);
    app.set_super_rand_active(cfg.super_rand_active);
    app.set_prompt_slot_index(cfg.prompt_slot.clamp(0, config::PROMPT_SLOTS as i32 - 1));
    app.set_prompt_random_active(cfg.prompt_random);
    app.set_prompt_slots_summary(slots_summary(&cfg.prompts).into());
    app.set_notify_enabled(cfg.notify_enabled);
    app.set_notify_success(cfg.notify_success);
    app.set_notify_timeout(cfg.notify_timeout);
    app.set_notify_policy(cfg.notify_policy);
    app.set_notify_other(cfg.notify_other);

    let c = &cfg.checks;
    app.set_chk_nails(c.nails);
    app.set_chk_orient(c.orient);
    app.set_chk_expression(c.expression);
    app.set_chk_outfit(c.outfit);
    app.set_chk_legwear(c.legwear);
    app.set_chk_environment(c.environment);
    app.set_chk_atmosphere(c.atmosphere);
    app.set_chk_pose(c.pose);
    app.set_chk_lighting(c.lighting);
    app.set_chk_camera(c.camera);
    app.set_chk_rare(c.rare);
    app.set_chk_accessories(c.accessories);
    app.set_chk_makeup(c.makeup);
    app.set_chk_body_type(c.body_type);
    app.set_chk_age_vibe(c.age_vibe);
    app.set_chk_color_palette(c.color_palette);
    app.set_chk_time_of_day(c.time_of_day);
    app.set_chk_weather(c.weather);
    app.set_chk_bg_props(c.bg_props);
    app.set_chk_material(c.material);
    app.set_chk_motion(c.motion);
    app.set_chk_curated(c.curated);
    app.set_chk_auto_b(c.auto_b);
}

/// Lee el estado actual de la interfaz para guardarlo.
///
/// `manual` es la copia de las casillas de antes de activar el Super
/// Randomizer. Si existe, se guarda ésa: lo contrario sería registrar la
/// última tirada aleatoria como si fuera la elección del usuario.
fn collect_config(
    app: &MainWindow,
    manual: Option<&config::Checks>,
    bank: &[String],
) -> config::Config {
    config::Config {
        skin: app.get_skin_index(),
        output_folder: app.get_output_folder().to_string(),
        model_index: app.get_model_index(),
        resolution_idx: app.get_resolution_idx(),
        interval_secs: app.get_interval_secs(),
        current_mode: app.get_current_mode(),
        theme_index: app.get_theme_index(),
        i2i_mode_index: app.get_i2i_mode_index(),
        rand_active: app.get_rand_active(),
        super_rand_active: app.get_super_rand_active(),
        prompts: bank.to_vec(),
        prompt_slot: app.get_prompt_slot_index(),
        prompt_random: app.get_prompt_random_active(),
        notify_enabled: app.get_notify_enabled(),
        notify_success: app.get_notify_success(),
        notify_timeout: app.get_notify_timeout(),
        notify_policy: app.get_notify_policy(),
        notify_other: app.get_notify_other(),
        checks: manual.cloned().unwrap_or_else(|| config::Checks {
            nails: app.get_chk_nails(),
            orient: app.get_chk_orient(),
            expression: app.get_chk_expression(),
            outfit: app.get_chk_outfit(),
            legwear: app.get_chk_legwear(),
            environment: app.get_chk_environment(),
            atmosphere: app.get_chk_atmosphere(),
            pose: app.get_chk_pose(),
            lighting: app.get_chk_lighting(),
            camera: app.get_chk_camera(),
            rare: app.get_chk_rare(),
            accessories: app.get_chk_accessories(),
            makeup: app.get_chk_makeup(),
            body_type: app.get_chk_body_type(),
            age_vibe: app.get_chk_age_vibe(),
            color_palette: app.get_chk_color_palette(),
            time_of_day: app.get_chk_time_of_day(),
            weather: app.get_chk_weather(),
            bg_props: app.get_chk_bg_props(),
            material: app.get_chk_material(),
            motion: app.get_chk_motion(),
            curated: app.get_chk_curated(),
            auto_b: app.get_chk_auto_b(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_log_no_crece_sin_limite() {
        let mut s = String::new();
        for i in 0..20_000 {
            s.push_str(&format!("[12:00:00] [INFO] línea de log número {i} con acentos áéí\n"));
            s = trim_log(&s);
        }
        assert!(s.len() <= LOG_MAX_BYTES, "el log superó el tope: {}", s.len());
        assert!(s.contains("19999"), "debe conservar las líneas más recientes");
    }

    #[test]
    fn el_recorte_del_log_respeta_caracteres_multibyte() {
        // Relleno con caracteres de 2 bytes para forzar cortes en frontera.
        let s = "ñ".repeat(LOG_MAX_BYTES);
        let out = trim_log(&s); // no debe entrar en pánico
        assert!(
            out.len() < s.len(),
            "una línea sin saltos también debe recortarse ({} -> {})",
            s.len(),
            out.len()
        );

        // Repetido muchas veces, debe converger y no crecer sin límite.
        let mut acc = String::new();
        for _ in 0..50 {
            acc.push_str(&"ñ".repeat(20_000));
            acc = trim_log(&acc);
        }
        assert!(acc.len() <= LOG_MAX_BYTES + 64, "no converge: {}", acc.len());
    }

    /// Ya no puede desincronizarse: la lista se genera desde `CATALOG`. Este
    /// test protege de que alguien vuelva a escribirla a mano en el `.slint`.
    ///
    /// POR QUÉ AQUÍ NO SE ABRE LA VENTANA
    /// ----------------------------------
    /// La versión anterior llamaba a `MainWindow::new()`. En Linux headless
    /// eso devolvía `Err` y el `if let Ok` se saltaba la comprobación —el
    /// test pasaba sin comprobar nada—, pero en macOS entra en **pánico**:
    /// AppKit exige que el bucle de eventos se cree en el hilo principal y el
    /// arnés de `cargo test` ejecuta cada prueba en un hilo aparte, así que
    /// winit aborta con «on macOS, `EventLoop` must be created on the main
    /// thread!» y `cargo test` termina con código 101. Ése era el fallo del
    /// trabajo de CI de macOS.
    ///
    /// Una prueba unitaria no debe abrir una ventana. Lo que de verdad se
    /// quiere proteger es lo que declara el `.slint`, y eso se lee igual en
    /// las tres plataformas y sin sesión gráfica.
    #[test]
    fn la_lista_de_la_ui_sale_de_la_tabla_de_modelos() {
        let etiquetas = models::labels();
        assert_eq!(etiquetas.len(), models::CATALOG.len());
        assert!(etiquetas.iter().any(|e| e.starts_with("Kie.AI")));

        const UI: &str = include_str!("../ui/main.slint");
        for propiedad in ["model-list", "model-max-refs-list"] {
            let marca = format!("> {propiedad}:");
            let declaracion = UI
                .lines()
                .map(str::trim)
                .find(|l| l.starts_with("in-out property <[") && l.contains(&marca))
                .unwrap_or_else(|| panic!("ui/main.slint ya no declara «{propiedad}»"));
            assert!(
                declaracion.ends_with(": [];"),
                "ui/main.slint ha vuelto a llevar «{propiedad}» escrita a mano: {declaracion}"
            );
        }
    }

    /// La interfaz indexa `model-max-refs-list` con el índice del modelo
    /// seleccionado. Si las dos listas no tienen el mismo tamaño y el mismo
    /// orden, la ventana enseñaría el máximo de otro modelo.
    #[test]
    fn los_maximos_de_referencias_van_en_paralelo_a_los_nombres() {
        let maximos: Vec<i32> = models::CATALOG.iter().map(|m| m.max_refs as i32).collect();
        assert_eq!(maximos.len(), models::labels().len());
        for (m, maximo) in models::CATALOG.iter().zip(&maximos) {
            assert_eq!(*maximo, m.max_refs as i32, "desalineado en «{}»", m.label);
        }
    }
}
