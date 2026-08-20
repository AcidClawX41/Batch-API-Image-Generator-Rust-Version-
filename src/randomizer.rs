//! randomizer.rs — Prompt randomization engine.
//!
//! Mode A: Modify/inject into an existing user prompt.
//! Mode B: Generate a complete prompt from scratch using pools.

use rand::seq::SliceRandom;
use rand::Rng;
use regex::Regex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::LazyLock;

use crate::pools;

static ORIENTATION_INDEX: AtomicUsize = AtomicUsize::new(0);

// ─── Expresiones regulares ───
//
// Se compilan una sola vez. En la 2.4.0 se construían con `Regex::new(...)`
// dentro de `modify_prompt`, es decir en cada generación y en cada pulsación
// del interruptor del randomizer. No son patrones triviales: el de las uñas
// tiene medio centenar de alternativas repetidas en dos grupos anidados.

static NAIL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:extremely|very|super|extra|long|short|sharp|pointed|curved|claw-like|claw|acrylic|natural|shiny|glossy|matte|metallic|bright|dark|deep|hot|bubblegum|ruby|blood|rose|chrome|pearl|fiery|neon|midnight|emerald|black|red|pink|yellow|gold|silver|white|blue|green|purple|crimson|carmine|cherry|orange|grey|gray|painted|polished|manicured|chipped|broken|glittery|sparkly|huge|massive|and|with)\s+(?:(?:extremely|very|super|extra|long|short|sharp|pointed|curved|claw-like|claw|acrylic|natural|shiny|glossy|matte|metallic|bright|dark|deep|hot|bubblegum|ruby|blood|rose|chrome|pearl|fiery|neon|midnight|emerald|black|red|pink|yellow|gold|silver|white|blue|green|purple|crimson|carmine|cherry|orange|grey|gray|painted|polished|manicured|chipped|broken|glittery|sparkly|huge|massive|and|with)\s+)*(?:nails|claws|toenails|talons)\b",
    )
    .expect("NAIL_RE es un literal válido")
});

static ORIENT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)"(?:Widescreen|Vertical|Cinematic|Tall|Portrait|Landscape|wide|tall)[^"]*""#)
        .expect("ORIENT_RE es un literal válido")
});

static EXPR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:a\s+|an\s+)?(?:and\s+|with\s+|very\s+|extremely\s+|slightly\s+|confident\s+|cold\s+|playful\s+|mischievous\s+|melancholic\s+|soft\s+|defiant\s+|fierce\s+|dreamy\s+|emotionally\s+|teasing\s+|proud\s+|aggressive\s+|determined\s+|shy\s+|intense\s+|happy\s+|sad\s+|angry\s+|serious\s+|seductive\s+|smug\s+|evil\s+|gentle\s+|warm\s+|sweet\s+|crazy\s+|insane\s+|wild\s+|blank\s+|stoic\s+|neutral\s+|arrogant\s+|cocky\s+)*(?:expression|stare|smirk|grin|gaze|look|smile)\b",
    )
    .expect("EXPR_RE es un literal válido")
});



// ─── Helpers ───

