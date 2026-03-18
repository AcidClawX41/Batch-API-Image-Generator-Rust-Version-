//! pools.rs — Randomizable content pools and theme presets.
//! Add, remove, or edit entries freely.

// ─── CHARACTER ───

pub const HAIR_COLORS: &[&str] = &[
    "fiery red hair", "ginger hair", "strawberry blonde hair",
    "platinum blonde hair", "ash blonde hair", "jet black hair",
    "raven black hair", "dark brown hair", "chestnut brown hair",
    "silver hair", "white hair", "pastel pink hair", "rose pink hair",
    "lavender hair", "violet hair", "ice blue hair", "deep blue hair",
    "emerald green hair", "teal hair", "two-tone split dyed hair",
    "ombre hair", "gradient dyed hair",
];

pub const HAIR_STYLES: &[&str] = &[
    "long straight hair", "long layered hair", "messy shoulder-length hair",
    "short bob haircut", "hime cut", "high ponytail", "side ponytail",
    "twin tails", "loose wavy hair", "curly hair", "braided hair",
    "single braid over the shoulder", "double braids", "ahoge",
    "windswept hair", "wet hair strands framing the face",
];

pub const EYE_COLORS: &[&str] = &[
    "striking blue eyes", "emerald green eyes", "golden amber eyes",
    "violet eyes", "ruby red eyes", "pink eyes", "silver-gray eyes",
    "heterochromia eyes", "deep brown eyes", "icy cyan eyes",
];

pub const EXPRESSIONS: &[&str] = &[
    "a confident expression", "a cold and intense stare", "a playful smirk",
    "a mischievous grin", "a melancholic gaze", "a soft shy expression",
    "a defiant look", "a fierce determined expression", "a dreamy faraway look",
    "an emotionally overwhelmed expression", "a teasing expression",
    "a proud and untouchable expression",
];

pub const SKIN_DETAILS: &[&str] = &[
    "soft skin shading", "subtle blush on the cheeks",
    "light freckles across the nose", "smooth anime-inspired complexion",
    "slightly flushed ears", "soft glossy lips",
];

// ─── NAILS / HANDS ───

pub const NAIL_COLORS: &[&str] = &[
    "glossy black", "ruby crimson", "deep carmine red", "hot pink",
    "bubblegum pink", "bright yellow", "metallic gold", "silver metallic",
    "dark purple", "midnight blue", "emerald green", "glossy white",
    "blood red", "rose gold", "chrome silver", "neon pink",
    "dark cherry", "matte black", "pearl white", "fiery orange",
];

pub const NAIL_STYLES: &[&str] = &[
    "short polished nails", "long almond-shaped nails",
    "long stiletto nails", "sharp pointed nails",
    "glass-like glossy nails", "mirror-finish chrome nails",
    "subtle pearl-coated nails", "slightly claw-like nails",
    "salon-perfect manicure",
];

pub const HAND_POSES: &[&str] = &[
    "elegant fingers lightly touching the face",
    "one hand pulling at the collar",
    "fingers brushing through the hair",
    "one hand resting on the wall",
    "hands clasped behind the back",
    "one hand extended toward the viewer",
    "fingers gripping fabric softly",
    "nails catching the light",
];

// ─── OUTFITS ───

pub const OUTFITS: &[&str] = &[
    "a Japanese school uniform with a black ribbon",
    "a sailor-style school uniform",
    "a fitted office outfit with a blazer",
    "a gothic black dress with lace details",
    "a casual oversized sweater outfit",
    "a turtleneck and pleated skirt combination",
    "a streetwear-inspired layered outfit",
    "a tactical futuristic bodysuit",
    "a fantasy-inspired dress with ornate trim",
    "a shrine maiden inspired outfit",
    "a refined winter coat outfit",
    "a sporty jacket with short skirt",
];

pub const LEGWEAR: &[&str] = &[
    "black thigh-high stockings", "semi-transparent tights",
    "white thigh-high socks", "patterned stockings",
    "black knee-high socks", "bare legs",
    "ribbed tights", "lace-trim thigh-highs",
];

pub const FABRIC_DETAILS: &[&str] = &[
    "with realistic fabric folds", "with subtle satin reflections",
    "with textured wool details", "with soft cotton texture",
    "with leather-like accents", "with metallic trims",
    "with delicate lace elements",
];

// ─── POSES ───

pub const POSES: &[&str] = &[
    "standing with a strong confident posture",
    "leaning against a wall", "sitting by a window",
    "walking forward with attitude", "turning back over the shoulder",
    "kneeling gracefully", "resting on a staircase",
    "half-crouched in a tense pose",
    "standing in the wind with hair flowing",
    "holding still in a quiet intimate moment",
    "mid-step as if caught in motion",
];

pub const ACTION_DETAILS: &[&str] = &[
    "with clothing and hair subtly moving",
    "with a hand raised near the lips",
    "with a tense shoulder line",
    "with a slight tilt of the head",
    "with a subtle shift of balance",
    "with expressive body language",
    "captured in a fleeting candid instant",
];

