//! randomizer.rs — Prompt randomization engine.
//!
//! Mode A: Modify/inject into an existing user prompt.
//! Mode B: Generate a complete prompt from scratch using pools.

use rand::seq::SliceRandom;
use rand::Rng;
use regex::Regex;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::pools;

static ORIENTATION_INDEX: AtomicUsize = AtomicUsize::new(0);

// ─── Helpers ───

fn pick<'a>(pool: &'a [&'a str]) -> &'a str {
    let mut rng = rand::thread_rng();
    pool.choose(&mut rng).unwrap()
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
}

impl Default for ModifyOptions {
    fn default() -> Self {
        Self {
            do_nails: false, do_orientation: false, do_expression: false,
            do_outfit: false, do_legwear: false, do_environment: false,
            do_atmosphere: false, do_pose: false, do_lighting: false,
            do_camera: false, do_rare: false,
        }
    }
}

pub fn modify_prompt(prompt: &str, opts: &ModifyOptions) -> String {
    let mut base = prompt.trim_end().to_string();
    let mut injections: Vec<String> = Vec::new();

    // REPLACE: Nail / claw color
    if opts.do_nails {
        let color = pick(pools::NAIL_COLORS);
        let nail_re = Regex::new(
            r"(?i)(?:shiny |glossy |matte |metallic |bright |dark |deep |hot |bubblegum |ruby |blood |rose |chrome |pearl |fiery |neon |midnight |emerald )*(?:black|red|pink|yellow|gold|silver|white|blue|green|purple|crimson|carmine|cherry|orange|grey|gray)(?:\s+(?:metallic|glossy|matte))?(\s+(?:nails|claws|toenails|nails and toenails|nails and claws|claws and toenails))"
        ).unwrap();
        if nail_re.is_match(&base) {
            base = nail_re.replace_all(&base, |caps: &regex::Captures| {
                format!("{}{}", color, &caps[1])
            }).to_string();
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

        let orient_re = Regex::new(
            r#"(?i)"(?:Widescreen|Vertical|Cinematic|Tall|Portrait|Landscape|wide|tall)[^"]*""#
        ).unwrap();
        if orient_re.is_match(&base) {
            base = orient_re.replace(&base, orient).to_string();
        } else {
            injections.push(orient.to_string());
        }
    }

    // REPLACE: Expression
    if opts.do_expression {
        let expr = pick(pools::EXPRESSIONS);
        let expr_re = Regex::new(
            r"(?i)(a\s+(?:confident|cold|playful|mischievous|melancholic|soft|defiant|fierce|dreamy|emotionally|teasing|proud)[^,.]*)"
        ).unwrap();
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
        format!("{}\n{}", base, injections.join(" "))
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

    // Character
    parts.push(format!(
        "The character has {}, {}, and {}.",
        pick(pools::HAIR_COLORS), pick(pools::HAIR_STYLES), pick(pools::EYE_COLORS)
    ));
    parts.push(format!("She has {}.", pick(expr_pool)));

    // Skin
    let skin = pick_n(pools::SKIN_DETAILS, 2);
    parts.push(format!("{}.", skin.join(", ")));

    // Outfit
    parts.push(format!(
        "She is wearing {}, {}, {}.",
        pick(outfit_pool), pick(pools::LEGWEAR), pick(pools::FABRIC_DETAILS)
    ));

    // Nails
    parts.push(format!(
        "Her hands feature {} {}.",
        pick(pools::NAIL_COLORS), pick(pools::NAIL_STYLES)
    ));
    parts.push(format!("Pose detail: {}.", pick(pools::HAND_POSES)));

    // Pose
    parts.push(format!(
        "She is {}, {}.",
        pick(pools::POSES), pick(pools::ACTION_DETAILS)
    ));

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
