# ⚡ Batch API Image Generator v2.4.0 (Rust + Slint)

Desktop batch image generator built with **Rust + Slint**. Generates AI images at configurable intervals with a powerful prompt randomizer engine, advanced **Image-to-Image** conditioning with up to **5 reference images**, and **Burst Generation** for continuous non-stop generation. Supports **42 models** across 4 API providers.

---

## Supported Providers & Models

### xAI (direct API)
| Model | ID |
|---|---|
| Grok Imagine Image | `grok-imagine-image` |
| Grok Imagine Image Quality | `grok-imagine-image-quality` |

### Google (direct API)
| Model | ID |
|---|---|
| Gemini 2.5 Flash Image | `gemini-2.5-flash-image` |
| Gemini 3 Pro Image | `gemini-3-pro-image-preview` |

### OpenAI (direct API)
| Model | ID |
|---|---|
| GPT Image 1.5 | `gpt-image-1.5` |
| GPT Image 1 | `gpt-image-1` |
| GPT Image 1 Mini | `gpt-image-1-mini` |
| DALL-E 3 (legacy) | `dall-e-3` |

### WaveSpeed.ai (unified API — 31 models)

#### Text-to-Image
| Model | Family |
|---|---|
| `wavespeed-ai/flux-2-max/text-to-image` | Flux 2 |
| `wavespeed-ai/flux-2-dev/text-to-image` | Flux 2 |
| `wavespeed-ai/flux-2-flash/text-to-image` | Flux 2 |
| `wavespeed-ai/flux-2-flex/text-to-image` | Flux 2 |
| `wavespeed-ai/flux-kontext-max/text-to-image` | Flux Kontext |
| `wavespeed-ai/flux-kontext-pro/text-to-image` | Flux Kontext |
| `bytedance/seedream-v5.0-lite/text-to-image` | ByteDance |
| `bytedance/seedream-v4.5/text-to-image` | ByteDance |
| `bytedance/dreamina-v3.1/text-to-image` | ByteDance |
| `google/nano-banana-2/text-to-image` | Google |
| `google/nano-banana-pro/text-to-image` | Google |
| `alibaba/wan-2.6/text-to-image` | Alibaba (T2I only) |
| `alibaba/wan-2.7/text-to-image` | Alibaba |
| `openai/gpt-image-2/text-to-image` | OpenAI via WaveSpeed |
| `wavespeed-ai/qwen-image-2.0-pro/text-to-image` | Alibaba |
| `kwaivgi/kling-image-o3/text-to-image` | Kuaishou |
| `x-ai/grok-2-image` | xAI |
| `x-ai/grok-imagine-image/text-to-image` | xAI |
| `wavespeed-ai/flux-kontext-max/multi` | Flux Kontext Multi |
| `wavespeed-ai/flux-kontext-pro/multi` | Flux Kontext Multi |
| `wavespeed-ai/uno/text-to-image` | UNO |
| `wavespeed-ai/wan-2.2/image-to-image` | WAN |

#### Image-to-Image / Edit (I2I native endpoints)
| Model | Field | Max Refs | Notes |
|---|---|---|---|
| `x-ai/grok-imagine-image/edit` | `image` singular | 1 | xAI images/edits format |
| `google/nano-banana-2/edit` | `images` array | 5 | |
| `google/nano-banana-2/edit-fast` | `images` array | 5 | Faster variant |
| `google/nano-banana-pro/edit` | `images` array | 5 | |
| `bytedance/seedream-v5.0-lite/edit` | `images` array | 5 | |
| `bytedance/seedream-v4.5/edit` | `images` array | 5 | |
| `wavespeed-ai/qwen-image/edit` | `images` array | 5 | |
| `wavespeed-ai/flux-2-klein-4b/edit` | `images` array | 5 | |
| `alibaba/wan-2.7/image-edit` | `images` array | 5 | |
| `openai/gpt-image-2/edit` | `images` array | 5 | resolution 1k/2k/4k |

---

## Features

### ⚡ Burst Generation *(New in v2.4)*
Fires generation requests continuously with zero interval — as soon as one image finishes, the next starts immediately. Perfect for rapid iteration. Uses the same model, prompt and reference images as the configured loop.