// ─── ENVIRONMENTS ───

pub const ENVIRONMENTS: &[&str] = &[
    "inside a dim classroom after sunset",
    "on a rainy city street at night",
    "in a neon-lit alley",
    "on a rooftop overlooking the city",
    "inside a quiet train carriage",
    "near a window with soft curtains moving",
    "inside a luxury modern apartment",
    "in an abandoned industrial corridor",
    "inside a traditional Japanese room",
    "in a moonlit garden",
    "in a cyberpunk downtown district",
    "inside a library with warm ambient light",
];

pub const ATMOSPHERIC_DETAILS: &[&str] = &[
    "with floating dust particles visible in the light",
    "with light rain in the background",
    "with drifting petals in the air",
    "with faint smoke or mist",
    "with reflective puddles on the ground",
    "with soft bokeh lights behind her",
    "with subtle motion blur in the environment",
    "with glowing signs in the distance",
];

// ─── LIGHTING ───

pub const LIGHTING_MOODS: &[&str] = &[
    "dramatic lighting and deep shadows",
    "soft golden hour lighting",
    "moody neon-lit atmosphere",
    "harsh cinematic spotlight",
    "ethereal backlit glow",
    "dark and brooding shadows with rim lighting",
    "warm sunset tones",
    "cool blue moonlight ambiance",
    "soft studio portrait lighting",
    "rainy night reflections with colored highlights",
    "diffused overcast daylight",
    "strong side-lighting for a sculpted face",
];

// ─── CAMERA / COMPOSITION ───

pub const ORIENTATIONS: &[&str] = &[
    "widescreen composition", "vertical portrait composition",
    "cinematic widescreen frame", "tall portrait framing",
    "close portrait crop",
];

pub const CAMERA_ANGLES: &[&str] = &[
    "close-up shot", "low angle shot", "dynamic perspective",
    "wide angle shot", "top-down shot", "over-the-shoulder perspective",
    "medium full-body shot", "tight facial portrait",
    "three-quarter body shot",
];

pub const LENS_STYLES: &[&str] = &[
    "85mm portrait lens look", "35mm cinematic lens look",
    "50mm natural perspective", "slight fisheye distortion",
    "compressed telephoto perspective",
];

pub const COMPOSITION_DETAILS: &[&str] = &[
    "rule of thirds composition",
    "strong foreground-background separation",
    "shallow depth of field", "sharp focus on the eyes",
    "dynamic diagonal framing", "negative space around the subject",
    "subject centered with symmetrical framing",
];

// ─── STYLE / RENDER ───

pub const ART_STYLES: &[&str] = &[
    "anime-style illustration with realistic rendering",
    "high-end semi-realistic anime art",
    "cinematic anime realism",
    "polished character illustration",
    "premium visual novel style artwork",
    "detailed key visual style",
    "stylized anime portrait with realistic textures",
];

pub const QUALITY_TAGS: &[&str] = &[
    "high detail", "ultra detailed", "sharp focus", "clean linework",
    "beautiful color grading", "highly detailed eyes",
    "carefully rendered hair strands", "refined shading",
    "professional composition",
];

// ─── RARE DETAILS ───

pub const RARE_DETAILS: &[&str] = &[
    "a tiny beauty mark under one eye",
    "smudged eyeliner for a slightly messy look",
    "subtle fang-like canine teeth visible in the smile",
    "a faint scar detail near the collarbone",
    "slightly runny mascara after the rain",
    "a translucent hair ornament catching the light",
    "fingerprints visible on glass nearby",
    "a reflection of city lights in the eyes",
    "a loose ribbon fluttering in the wind",
    "one stocking slightly wrinkled for realism",
    "a delicate ear cuff accessory",
    "a small bandage on one finger",
];

pub const ULTRA_RARE_DETAILS: &[&str] = &[
    "a surreal chromatic aberration effect around bright highlights",
    "wet pavement reflections mirroring the subject",
    "the scene framed through a cracked glass surface",
    "a subtle double-exposure feeling in the background",
    "faint holographic screen reflections across the face",
    "wind carrying paper fragments through the frame",
];

// ─── THEME PRESETS ───

pub struct ThemePreset {
    pub key: &'static str,
    pub label: &'static str,
    pub base: &'static str,
    pub outfits: Option<&'static [&'static str]>,
    pub expressions: Option<&'static [&'static str]>,
    pub environments: Option<&'static [&'static str]>,
}

const TSUNDERE_OUTFITS: &[&str] = &[
    "a Japanese school uniform with a black ribbon",
    "a sailor-style school uniform",
];
const TSUNDERE_EXPRS: &[&str] = &[
    "a defiant look", "a proud and untouchable expression", "a teasing expression",
];
const TSUNDERE_ENVS: &[&str] = &[
    "inside a dim classroom after sunset",
    "on a rooftop overlooking the city",
    "near a window with soft curtains moving",
];

