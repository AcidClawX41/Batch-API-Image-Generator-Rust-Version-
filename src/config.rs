//! config.rs — Persistencia de preferencias entre arranques.
//!
//! Batch Image Generator — Eric Valls Gramunt
//!
//! POR QUÉ EXISTE
//! --------------
//! Hasta la 2.4.0 toda la configuración vivía únicamente en propiedades de la
//! interfaz: al cerrar la aplicación se perdía la skin, la carpeta de salida,
//! el modelo elegido y los 21 interruptores del randomizer.
//!
//! QUÉ NO SE GUARDA AQUÍ
//! ---------------------
//! **Las API keys no se persisten.** Guardarlas en un fichero de texto plano
//! sería un retroceso de seguridad, y el sitio correcto es el llavero del
//! sistema (crate `keyring`, que usa Secret Service en Linux, Keychain en
//! macOS y Credential Manager en Windows). Queda pendiente de decidir.
//!
//! UBICACIÓN
//! ---------
//! Se usa el directorio de configuración estándar de cada plataforma, vía la
//! crate `dirs`:
//!   Linux   → ~/.config/batch-image-generator/config.json
//!   macOS   → ~/Library/Application Support/batch-image-generator/config.json
//!   Windows → %APPDATA%\batch-image-generator\config.json

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const APP_DIR: &str = "batch-image-generator";
const FILE_NAME: &str = "config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// 0 = Oscuro · 1 = Claro · 2 = Cyberpunk
    pub skin: i32,
    pub output_folder: String,
    pub model_index: i32,
    pub resolution_idx: i32,
    pub interval_secs: i32,
    pub current_mode: i32,
    pub theme_index: i32,
    pub i2i_mode_index: i32,
    pub rand_active: bool,
    /// Super Randomizer: sortea las categorías en cada generación.
    pub super_rand_active: bool,
    pub checks: Checks,

    /// Banco de prompts: hasta 5 textos guardados por el usuario.
    ///
    /// Se guarda siempre con 5 posiciones (las vacías, como cadena vacía)
    /// para que el índice de la ranura sea estable entre arranques.
    pub prompts: Vec<String>,
    /// Ranura seleccionada en el desplegable (0-4).
    pub prompt_slot: i32,
    /// Elegir una de las ranuras guardadas al azar en cada generación.
    pub prompt_random: bool,

    /// Notificaciones de escritorio.
    pub notify_enabled: bool,
    pub notify_success: bool,
    pub notify_timeout: bool,
    pub notify_policy: bool,
    pub notify_other: bool,
}

/// Número de ranuras del banco de prompts.
pub const PROMPT_SLOTS: usize = 5;

/// Estado de los interruptores del randomizer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Checks {
    pub nails: bool,
    pub orient: bool,
    pub expression: bool,
    pub outfit: bool,
    pub legwear: bool,
    pub environment: bool,
    pub atmosphere: bool,
    pub pose: bool,
    pub lighting: bool,
    pub camera: bool,
    pub rare: bool,
    pub accessories: bool,
    pub makeup: bool,
    pub body_type: bool,
    pub age_vibe: bool,
    pub color_palette: bool,
    pub time_of_day: bool,
    pub weather: bool,
    pub bg_props: bool,
    pub material: bool,
    pub motion: bool,
    pub curated: bool,
    pub auto_b: bool,
}

impl Default for Checks {
    fn default() -> Self {
        Self {
            nails: true,
            orient: true,
            auto_b: true,
            expression: false,
            outfit: false,
            legwear: false,
            environment: false,
            atmosphere: false,
            pose: false,
            lighting: false,
            camera: false,
            rare: false,
            accessories: false,
            makeup: false,
            body_type: false,
            age_vibe: false,
            color_palette: false,
            time_of_day: false,
            weather: false,
            bg_props: false,
            material: false,
            motion: false,
            curated: false,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            skin: 0,
            output_folder: default_output_folder(),
            model_index: 0,
            resolution_idx: 0,
            interval_secs: 60,
            current_mode: 0,
            theme_index: 0,
            i2i_mode_index: 0,
            rand_active: false,
            super_rand_active: false,
            checks: Checks::default(),
            prompts: vec![String::new(); PROMPT_SLOTS],
            prompt_slot: 0,
            prompt_random: false,
            notify_enabled: false,
            // Los fallos vienen activados y el éxito no: en Burst serían
            // cientos de avisos.
            notify_success: false,
            notify_timeout: true,
            notify_policy: true,
            notify_other: true,
        }
    }
}

fn default_output_folder() -> String {
    dirs::home_dir()
        .map(|h| h.join("batch_images").to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Ruta del fichero de configuración, si la plataforma expone una.
pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join(APP_DIR).join(FILE_NAME))
}

