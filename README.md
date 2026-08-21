# ⚡ Batch API Image Generator v2.5.0 (Rust + Slint)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![CI](https://github.com/AcidClawX41/Batch-API-Image-Generator-Rust-Version-/actions/workflows/ci.yml/badge.svg)](https://github.com/AcidClawX41/Batch-API-Image-Generator-Rust-Version-/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/AcidClawX41/Batch-API-Image-Generator-Rust-Version-)](https://github.com/AcidClawX41/Batch-API-Image-Generator-Rust-Version-/releases)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue)

Desktop batch image generator built with **Rust + Slint**. Generates AI images at configurable intervals with a powerful prompt randomizer engine, advanced **Image-to-Image** conditioning with up to **5 reference images**, and **Burst Generation** for continuous non-stop generation. Supports **51 models** across 5 API providers (xAI, Google, OpenAI, WaveSpeed, Kie.AI).

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
| `wavespeed-ai/wan-2.2/image-to-image` | WAN (I2I only, `image` singular) |

#### Image-to-Image / Edit (I2I native endpoints)
| Model | Field | Max Refs | Notes |
|---|---|---|---|
| `x-ai/grok-imagine-image/edit` | `image` singular | 1 | xAI images/edits format |
| `google/nano-banana-2/edit` | `images` array | 5 | |
| `google/nano-banana-2/edit-fast` | `images` array | 5 | Faster variant |
| `google/nano-banana-pro/edit` | `images` array | 5 | |
| `bytedance/seedream-v5.0-lite/edit` | `images` array | 5 | |
| `bytedance/seedream-v4.5/edit` | `images` array | 5 | |
| `wavespeed-ai/qwen-image/edit` | `image` singular | 1 | Verified against provider docs |
| `wavespeed-ai/flux-2-klein-4b/edit` | `images` array | 5 | |
| `alibaba/wan-2.7/image-edit` | `images` array | 3 | Provider caps at 3 |
| `openai/gpt-image-2/edit` | `images` array | 5 | resolution 1k/2k/4k |

### Kie.AI (unified API — 11 models) *(New in v2.5)*

Upload → `createTask` → poll `recordInfo`. Every model's aspect-ratio and size
fields are verified against Kie.AI's own docs: sending `"auto"` to a model that
does not accept it returns HTTP 500, and omitting a required field returns
`This field is required`. Both were hit in real use and both are now pinned.

| Model | T2I | I2I | Reference field | Max refs |
|---|---|---|---|---|
| GPT Image 2 | ✅ | ✅ | `input_urls` | 5 |
| Nano Banana 2 | ✅ | ✅ | `image_input` | 5 |
| Nano Banana Pro | ✅ | ✅ | `image_input` | 5 |
| Nano Banana 2 Lite | ✅ | ✅ | `image_urls` | 5 |
| Seedream 4.0 | ✅ | — | — | — |
| Seedream 4.0 Edit | — | ✅ | `image_urls` | 5 |
| Seedream 5.0 Pro | ✅ | ✅ | `image_urls` | 5 |
| Seedream 5.0 Lite | ✅ | — | — | — |
| Grok Imagine | ✅ | ✅ | `image_urls` | 2 |
| Flux.2 Pro | ✅ | ✅ | `input_urls` | 5 |
| Qwen Image 3.0 | ✅ | ✅ | `image_urls` | 3 |

---

## Features

### 🎨 Three Switchable Skins *(New in v2.5)*
**Dark · Light · Cyberpunk**, switchable from the header and identical on
Windows, macOS and Linux.

`std-widgets.slint` takes its palette from the desktop, which is why the app was
unreadable under a light Ubuntu theme. Slint's `Palette` global only lets you
write `color-scheme` at runtime — its colours are read-only — so three skins
required a custom widget set. `ui/widgets.slint` provides it; every colour
derives from a single `Theme.skin` integer.

### 🎰 Super Randomizer *(New in v2.5)*
Instead of picking the boxes yourself, each generation draws a **random number
of categories** (1 to all 21) and a random subset of them. It does not replace
the manual mode — it sits next to it.