/// Elige un elemento al azar del pool.
///
/// Devuelve `""` con un pool vacío en lugar de entrar en pánico: hoy los
/// pools son constantes del binario, pero en cuanto se externalicen a JSON
/// un fichero mal editado tiraría la aplicación.
fn pick<'a>(pool: &'a [&'a str]) -> &'a str {
    let mut rng = rand::thread_rng();
    pool.choose(&mut rng).copied().unwrap_or("")
}

fn pick_n<'a>(pool: &'a [&'a str], n: usize) -> Vec<&'a str> {
    let mut rng = rand::thread_rng();
    let k = n.min(pool.len());
    pool.choose_multiple(&mut rng, k).cloned().collect()
}

fn maybe<'a>(pool: &'a [&'a str], probability: f64) -> Option<&'a str> {
    let mut rng = rand::thread_rng();
    if rng.gen_bool(probability.clamp(0.0, 1.0)) {
        Some(pick(pool))
    } else {
        None
    }
}

// ─── Mode A: Modify / inject into existing prompt ───

#[derive(Default)]
pub struct ModifyOptions {
    pub do_nails: bool,
    pub do_orientation: bool,
    pub do_expression: bool,
    pub do_outfit: bool,
    pub do_legwear: bool,
    pub do_environment: bool,
    pub do_atmosphere: bool,
    pub do_pose: bool,
    pub do_lighting: bool,
    pub do_camera: bool,
    pub do_rare: bool,
    // New pools
    pub do_accessories: bool,
    pub do_makeup: bool,
    pub do_body_type: bool,
    pub do_age_vibe: bool,
    pub do_color_palette: bool,
    pub do_time_of_day: bool,
    pub do_weather: bool,
    pub do_bg_props: bool,
    pub do_material: bool,
    pub do_motion: bool,
}



pub fn modify_prompt(prompt: &str, opts: &ModifyOptions) -> String {
    let mut base = prompt.trim_end().to_string();
    let mut injections: Vec<String> = Vec::new();

    // REPLACE: Nail / claw color and style
    if opts.do_nails {
        let color = pick(pools::NAIL_COLORS);
        let style = pick(pools::NAIL_STYLES);
        let nail_re = &*NAIL_RE;
        if nail_re.is_match(&base) {
            base = nail_re.replace_all(&base, format!("{} {}", color, style).as_str()).to_string();
        } else {
            injections.push(format!("Her hands feature {} {}.", color, style));
        }
    }

    // REPLACE: Orientation
    if opts.do_orientation {
        let orientations = [
            "\"Widescreen picture\"",
            "\"Vertical picture\"",
            "\"Cinematic widescreen shot\"",
            "\"Tall portrait composition\"",
        ];
        let idx = ORIENTATION_INDEX.fetch_add(1, Ordering::Relaxed);
        let orient = orientations[idx % orientations.len()];

        let orient_re = &*ORIENT_RE;
        if orient_re.is_match(&base) {
            base = orient_re.replace(&base, orient).to_string();
        } else {
            injections.push(orient.to_string());
        }
    }

    // REPLACE: Expression
    if opts.do_expression {
        let expr = pick(pools::EXPRESSIONS);
        let expr_re = &*EXPR_RE;
        if expr_re.is_match(&base) {
            base = expr_re.replace(&base, expr).to_string();
        } else {
            injections.push(format!("She has {}.", expr));
        }
    }

    // INJECT: Outfit
    if opts.do_outfit {
        injections.push(format!(
            "She is wearing {}, {}.",
            pick(pools::OUTFITS), pick(pools::FABRIC_DETAILS)
        ));
    }

    // INJECT: Legwear
    if opts.do_legwear {
        injections.push(format!("Wearing {}.", pick(pools::LEGWEAR)));
    }

    // INJECT: Environment
    if opts.do_environment {
        injections.push(format!("The scene is set {}.", pick(pools::ENVIRONMENTS)));
    }

    // INJECT: Atmosphere
    if opts.do_atmosphere {
        injections.push(format!("Scene atmosphere: {}.", pick(pools::ATMOSPHERIC_DETAILS)));
    }

    // INJECT: Pose
    if opts.do_pose {
        injections.push(format!(
            "She is {}, {}.",
            pick(pools::POSES), pick(pools::ACTION_DETAILS)
        ));
    }

    // INJECT: Lighting
    if opts.do_lighting {
        injections.push(format!("Lighting: {}.", pick(pools::LIGHTING_MOODS)));
    }

    // INJECT: Camera
    if opts.do_camera {
        injections.push(format!(
            "{}, {}.",
            pick(pools::CAMERA_ANGLES), pick(pools::LENS_STYLES)
        ));
    }

    // INJECT: Accessories
    if opts.do_accessories {
        injections.push(format!("Wearing {}.", pick(pools::ACCESSORIES)));
    }

    // INJECT: Makeup
    if opts.do_makeup {
        injections.push(format!("Beauty detail: {}.", pick(pools::MAKEUP_DETAILS)));
    }

    // INJECT: Body type
    if opts.do_body_type {
        injections.push(format!("She has a {}.", pick(pools::BODY_TYPES)));
    }

    // INJECT: Age vibe
    if opts.do_age_vibe {
        injections.push(format!("Character is an {}.", pick(pools::AGE_VIBES)));
    }

    // INJECT: Color palette
    if opts.do_color_palette {
        injections.push(format!("Color grading: {}.", pick(pools::COLOR_PALETTES)));
    }

    // INJECT: Time of day
    if opts.do_time_of_day {
        injections.push(format!("The scene takes place {}.", pick(pools::TIMES_OF_DAY)));
    }

    // INJECT: Weather
    if opts.do_weather {
        injections.push(format!("Weather: {}.", pick(pools::WEATHER_CONDITIONS)));
    }

    // INJECT: Background props
    if opts.do_bg_props {
        injections.push(format!("Background elements: {}.", pick(pools::BACKGROUND_PROPS)));
    }

    // INJECT: Material emphasis
    if opts.do_material {
        injections.push(format!("Texture emphasis: {}.", pick(pools::MATERIAL_EMPHASIS)));
    }

    // INJECT: Motion
    if opts.do_motion {
        injections.push(format!("Motion detail: {}.", pick(pools::MOTION_DETAILS)));
    }

    // INJECT: Rare
    if opts.do_rare {
        if let Some(r) = maybe(pools::RARE_DETAILS, 0.5) {
            injections.push(format!("Extra detail: {}.", r));
        }
        if let Some(u) = maybe(pools::ULTRA_RARE_DETAILS, 0.2) {
            injections.push(format!("Cinematic touch: {}.", u));
        }
    }

    if injections.is_empty() {
        base
    } else {
        // Weave the base prompt with the injections so models don't ignore
        // the user's original intent.  Strategy:
        //   1. Lead with the full base prompt (sets the main subject).
        //   2. Append the randomized details as a comma-separated block that
        //      reads as elaboration, NOT as a replacement.
        //   3. Close with a short reinforcement of the core subject so the
        //      model keeps it top-of-mind.
        let detail_block = injections.join(" ");
        let reinforcement = extract_subject_hint(&base);
        if reinforcement.is_empty() {
            format!("{}\nAdditional details: {}", base, detail_block)
        } else {
            format!(
                "{}\nAdditional details: {}\nThe main subject remains: {}",
                base, detail_block, reinforcement
            )
        }
    }
}

/// Pull a short "subject hint" from the user's base prompt.
/// Grabs the first sentence (up to the first period, comma, or newline)
/// capped at 120 chars.  Returns empty string if the base is too short to
/// bother reinforcing.
fn extract_subject_hint(base: &str) -> String {
    if base.len() < 20 {
        return String::new();
    }
    // El `unwrap_or(base.len().min(120))` original cortaba por byte: si el
    // byte 120 caía dentro de un carácter multibyte (una `ñ`, un acento, un
    // emoji), `base[..end]` entraba en pánico.
    match base
        .char_indices()
        .find(|(i, c)| *i > 15 && (*c == '.' || *c == '\n'))
        .map(|(i, _)| i)
    {
        Some(end) => base[..end].trim().to_string(),
        None => crate::util::take_chars(base, 120).trim().to_string(),
    }
}

// ─── Mode B: Generate complete prompt from scratch ───

pub fn generate_full_prompt(preset_index: usize, use_curated: bool) -> String {
    let preset = pools::THEME_PRESETS.get(preset_index)
        .unwrap_or(&pools::THEME_PRESETS[pools::THEME_PRESETS.len() - 1]);

    let outfit_pool = preset.outfits.unwrap_or(pools::OUTFITS);
    let expr_pool = preset.expressions.unwrap_or(pools::EXPRESSIONS);
    let env_pool = preset.environments.unwrap_or(pools::ENVIRONMENTS);

    let mut parts: Vec<String> = Vec::new();

    parts.push(format!("{},", preset.base));
    parts.push(format!("{}.", pick(pools::ART_STYLES)));

    // Age / presence anchor
    parts.push(format!("Character is an {}.", pick(pools::AGE_VIBES)));

    // Body type
    if let Some(bt) = maybe(pools::BODY_TYPES, 0.6) {
        parts.push(format!("She has a {}.", bt));
    }

    // Character
    parts.push(format!(
        "The character has {}, {}, and {}.",
        pick(pools::HAIR_COLORS), pick(pools::HAIR_STYLES), pick(pools::EYE_COLORS)
    ));
    parts.push(format!("She has {}.", pick(expr_pool)));

    // Skin + makeup
    let skin = pick_n(pools::SKIN_DETAILS, 2);
    parts.push(format!("{}.", skin.join(", ")));
    if let Some(mk) = maybe(pools::MAKEUP_DETAILS, 0.55) {
        parts.push(format!("Beauty detail: {}.", mk));
    }

    // Accessories
    if let Some(acc) = maybe(pools::ACCESSORIES, 0.5) {
        parts.push(format!("Wearing {}.", acc));
    }

    // Outfit + material
    parts.push(format!(
        "She is wearing {}, {}, {}.",
        pick(outfit_pool), pick(pools::LEGWEAR), pick(pools::FABRIC_DETAILS)
    ));
    if let Some(mat) = maybe(pools::MATERIAL_EMPHASIS, 0.4) {
        parts.push(format!("Texture emphasis: {}.", mat));
    }

    // Nails
    parts.push(format!(
        "Her hands feature {} {}.",
        pick(pools::NAIL_COLORS), pick(pools::NAIL_STYLES)
    ));
    parts.push(format!("Pose detail: {}.", pick(pools::HAND_POSES)));

    // Pose + motion
    parts.push(format!(
        "She is {}, {}.",
        pick(pools::POSES), pick(pools::ACTION_DETAILS)
    ));
    if let Some(mot) = maybe(pools::MOTION_DETAILS, 0.45) {
        parts.push(format!("Motion detail: {}.", mot));
    }

    // Environment
    if use_curated && !pools::CURATED_COMBOS.is_empty() {
        let combo = {
            let mut rng = rand::thread_rng();
            &pools::CURATED_COMBOS[rng.gen_range(0..pools::CURATED_COMBOS.len())]
        };
        parts.push(format!("The scene is set {}, {}.", combo.environment, combo.atmosphere));
        parts.push(format!("Lighting: {}.", combo.lighting));
        parts.push(format!("{}, {}.", combo.camera, combo.lens));
    } else {
        parts.push(format!(
            "The scene is set {}, {}.",
            pick(env_pool), pick(pools::ATMOSPHERIC_DETAILS)
        ));
        parts.push(format!("Lighting: {}.", pick(pools::LIGHTING_MOODS)));
        parts.push(format!("{}, {}.", pick(pools::CAMERA_ANGLES), pick(pools::LENS_STYLES)));
    }

    // Time of day
    if let Some(tod) = maybe(pools::TIMES_OF_DAY, 0.5) {
        parts.push(format!("The scene takes place {}.", tod));
    }

    // Weather
    if let Some(wth) = maybe(pools::WEATHER_CONDITIONS, 0.4) {
        parts.push(format!("Weather: {}.", wth));
    }

    // Background props
    if let Some(bp) = maybe(pools::BACKGROUND_PROPS, 0.45) {
        parts.push(format!("Background elements: {}.", bp));
    }

    // Color palette
    if let Some(cp) = maybe(pools::COLOR_PALETTES, 0.5) {
        parts.push(format!("Color grading: {}.", cp));
    }

    // Composition
    parts.push(format!(
        "{}, {}.",
        pick(pools::ORIENTATIONS), pick(pools::COMPOSITION_DETAILS)
    ));

    // Quality
    let tags = pick_n(pools::QUALITY_TAGS, 4);
    parts.push(format!("{}.", tags.join(", ")));

    // Rare
    if let Some(r) = maybe(pools::RARE_DETAILS, 0.45) {
        parts.push(format!("Extra detail: {}.", r));
    }
    if let Some(u) = maybe(pools::ULTRA_RARE_DETAILS, 0.15) {
        parts.push(format!("Special cinematic touch: {}.", u));
    }

    parts.join(" ")
}

// ─── Super Randomizer ───────────────────────────────────────────────────
//
// En el modo normal es el usuario quien marca qué categorías se inyectan y
// esa elección vale para todas las generaciones. En Super Randomizer se
// sortea en **cada** generación cuántas categorías entran (entre 1 y todas)
// y cuáles son, de modo que un Burst largo no repite dos veces la misma
// combinación.

/// Nombres de las 21 categorías, en el orden en que aparecen en la interfaz.
/// Se usan para decir en el log qué salió sorteado.
pub const CATEGORY_NAMES: [&str; 21] = [
    "Uñas", "Orientación", "Expresión",
    "Ropa", "Medias", "Escenario", "Atmósfera",
    "Pose", "Luz", "Cámara", "Detalles raros",
    "Accesorios", "Maquillaje", "Silueta", "Edad",
    "Paleta color", "Hora del día", "Clima",
    "Props fondo", "Material", "Movimiento",
];

fn activar(o: &mut ModifyOptions, index: usize) {
    match index {
        0 => o.do_nails = true,
        1 => o.do_orientation = true,
        2 => o.do_expression = true,
        3 => o.do_outfit = true,
        4 => o.do_legwear = true,
        5 => o.do_environment = true,
        6 => o.do_atmosphere = true,
        7 => o.do_pose = true,
        8 => o.do_lighting = true,
        9 => o.do_camera = true,
        10 => o.do_rare = true,
        11 => o.do_accessories = true,
        12 => o.do_makeup = true,
        13 => o.do_body_type = true,
        14 => o.do_age_vibe = true,
        15 => o.do_color_palette = true,
        16 => o.do_time_of_day = true,
        17 => o.do_weather = true,
        18 => o.do_bg_props = true,
        19 => o.do_material = true,
        20 => o.do_motion = true,
        _ => {}
    }
}

/// Sortea un subconjunto de categorías para una generación.
///
/// El número de categorías es uniforme entre 1 y 21: así salen tanto tiradas
/// mínimas (una sola categoría, cambio sutil) como máximas (las 21, cambio
/// radical), pasando por todo lo de en medio. Nunca sale vacío, porque una
/// generación sin ninguna categoría sería idéntica al prompt base y el modo
/// no tendría efecto.
///
/// Devuelve también los nombres elegidos, en el orden de la interfaz, para
/// poder registrarlos.
pub fn random_options() -> (ModifyOptions, Vec<&'static str>) {
    let mut rng = rand::thread_rng();
    let total = CATEGORY_NAMES.len();
    let cuantas = rng.gen_range(1..=total);

    let mut indices: Vec<usize> = (0..total).collect();
    indices.shuffle(&mut rng);
    indices.truncate(cuantas);
    indices.sort_unstable();

    let mut opts = ModifyOptions::default();
    let mut nombres = Vec::with_capacity(cuantas);
    for &i in &indices {
        activar(&mut opts, i);
        nombres.push(CATEGORY_NAMES[i]);
    }
    (opts, nombres)
}

/// Los 21 interruptores en el orden de la interfaz, para volcarlos sobre las
/// casillas y que se vea qué ha tocado en cada generación.
pub fn options_as_flags(o: &ModifyOptions) -> [bool; 21] {
    [
        o.do_nails, o.do_orientation, o.do_expression,
        o.do_outfit, o.do_legwear, o.do_environment, o.do_atmosphere,
        o.do_pose, o.do_lighting, o.do_camera, o.do_rare,
        o.do_accessories, o.do_makeup, o.do_body_type, o.do_age_vibe,
        o.do_color_palette, o.do_time_of_day, o.do_weather,
        o.do_bg_props, o.do_material, o.do_motion,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn todas_activas() -> ModifyOptions {
        ModifyOptions {
            do_nails: true, do_orientation: true, do_expression: true,
            do_outfit: true, do_legwear: true, do_environment: true,
            do_atmosphere: true, do_pose: true, do_lighting: true,
            do_camera: true, do_rare: true, do_accessories: true,
            do_makeup: true, do_body_type: true, do_age_vibe: true,
            do_color_palette: true, do_time_of_day: true, do_weather: true,
            do_bg_props: true, do_material: true, do_motion: true,
        }
    }

    /// Regresión de H-02: un prompt con acentos o emojis en la posición
    /// equivocada hacía caer la aplicación en `extract_subject_hint`.
    #[test]
    fn modify_prompt_no_entra_en_panico_con_texto_multibyte() {
        let casos = [
            format!("{}ñ{}", "a".repeat(119), "b".repeat(80)),
            format!("{}🎨{}", "x".repeat(118), "y".repeat(50)),
            "Mujer pelirroja con expresión desafiante bajo la lluvia en Tokio de noche".to_string(),
            "ñ".repeat(300),
            "🎨🔥💀".repeat(60),
            String::new(),
            "corto".to_string(),
        ];
        for c in &casos {
            let _ = modify_prompt(c, &todas_activas());
            let _ = modify_prompt(c, &ModifyOptions::default());
        }
    }

    #[test]
    fn extract_subject_hint_es_seguro_en_cualquier_longitud() {
        let base = "Una mujer pelirroja 🔥 con expresión desafiante y uñas afiladas";
        for n in 0..base.chars().count() {
            let recorte: String = base.chars().take(n).collect();
            let _ = extract_subject_hint(&recorte);
        }
    }

    #[test]
    fn sin_opciones_el_prompt_se_devuelve_intacto() {
        let base = "Retrato de un personaje con acentos: ñáéíóú";
        assert_eq!(modify_prompt(base, &ModifyOptions::default()), base);
    }

    #[test]
    fn con_opciones_el_prompt_base_se_conserva() {
        let base = "Retrato de una mujer pelirroja en la lluvia";
        let out = modify_prompt(base, &todas_activas());
        assert!(out.starts_with(base), "el prompt del usuario debe encabezar el resultado");
        assert!(out.len() > base.len(), "debe haber añadido detalles");
    }

    #[test]
    fn generate_full_prompt_acepta_indices_fuera_de_rango() {
        for idx in [0usize, 1, 5, 99, usize::MAX] {
            let p = generate_full_prompt(idx, false);
            assert!(!p.is_empty());
        }
        let _ = generate_full_prompt(0, true);
    }

    #[test]
    fn pick_con_pool_vacio_no_entra_en_panico() {
        let vacio: &[&str] = &[];
        assert_eq!(pick(vacio), "");
    }

    #[test]
    fn el_super_randomizer_nunca_sortea_vacio() {
        for _ in 0..500 {
            let (_, nombres) = random_options();
            assert!(!nombres.is_empty(), "una tirada sin categorías no cambiaría nada");
            assert!(nombres.len() <= CATEGORY_NAMES.len());
        }
    }

    #[test]
    fn el_super_randomizer_llega_a_los_dos_extremos() {
        // Con 2000 tiradas uniformes sobre 1..=21 es prácticamente seguro ver
        // tanto el mínimo como el máximo.
        let mut vio_una = false;
        let mut vio_todas = false;
        for _ in 0..2000 {
            let (_, n) = random_options();
            if n.len() == 1 { vio_una = true; }
            if n.len() == CATEGORY_NAMES.len() { vio_todas = true; }
        }
        assert!(vio_una, "nunca sortea una sola categoría");
        assert!(vio_todas, "nunca sortea las 21");
    }

    #[test]
    fn los_nombres_sorteados_no_se_repiten_y_van_en_orden() {
        for _ in 0..200 {
            let (_, nombres) = random_options();
            let mut vistos = std::collections::HashSet::new();
            for n in &nombres {
                assert!(vistos.insert(*n), "categoría repetida: {n}");
            }
            let posiciones: Vec<usize> = nombres
                .iter()
                .map(|n| CATEGORY_NAMES.iter().position(|c| c == n).unwrap())
                .collect();
            let mut ordenadas = posiciones.clone();
            ordenadas.sort_unstable();
            assert_eq!(posiciones, ordenadas, "deben salir en el orden de la interfaz");
        }
    }

    /// Los nombres devueltos tienen que corresponder exactamente con los
    /// interruptores activados: si no, el log mentiría sobre lo que se envió.
    #[test]
    fn los_nombres_coinciden_con_los_interruptores() {
        for _ in 0..200 {
            let (opts, nombres) = random_options();
            let flags = options_as_flags(&opts);
            let activos: Vec<&str> = flags
                .iter()
                .enumerate()
                .filter(|(_, on)| **on)
                .map(|(i, _)| CATEGORY_NAMES[i])
                .collect();
            assert_eq!(activos, nombres);
        }
    }

    #[test]
    fn un_prompt_generado_con_opciones_sorteadas_cambia_el_texto() {
        let base = "Retrato de una mujer pelirroja";
        let (opts, _) = random_options();
        let out = modify_prompt(base, &opts);
        assert!(out.starts_with(base));
        assert!(out.len() > base.len(), "el sorteo debe añadir algo");
    }
}