### 🖼 Image-to-Image with up to 5 Reference Images *(Expanded in v2.4)*
- Upload up to **5 reference images** (PNG/JPG/WEBP) to guide generation or directly edit existing pictures
- Choose between **Style Reference** (loose guidance) and **Direct Edit** (strict adherence to content)
- Each slot has a distinct color indicator (🔵 blue, 🟢 green, 🟣 purple, 🟠 orange, 🟡 yellow)
- Dynamic routing sends the correct field (`image` singular vs `images` array) per model's API contract
- Encodes images directly as Base64 data URIs — no separate upload step

### 🖥 Output Resolution Selector *(New in v2.4)*
Choose output resolution for all WaveSpeed models directly from the UI: **Auto** (model default), **1k (1024)**, **2k (2048)**, or **4k (4096)**. For GPT Image 2 Edit the `resolution` field is sent natively; for all other WaveSpeed models the `size` field is mapped accordingly.

### 🔄 Batch Loop
Configurable interval (10–600 seconds) between generations. Status countdown shows remaining time. Three control buttons:
- **▶ Iniciar Loop** — start the batch loop at configured interval
- **⚡⚡ Burst** — continuous generation with no pause
- **■ Detener** — stop either mode

### 🎲 Prompt Randomizer
- **Mode A** — Write your own prompt; the randomizer injects additional details (outfit, lighting, pose, camera, nails, expression, etc.)
- **Mode B** — Fully auto-generated prompts from curated pools and theme presets
- **20+ injectable categories**: nails, orientation, expression, outfit, legwear, environment, atmosphere, pose, lighting, camera, accessories, makeup, body type, age vibe, color palette, time of day, weather, background props, material, motion, rare details
- **Smart prompt reinforcement** — preserves the user's base prompt subject so AI models don't ignore it
- **Advanced Regex Engine** — strict word-boundary algorithms target precise descriptors without overwriting the original prompt's core content

### 🔑 Dual API Key Support
Separate keys for xAI/Google/OpenAI (direct) and WaveSpeed.ai (unified gateway).

### 🖥 Cross-Platform
Windows, macOS, Linux.

---

## Build

```bash
cargo build --release
```

The binary will be at `target/release/xai-imagine-generator` (or `.exe` on Windows).

### Requirements

