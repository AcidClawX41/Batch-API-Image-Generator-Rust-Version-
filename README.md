# Batch API Image Generator v2.1 (Rust + Slint)

Desktop batch image generator built with **Rust + Slint**. Generates AI images at configurable intervals with a powerful prompt randomizer engine and advanced **Image-to-Image** conditioning. Supports **24 models** across 4 API providers.

## Supported Providers & Models

### xAI (direct API)
| Model | ID |
|---|---|
| Grok Imagine Image | `grok-imagine-image` |
| Grok Imagine Image Pro | `grok-imagine-image-pro` |

### Google (direct API)
| Model | ID |
|---|---|
| Gemini 2.5 Flash Image (Nano Banana) | `gemini-2.5-flash-image` |
| Gemini 3 Pro Image (Nano Banana Pro) | `gemini-3-pro-image-preview` |

### OpenAI (direct API)
| Model | ID |
|---|---|
| GPT Image 1.5 | `gpt-image-1.5` |
| GPT Image 1 | `gpt-image-1` |
| GPT Image 1 Mini | `gpt-image-1-mini` |
| DALL-E 3 (legacy) | `dall-e-3` |

### WaveSpeed.ai (unified API — Flux, Seedream, Grok, and more)
| Model | ID | Family |
|---|---|---|
| Flux 2 Max | `wavespeed-ai/flux-2-max/text-to-image` | Flux 2 |
| Flux 2 Dev | `wavespeed-ai/flux-2-dev/text-to-image` | Flux 2 |
| Flux 2 Flash | `wavespeed-ai/flux-2-flash/text-to-image` | Flux 2 |
| Flux 2 Flex | `wavespeed-ai/flux-2-flex/text-to-image` | Flux 2 |
| Flux Kontext Max | `wavespeed-ai/flux-kontext-max/text-to-image` | Flux Kontext |
| Flux Kontext Pro | `wavespeed-ai/flux-kontext-pro/text-to-image` | Flux Kontext |
| Seedream 5.0 Lite | `bytedance/seedream-v5.0-lite` | ByteDance |
| Seedream 4.5 | `bytedance/seedream-v4.5` | ByteDance |
| Nano Banana 2 | `google/nano-banana-2/text-to-image` | Google |
| Nano Banana Pro | `google/nano-banana-pro/text-to-image` | Google |
| WAN 2.6 | `alibaba/wan-2.6/text-to-image` | Alibaba |
| Dreamina 3.1 | `bytedance/dreamina-v3.1/text-to-image` | ByteDance |
| Qwen Image 2.0 Pro | `wavespeed-ai/qwen-image-2.0-pro/text-to-image` | Alibaba |
| Kling O3 | `kwaivgi/kling-image-o3/text-to-image` | Kuaishou |
| Grok 2 Image | `x-ai/grok-2-image` | xAI |
| Grok Imagine Image | `x-ai/grok-imagine-image-text-to-image` | xAI |

## Features

- **Image-to-Image (I2I) Conditioning (New in v2.1):** 
  - Upload reference images (PNG/JPG/WEBP) to guide generation styles or directly edit existing pictures.
  - Choose between **Style Reference** (loose guidance) and **Direct Edit** (strict adherence to content).
  - Encodes images directly into Base64 for seamless transmission to the supported APIs.
- **Two generation modes:**
  - **Mode A** — Write your own prompt + randomizer injects additional details (outfit, lighting, pose, camera, etc.)
  - **Mode B** — Fully auto-generated prompts from curated pools and theme presets
- **Prompt Randomizer** with 20+ injectable categories (nails, orientation, expression, outfit, legwear, environment, atmosphere, pose, lighting, camera, accessories, makeup, body type, age vibe, color palette, time of day, weather, background props, material, motion, rare details)
- **Batch loop** with configurable interval (10–600 seconds)
- **Dual API key support** — separate keys for xAI/Google/OpenAI and WaveSpeed.ai
- **Smart prompt reinforcement** — the randomizer preserves and reinforces the user's base prompt so AI models don't ignore the original subject
- **Cross-platform** — Windows, macOS, Linux

## Build

```bash
cargo build --release
```

The binary will be at `target/release/xai-imagine-generator` (or `.exe` on Windows).

## Requirements

- Rust 1.70+
- An API key from at least one provider:
  - [xAI Console](https://console.x.ai) for Grok models (direct)
  - [Google AI Studio](https://aistudio.google.com) for Gemini / Nano Banana (direct)
  - [OpenAI Platform](https://platform.openai.com) for GPT Image / DALL-E (direct)
  - [WaveSpeed.ai](https://wavespeed.ai) for Flux, Seedream, Grok via WaveSpeed, and 10+ more models

## Architecture

```
src/
  main.rs        — UI wiring, callbacks, countdown timer, generation loop
  api.rs         — Multi-provider HTTP client (OpenAI-compat + WaveSpeed async polling)
  randomizer.rs  — Prompt modification engine (Mode A: inject, Mode B: generate)
  pools.rs       — Randomization pools (styles, outfits, environments, etc.)
ui/
  main.slint     — Slint UI layout (dark theme, responsive)
```

## Notes

- xAI, Google, and OpenAI use OpenAI-compatible endpoints returning `b64_json`.
- WaveSpeed uses a different flow: POST to submit → sync mode waits for completion → download image from CDN URL. Falls back to polling if sync mode times out.
- **WaveSpeed I2I Routing (v2.1)**: The app dynamically detects model families and routes image reference requests to their respective endpoints (`/image-edit` for WAN, `/edit` with an `images[]` payload for Seedream/Qwen/Nano-Banana, and the base URL for Flux Kontext).
- Seedream and Dreamina models automatically use 1920x1920 resolution (minimum required by ByteDance). All other WaveSpeed models default to 1024x1024.
- Output images are saved to the selected folder with a provider-specific filename prefix and timestamp.

## License

See [LICENSE](LICENSE).
