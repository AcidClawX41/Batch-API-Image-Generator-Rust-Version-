//! pools.rs — Randomizable content pools and theme presets.


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
    "short hair with a bang", "short hair with a bang and a wig",
];

pub const EYE_COLORS: &[&str] = &[
    "striking blue eyes", "emerald green eyes", "golden amber eyes",
    "violet eyes", "ruby red eyes", "pink eyes", "silver-gray eyes",
    "heterochromia eyes", "deep brown eyes", "icy cyan eyes",
    "neon pink eyes", "neon green eyes", "neon blue eyes",
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
    "holographic rainbow", "iridescent opal shimmer", "galaxy nebula purple",
    "neon cyan glitch", "electric venom green", "toxic slime lime",
    "unicorn horn pastel", "glow-in-the-dark radioactive", "psychedelic swirl magenta",
    "mermaid scale teal", "diamond dust sparkle", "lava crackle orange",
    "cyberpunk chrome magenta", "frosted arctic blue", "pearlescent sunset gradient",
    "witchy black glitter", "alien iridescent silver", "blood moon crimson crackle",
    "sparkling starfield navy", "matte holographic violet", "neon coral poison",
    "rainbow chrome flip", "glowing plasma pink", "glittery bubblegum galaxy",
    "metallic toxic yellow", "pearlized aurora borealis", "deep sea bioluminescent",
    "flamingo chrome pink", "void black with starlight", "candy apple holographic",
    "electric lavender shock", "sunset gradient chrome", "zombie slime green glow",
];

pub const NAIL_STYLES: &[&str] = &[
    "short polished nails", "long almond-shaped nails",
    "long stiletto nails", "sharp pointed nails",
    "glass-like glossy nails", "mirror-finish chrome nails",
    "subtle pearl-coated nails", "slightly claw-like nails",
    "salon-perfect manicure",
    "long coffin nails", "extra long dagger nails", "extreme stiletto talons",
    "ballerina shaped nails", "square blocky nails", "micro short square nails",
    "acrylic nails with rhinestones", "3D jewel-encrusted nails", "spiked gothic nails",
    "cracked and broken acrylics", "dripping wet-look nails", "glitter bomb overload",
    "holographic chrome tips", "matte black with gold inlays", "pearl and chain nails",
    "talon-like predator nails", "heart-shaped tips", "butterfly wing nails",
    "neon glowing edge nails", "french tip but reversed", "rainbow gradient coffin",
    "embedded crystal shards", "velvet matte texture", "wet glossy latex nails",
    "claw extension extreme", "festival glitter explosion", "demon horn nails",
];

pub const HAND_POSES: &[&str] = &[
    "elegant fingers lightly touching the face",
    "one hand pulling at the collar",
    "Paw Pose",
    "Claws Scratching Things",
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
    "a classic french maid outfit with white frills",
    "a sexy black latex catsuit with zippers",
    "a bunny girl leotard with fishnet stockings",
    "a pink lolita dress with huge bows",
    "a micro bikini armor fantasy set",
    "a transparent sheer babydoll nightie",
    "a cyberpunk neon bodysuit with glowing lines",
    "a nurse uniform with extreme short skirt",
    "a cheerleader outfit with pom-poms",
    "a shibari-inspired rope harness dress",
    "a wet white shirt and black panties combo",
    "a gothic lolita with layered petticoats",
    "a demon queen corset and thigh-high boots",
    "a school swimsuit with see-through fabric",
    "a leather dominatrix corset set",
    "a magical girl transformation outfit",
    "a kimono with exposed shoulders and slits",
    "a succubus lingerie with bat wings",
    "a ripped punk rock outfit with chains",
    "a shiny PVC police uniform",
    "a virgin killer sweater with nothing underneath",
    "a steampunk corset and goggles set",
    "a catgirl neko maid dress with bell collar",
    "a holographic future idol stage costume",
    "a bondage leather harness and micro skirt",
];