Your manual selection is snapshotted while Super is on, so turning it off gives
you back the combination you had, and closing the app never saves a random draw
as if it were your choice.

### 📝 Prompt Bank *(New in v2.5)*
Save up to **5 prompts** and optionally have one picked at random per
generation. The Randomizer and Super Randomizer then apply on top of whichever
was drawn.

### 🔔 Desktop Notifications *(New in v2.5)*
XDG/D-Bus on Linux (Wayland and XWayland alike — the transport is D-Bus, not the
display server), WinRT toasts on Windows, `UNUserNotificationCenter` on macOS.

Per-event switches for success, server timeout and content-policy rejection.
Success is **off** by default: in a long Burst it would be hundreds of alerts.

Diagnostics without opening the window:

```bash
./xai-imagine-generator --test-notification
```

Prints the real result of the call plus the environment (session D-Bus, which
daemon answers, desktop, session type) — because "I see no notifications" has
two very different causes and the window cannot tell them apart.

### 💾 Persisted Preferences *(New in v2.5)*
Skin, output folder, model, interval, mode, every randomizer switch, the prompt
bank and the notification settings survive a restart. Stored in the platform's
config directory (`~/.config/batch-image-generator/config.json` on Linux).

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
- **▶ Iniciar Loop** *(Start Loop)* — start the batch loop at the configured interval
- **⚡⚡ Burst** — continuous generation with no pause
- **■ Detener** *(Stop)* — stop either mode

> The UI ships with Spanish labels; this README glosses them in English where
> they appear.

### 🎲 Prompt Randomizer
- **Mode A** — Write your own prompt; the randomizer injects additional details (outfit, lighting, pose, camera, nails, expression, etc.)
- **Mode B** — Fully auto-generated prompts from curated pools and theme presets
- **20+ injectable categories**: nails, orientation, expression, outfit, legwear, environment, atmosphere, pose, lighting, camera, accessories, makeup, body type, age vibe, color palette, time of day, weather, background props, material, motion, rare details
- **Smart prompt reinforcement** — preserves the user's base prompt subject so AI models don't ignore it
- **Advanced Regex Engine** — strict word-boundary algorithms target precise descriptors without overwriting the original prompt's core content

### 🔑 Three API Key Slots
Separate keys for xAI/Google/OpenAI (direct), WaveSpeed.ai and Kie.AI.