impl Config {
    /// Carga la configuración. Ante cualquier problema (no existe, está
    /// corrupta, no hay permisos) devuelve los valores por defecto: nunca
    /// impide arrancar la aplicación.
    pub fn load() -> Self {
        let Some(path) = config_path() else {
            return Self::default();
        };
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Self::default();
        };
        match serde_json::from_str::<Config>(&text) {
            Ok(mut cfg) => {
                // Una configuración de una versión anterior puede traer menos
                // ranuras —o ninguna—; se normaliza a 5 para que el índice
                // del desplegable nunca se salga de rango.
                cfg.prompts.resize(PROMPT_SLOTS, String::new());
                cfg.prompts.truncate(PROMPT_SLOTS);
                cfg
            }
            Err(e) => {
                eprintln!(
                    "[config] {} ilegible ({e}); se usan valores por defecto.",
                    path.display()
                );
                Self::default()
            }
        }
    }

    /// Guarda la configuración. Devuelve `Err` con un mensaje legible, pero
    /// quien llama puede ignorarlo: no poder guardar preferencias no es
    /// motivo para interrumpir el trabajo del usuario.
    pub fn save(&self) -> Result<(), String> {
        let path = config_path().ok_or("No se pudo determinar el directorio de configuración.")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Error creando {}: {e}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Error serializando configuración: {e}"))?;
        std::fs::write(&path, json).map_err(|e| format!("Error escribiendo {}: {e}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn los_valores_por_defecto_son_razonables() {
        let c = Config::default();
        assert_eq!(c.skin, 0);
        assert_eq!(c.interval_secs, 60);
        assert!(c.checks.nails);
        assert!(c.checks.auto_b);
    }

    #[test]
    fn ida_y_vuelta_por_json() {
        let mut c = Config {
            skin: 2,
            interval_secs: 45,
            ..Config::default()
        };
        c.checks.weather = true;
        let text = serde_json::to_string(&c).unwrap();
        let back: Config = serde_json::from_str(&text).unwrap();
        assert_eq!(back.skin, 2);
        assert_eq!(back.interval_secs, 45);
        assert!(back.checks.weather);
    }

    /// Un fichero de una versión anterior, al que le faltan campos, debe
    /// cargarse rellenando el resto con los valores por defecto.
    #[test]
    fn tolera_configuraciones_parciales() {
        let back: Config = serde_json::from_str(r#"{"skin": 1}"#).unwrap();
        assert_eq!(back.skin, 1);
        assert_eq!(back.interval_secs, 60);
        assert!(back.checks.nails);
    }

    /// El Super Randomizer sobrescribe las casillas en cada generación. Lo
    /// que se guarda debe ser la selección **manual** del usuario, no la
    /// última tirada aleatoria.
    #[test]
    fn el_super_randomizer_se_guarda_sin_pisar_la_seleccion_manual() {
        let manual = Checks { nails: true, orient: true, pose: true, ..Checks::default() };
        let c = Config {
            super_rand_active: true,
            checks: manual.clone(),
            ..Config::default()
        };
        let back: Config = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert!(back.super_rand_active);
        assert!(back.checks.nails && back.checks.orient && back.checks.pose);
        assert!(!back.checks.weather, "no debe colarse nada del sorteo");
    }

    /// Una configuración de la 2.4.0 no tiene el campo: debe cargarse con el
    /// modo apagado, no fallar.
    #[test]
    fn una_config_antigua_arranca_con_el_super_randomizer_apagado() {
        let back: Config = serde_json::from_str(r#"{"skin": 1, "rand_active": true}"#).unwrap();
        assert!(!back.super_rand_active);
        assert!(back.rand_active);
    }

    #[test]
    fn tolera_json_con_campos_desconocidos() {
        let back: Config =
            serde_json::from_str(r#"{"skin": 2, "campo_futuro": 123}"#).unwrap();
        assert_eq!(back.skin, 2);
    }

    #[test]
    fn el_banco_de_prompts_se_normaliza_a_cinco_ranuras() {
        let c = Config::default();
        assert_eq!(c.prompts.len(), PROMPT_SLOTS);
        assert!(c.prompts.iter().all(|p| p.is_empty()));
    }

    #[test]
    fn una_config_con_menos_ranuras_se_completa() {
        let mut c: Config = serde_json::from_str(r#"{"prompts":["uno","dos"]}"#).unwrap();
        c.prompts.resize(PROMPT_SLOTS, String::new());
        assert_eq!(c.prompts.len(), PROMPT_SLOTS);
        assert_eq!(c.prompts[0], "uno");
        assert_eq!(c.prompts[4], "");
    }

    #[test]
    fn las_notificaciones_vienen_apagadas_pero_con_los_fallos_marcados() {
        let c = Config::default();
        assert!(!c.notify_enabled, "no deben activarse sin que el usuario lo pida");
        assert!(!c.notify_success, "el éxito en Burst serían cientos de avisos");
        assert!(c.notify_timeout && c.notify_policy && c.notify_other);
    }

    #[test]
    fn el_banco_sobrevive_a_la_ida_y_vuelta_por_json() {
        let mut c = Config::default();
        c.prompts[2] = "un prompt con acentos: ñáé 🎨".to_string();
        c.prompt_random = true;
        c.notify_enabled = true;
        let back: Config = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(back.prompts[2], "un prompt con acentos: ñáé 🎨");
        assert!(back.prompt_random);
        assert!(back.notify_enabled);
    }
}