- Rust 1.70+
- An API key from at least one provider:
  - [xAI Console](https://console.x.ai) for Grok models (direct)
  - [Google AI Studio](https://aistudio.google.com) for Gemini / Nano Banana (direct)
  - [OpenAI Platform](https://platform.openai.com) for GPT Image / DALL-E (direct)
  - [WaveSpeed.ai](https://wavespeed.ai) for Flux, Seedream, WAN, Grok via WaveSpeed, and 30+ more models

---

## Architecture

```
src/
  main.rs        — UI wiring, callbacks, countdown timer, generation loop, burst mode
  api.rs         — Multi-provider HTTP client (OpenAI-compat + WaveSpeed async polling)
  randomizer.rs  — Prompt modification engine (Mode A: inject, Mode B: generate)
  pools.rs       — Randomization pools (styles, outfits, environments, etc.)
ui/
  main.slint     — Slint UI layout (dark theme, 5 I2I slots, dual control rows)
```

---

## Technical Notes

### WaveSpeed I2I Dynamic Routing
The app inspects each model's identifier and routes accordingly:

| Condition | Endpoint | Field |
|---|---|---|
| Already ends with `/edit`, `/edit-fast`, `/image-edit`, `/image-to-image`, `/multi` | used as-is | per model |
| `flux-kontext` (single) | base URL | `image` singular |
| `flux-kontext/multi` or `uno` | base URL | `images` array |
| `wan-2.7` (T2I base) | `+ /image-to-image` | `images` array |
| `grok-imagine-image/edit` | used as-is | `image` singular (xAI format) |
| `nano-banana`, `seedream`, `qwen`, `/edit` family | used as-is | `images` array |
| Other flux | `+ /image-to-image` | `images` array |

> **Note:** `alibaba/wan-2.6` is T2I only and has no I2I endpoint. Use `alibaba/wan-2.7/image-edit` for reference image generation with Alibaba WAN.

### WaveSpeed Polling Flow
POST to submit → sync mode waits for completion → download image from CDN URL. Falls back to polling if sync mode times out (180s).

### Resolution Defaults
- ByteDance (Seedream, Dreamina): **1920×1920** (minimum required by provider)
- All other WaveSpeed models: **1024×1024**

### xAI / Google / OpenAI
Use OpenAI-compatible endpoints returning `b64_json`. Output saved directly from the response.

---

## Changelog

### v2.4.0
- **New:** GPT Image 2 Text-to-Image (`openai/gpt-image-2/text-to-image`) via WaveSpeed added to model catalog.
- **New:** GPT Image 2 Edit (`openai/gpt-image-2/edit`) via WaveSpeed — I2I with `images` array + native `resolution` field.
- **New:** Output resolution selector in UI — Auto / 1k / 2k / 4k. Applies to all WaveSpeed models.
- **New:** `resolution_to_size` helper maps UI selection to `size` string (`1024*1024` / `2048*2048` / `4096*4096`).
- **New:** `output_resolution: &str` parameter threaded through `generate_image`, `generate_wavespeed`, `generate_wavespeed_i2i`.
- **Fix:** GPT Image 2 T2I uses a schema without `size` field (only `prompt` + `aspect_ratio`); handled with a dedicated branch.

### v2.3.0
- **Fix:** `x-ai/grok-imagine-image/edit` was sending `images` (array) — API requires `image` singular (xAI images/edits format). Added `is_grok_edit` flag.
- **Fix:** `google/nano-banana-2/edit-fast` (`/edit-fast` suffix) was not included in `needs_images_array` — fell through to singular field. Added `ends_with("/edit-fast")` to the condition.
- **Fix:** `alibaba/wan-2.6` with I2I active tried to hit non-existent `/image-to-image` endpoint. Now returns a descriptive error directing to `wan-2.7/image-edit`.
- **Fix:** Models already carrying `/edit` or `/edit-fast` in their name were having the suffix appended again (`…/edit/edit`). Added both to the "already I2I" early-return check.
- **New:** Burst Generation mode — continuous generation with zero pause between requests.
- **New:** 9 native I2I/edit endpoints added (Grok Edit, Nano Banana 2 Edit/Edit-Fast/Pro, Seedream v5 Lite/v4.5 Edit, Qwen Edit, Flux 2 Klein Edit, WAN 2.7 Image-Edit).
- **New:** UNO, WAN 2.2 I2I, Flux Kontext Multi, Kling O3, Qwen Image 2.0 Pro, Dreamina v3.1, WAN 2.7 added to MODEL_CATALOG (40 total).
- **New:** Reference image slots expanded from 2 to 5 (slots 3–5 with purple/orange/yellow indicators).
- **New:** `generate_image` signature changed to `ref_images: &[(String, String)]` for variable-count multi-image support.
- **New:** WaveSpeed body construction switched from typed structs to `serde_json::json!` macro for dynamic field routing without lifetime complexity.
- **UI:** Two-row control button layout — Loop + Burst + Stop on top row; Generate 1 on bottom row.
- **UI:** Dynamic I2I info note adapts message based on number of loaded reference images.

### v2.2.0
- Universal Image-to-Image routing across all WaveSpeed endpoints.
- Stable asynchronous countdown batch loop (no duplicate simultaneous requests).
- Advanced Regex Engine for the Randomizer (word-boundary checks for nails, expressions, etc.).
- Expanded `do_nails` vocabulary (3D, glossy, claw-like with structural shape + color).

### v2.1.0
- Image-to-Image conditioning (2 reference image slots).
- Style Reference vs Direct Edit modes.
- Base64 encoding pipeline.

### v2.0.0
- Initial Rust + Slint rewrite.
- Multi-provider support (xAI, Google, OpenAI, WaveSpeed).
- Prompt Randomizer (Mode A & B).
- Configurable batch loop.

---

## License

See [LICENSE](LICENSE).