> Keys are **not** written to `config.json`. Storing them in plain text would be
> a security regression; the right place is the system keyring, which is
> pending. Everything else is persisted.

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
  - [Kie.AI](https://kie.ai) for GPT Image 2, Nano Banana, Seedream, Flux.2 and Qwen

Prebuilt binaries for Windows, macOS (Intel and Apple Silicon) and Linux are
attached to every [release](https://github.com/AcidClawX41/Batch-API-Image-Generator-Rust-Version-/releases).
On Linux and macOS, `chmod +x` the binary before running it.

---

## Architecture

```
src/
  main.rs        — UI wiring, callbacks, countdown timer, generation loop, burst mode
  models.rs      — single source of truth: 51 models, provider, endpoints, image field
  api.rs         — multi-provider HTTP client (OpenAI-compat · WaveSpeed · Kie.AI)
  randomizer.rs  — prompt engine (Mode A: inject · Mode B: generate · Super Randomizer)
  pools.rs       — randomization pools (styles, outfits, environments, …)
  config.rs      — preferences persisted across runs (skin, folder, model, switches)
  notify.rs      — cross-platform desktop notifications
  util.rs        — char-safe string helpers (byte slicing used to panic on UTF-8)
ui/
  theme.slint    — three skins; every colour derives from one `Theme.skin` int
  widgets.slint  — custom widget set, so the look does not follow the OS palette
  main.slint     — window layout
```

---

## Technical Notes

### Model Table — single source of truth *(rewritten in v2.5)*

Until v2.4 the endpoint and the name of the reference-image field were guessed
from the model identifier with a cascade of `contains()` / `ends_with()`
checks, and the same information was duplicated by hand in three places
(`MODEL_CATALOG`, the Slint dropdown, and the routing code). Nothing kept them
in sync, and it broke: the `// ← was missing` comment next to
`ends_with("/edit-fast")` is the scar.

Kie.AI made it untenable — its models use **three different names** for the
same reference-image field (`input_urls`, `image_urls`, `image_input`) with no
rule derivable from the identifier.

Every model is now declared once in `src/models.rs`:

```rust
ModelSpec {
    label: "WaveSpeed — WAN 2.2",
    provider: ImageProvider::WaveSpeed,
    t2i_id: None,
    i2i_id: Some("wavespeed-ai/wan-2.2/image-to-image"),
    ref_field: RefField::WsImage,   // verified against provider docs
    max_refs: 1,
    aspect: "",
    size_style: SizeStyle::WsSize,
}
```

The dropdown labels, the routing and the UI hints all come from that table, so
the window cannot show one model and send another. Tests pin the exact request
body per model, so a wrong field is caught in half a millisecond instead of by
an HTTP 400 that costs credits.

### WaveSpeed Polling Flow
POST to submit → sync mode waits for completion → download image from CDN URL. Falls back to polling if sync mode times out (180s).

### Resolution Defaults
- ByteDance (Seedream, Dreamina): **1920×1920** (minimum required by provider)
- All other WaveSpeed models: **1024×1024**

### xAI / Google / OpenAI
Use OpenAI-compatible endpoints returning `b64_json`. Output saved directly from the response.

---

## Changelog

### v2.5.0
- **New:** three switchable skins (Dark / Light / Cyberpunk) on a custom widget
  set, so the look no longer follows the desktop palette.
- **New:** Kie.AI as a fifth provider — 11 models, upload → `createTask` → poll.
- **New:** Super Randomizer — a random number of categories drawn per generation.
- **New:** prompt bank of 5 slots with optional random pick per generation.
- **New:** cross-platform desktop notifications, with `--test-notificacion` for
  diagnosing them from a terminal.
- **New:** preferences persisted between runs.
- **Refactor:** `src/models.rs` is now the single source of truth for all 51
  models; the `contains()` cascade that guessed endpoints and field names is
  gone, and with it the class of bug it kept producing.
- **Fix:** four UTF-8 panics — `&s[..n]` on a multibyte boundary crashed the app
  on any accented prompt.
- **Fix:** Burst overwrote images generated within the same second.
- **Fix:** Stop did not cancel the in-flight request — the app said STOPPED and
  a billed image appeared minutes later.
- **Fix:** WAN 2.2 and Qwen Image Edit sent `images` (array) where the provider
  requires `image` (single) — HTTP 400 on every request.
- **Fix:** a CDN hiccup threw away an already-paid generation; the download now
  retries transient failures and keeps the URL if it gives up.
- **Fix:** the log grew O(n²) and was never trimmed.
- **Fix:** one HTTP client is shared instead of one per request; regexes compile
  once.
- **CI:** builds and tests on Linux, Windows and macOS on every push; the release
  workflow no longer uploads two assets with the same name.

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

Released under the **MIT License** — see [LICENSE](LICENSE) for the full text.

```
Copyright (c) 2026 Eric .V
```

You are free to use, copy, modify, merge, publish, distribute, sublicense and
sell copies of this software, provided the copyright notice and the permission
notice are included in all copies or substantial portions of it. The software is
provided "as is", without warranty of any kind.

### Third-party services

The MIT licence covers **this application only**. Images you generate are
subject to the terms of whichever provider produced them — xAI, Google, OpenAI,
WaveSpeed.ai or Kie.AI — including their content policies and their rules on
commercial use. Check each provider's terms before publishing or selling
generated output.

---

## Author

**Eric Valls Gramunt** — [@AcidClawX41](https://github.com/AcidClawX41)

*The copyright line above is quoted verbatim from [`LICENSE`](LICENSE). If you
want your full name there, edit `LICENSE` and this section will need to match.*