const CYBER_OUTFITS: &[&str] = &[
    "a tactical futuristic bodysuit",
    "a streetwear-inspired layered outfit",
];
const CYBER_EXPRS: &[&str] = &[
    "a cold and intense stare", "a fierce determined expression",
];
const CYBER_ENVS: &[&str] = &[
    "in a neon-lit alley", "in a cyberpunk downtown district",
];

const GOTHIC_OUTFITS: &[&str] = &[
    "a gothic black dress with lace details",
    "a refined winter coat outfit",
];
const GOTHIC_EXPRS: &[&str] = &[
    "a melancholic gaze", "a confident expression",
];
const GOTHIC_ENVS: &[&str] = &[
    "inside a library with warm ambient light",
    "in a moonlit garden",
    "inside a traditional Japanese room",
];

const MIKO_OUTFITS: &[&str] = &[
    "a shrine maiden inspired outfit",
    "a fantasy-inspired dress with ornate trim",
];
const MIKO_EXPRS: &[&str] = &[
    "a soft shy expression", "a dreamy faraway look",
];
const MIKO_ENVS: &[&str] = &[
    "inside a traditional Japanese room", "in a moonlit garden",
];

const CASUAL_OUTFITS: &[&str] = &[
    "a casual oversized sweater outfit",
    "a turtleneck and pleated skirt combination",
    "a sporty jacket with short skirt",
];
const CASUAL_EXPRS: &[&str] = &[
    "a playful smirk", "a mischievous grin", "a confident expression",
];
const CASUAL_ENVS: &[&str] = &[
    "on a rainy city street at night",
    "inside a quiet train carriage",
    "inside a luxury modern apartment",
];

pub const THEME_PRESETS: &[ThemePreset] = &[
    ThemePreset {
        key: "tsundere_school", label: "Tsundere School",
        base: "A portrait of an original anime girl character with a fiery and proud personality",
        outfits: Some(TSUNDERE_OUTFITS), expressions: Some(TSUNDERE_EXPRS),
        environments: Some(TSUNDERE_ENVS),
    },
    ThemePreset {
        key: "cyberpunk", label: "Cyberpunk",
        base: "A portrait of an original futuristic anime girl character",
        outfits: Some(CYBER_OUTFITS), expressions: Some(CYBER_EXPRS),
        environments: Some(CYBER_ENVS),
    },
    ThemePreset {
        key: "gothic", label: "Gothic",
        base: "A portrait of an original gothic anime girl character",
        outfits: Some(GOTHIC_OUTFITS), expressions: Some(GOTHIC_EXPRS),
        environments: Some(GOTHIC_ENVS),
    },
    ThemePreset {
        key: "miko_shrine", label: "Miko / Shrine",
        base: "A portrait of an original anime shrine maiden character",
        outfits: Some(MIKO_OUTFITS), expressions: Some(MIKO_EXPRS),
        environments: Some(MIKO_ENVS),
    },
    ThemePreset {
        key: "casual_modern", label: "Casual Modern",
        base: "A portrait of an original anime girl character in a modern urban setting",
        outfits: Some(CASUAL_OUTFITS), expressions: Some(CASUAL_EXPRS),
        environments: Some(CASUAL_ENVS),
    },
    ThemePreset {
        key: "full_random", label: "🎲 Full Random",
        base: "A portrait of an original anime girl character",
        outfits: None, expressions: None, environments: None,
    },
];

// ─── CURATED COMBOS ───

pub struct CuratedCombo {
    pub environment: &'static str,
    pub lighting: &'static str,
    pub atmosphere: &'static str,
    pub camera: &'static str,
    pub lens: &'static str,
}

pub const CURATED_COMBOS: &[CuratedCombo] = &[
    CuratedCombo {
        environment: "on a rainy city street at night",
        lighting: "moody neon-lit atmosphere",
        atmosphere: "with reflective puddles on the ground",
        camera: "three-quarter body shot",
        lens: "35mm cinematic lens look",
    },
    CuratedCombo {
        environment: "on a rooftop overlooking the city",
        lighting: "cool blue moonlight ambiance",
        atmosphere: "with soft bokeh lights behind her",
        camera: "low angle shot",
        lens: "50mm natural perspective",
    },
    CuratedCombo {
        environment: "inside a library with warm ambient light",
        lighting: "dramatic lighting and deep shadows",
        atmosphere: "with floating dust particles visible in the light",
        camera: "close-up shot",
        lens: "85mm portrait lens look",
    },
    CuratedCombo {
        environment: "inside a quiet train carriage",
        lighting: "diffused overcast daylight",
        atmosphere: "with subtle motion blur in the environment",
        camera: "medium full-body shot",
        lens: "50mm natural perspective",
    },
    CuratedCombo {
        environment: "in a moonlit garden",
        lighting: "ethereal backlit glow",
        atmosphere: "with drifting petals in the air",
        camera: "dynamic perspective",
        lens: "85mm portrait lens look",
    },
];