pub const LEGWEAR: &[&str] = &[
    "black thigh-high stockings", "semi-transparent tights",
    "white thigh-high socks", "patterned stockings",
    "black knee-high socks", "bare legs",
    "ribbed tights", "lace-trim thigh-highs",
    "fishnet thigh-high stockings", "glossy black latex thigh-highs",
    "white garter belt stockings", "rainbow striped over-knee socks",
    "ripped and torn fishnets", "sheer seamed pantyhose",
    "neon pink glowing thigh-highs", "cyberpunk LED-lined stockings",
    "bunny girl white fishnets with pom-poms", "french maid lace thigh-highs with bows",
    "wet-look shiny pantyhose", "black thigh-highs with red heart garters",
    "suspender belt and sheer stockings", "striped knee-high school socks",
    "transparent vinyl leg covers", "sparkly glitter bomb tights",
    "pastel rainbow thigh-high socks", "red demon latex legwear",
    "classic schoolgirl white knee-highs", "succubus chain fishnets",
    "armored strap thigh-highs", "dripping wet glossy stockings",
    "velvet matte black thigh-highs", "holographic chrome tights",
    "heart-patterned garter stockings",
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
    "seductive hand on hip with arched back",
    "peace sign with playful wink and tongue out",
    "lying on back with legs playfully in the air",
    "dynamic magical girl transformation pose",
    "leaning forward with cleavage emphasis",
    "sitting backwards on a chair teasingly",
    "low angle looking up with sultry gaze",
    "arms raised in idol victory pose",
    "kneeling with hands on thighs and back arched",
    "spinning with skirt flare and hair whip",
    "floating mid-air in zero gravity pose",
    "teasingly lifting skirt with one hand",
    "yoga flexible bridge pose on toes",
    "leaning over with hands on knees seductive",
    "dramatic hair flip mid-turn",
    "crouching like a catgirl ready to pounce",
    "lying on side with leg kick and wink",
    "standing with one leg raised high",
    "back view with over-shoulder seductive glance",
    "hands behind head with chest forward",
    "jumping with twintails flying and skirt up",
    "sitting on floor hugging knees shyly",
    "dynamic fighting stance with glowing aura",
    "reclining on elbows with legs crossed",
    "ahegao-ready arched back pose with tongue",
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
    "realistic anime-style illustration",
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

// ─── ACCESSORIES ───

pub const ACCESSORIES: &[&str] = &[
    "a black choker", "a silver chain necklace", "a delicate lace choker",
    "small hoop earrings", "long dangling earrings", "a pearl necklace",
    "thin-framed glasses", "stylish sunglasses", "a ribbon hair clip",
    "a flower hair ornament", "a jeweled hairpin", "a leather bracelet",
    "lace gloves", "arm warmers", "a decorative waist belt",
    "a velvet ribbon tied around the wrist", "a pendant with a gemstone",
    "fingerless leather gloves", "a beret", "a silk scarf around the neck",
];

// ─── MAKEUP / BEAUTY ───

pub const MAKEUP_DETAILS: &[&str] = &[
    "soft eyeliner", "smoky eye makeup", "winged eyeliner",
    "glittery eye shadow", "subtle glossy lipstick", "deep red lipstick",
    "soft pink lipstick", "light mascara", "defined lashes",
    "a soft blush gradient", "a polished beauty look",
    "slightly smudged eyeliner", "delicate lip gloss",
    "bold dramatic eye makeup", "natural no-makeup look",
];

// ─── BODY TYPE / SILHOUETTE ───

pub const BODY_TYPES: &[&str] = &[
    "slender build", "curvy silhouette", "athletic build",
    "graceful figure", "soft elegant proportions",
    "lean toned physique", "mature feminine silhouette",
    "thin build", "weak body",
];

// ─── AGE / PRESENCE ───

pub const AGE_VIBES: &[&str] = &[
    "adult woman", "young adult woman", "mature-looking anime woman",
    "confident adult character", "elegant grown woman",
    "youthful girl", "loli girl", "college-age woman",
    "early-twenties woman",
];

// ─── COLOR PALETTE ───

pub const COLOR_PALETTES: &[&str] = &[
    "warm red and gold palette", "cool blue and silver palette",
    "black and crimson palette", "soft pastel palette",
    "neon magenta and cyan palette", "muted cinematic palette",
    "deep violet and blue palette", "emerald and black palette",
    "sepia and amber tones", "monochrome with a single accent color",
    "warm autumn palette", "icy winter palette",
];

// ─── TIME OF DAY ───

pub const TIMES_OF_DAY: &[&str] = &[
    "at sunrise", "in the early morning", "at golden hour",
    "at sunset", "at blue hour", "late at night",
    "under moonlight", "before dawn", "at high noon",
    "during twilight",
];

// ─── WEATHER / CLIMATE ───

pub const WEATHER_CONDITIONS: &[&str] = &[
    "during light rain", "during heavy rain", "on a windy evening",
    "in humid summer air", "on a cold winter night", "in spring breeze",
    "in soft snowfall", "under stormy clouds", "in dense fog",
    "under a clear starry sky", "with cherry blossoms drifting in the air",
];

// ─── BACKGROUND PROPS ───

pub const BACKGROUND_PROPS: &[&str] = &[
    "glowing city signs", "paper lanterns", "bookshelves",
    "rain-soaked pavement", "flower petals scattered on the ground",
    "velvet curtains", "ornate mirrors", "candles in the background",
    "metal railings", "holographic displays",
    "train windows with reflections", "shoji screens",
    "neon vending machines", "stacked wooden crates",
    "a grand staircase in the background",
];

// ─── MATERIAL / TEXTURE ───

pub const MATERIAL_EMPHASIS: &[&str] = &[
    "silk textures", "velvet textures", "leather textures",
    "latex shine", "satin sheen", "matte fabric contrast",
    "metallic surfaces", "glass reflections", "wet surface reflections",
    "embroidered fabric details", "lace overlay textures",
    "denim texture details", "translucent fabric layers",
];

// ─── MOTION DETAILS ───

pub const MOTION_DETAILS: &[&str] = &[
    "hair flowing in the wind", "loose strands crossing the face",
    "fabric fluttering softly", "a ribbon moving in the breeze",
    "subtle motion in the skirt hem", "wind pushing the coat backward",
    "floating strands of hair catching the light",
    "earrings swaying gently", "scarf trailing behind her",
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
