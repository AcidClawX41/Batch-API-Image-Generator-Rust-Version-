# Batch API Image Generator (Rust Version)

Desktop batch image generator built with Rust + Slint. It generates AI images at configurable intervals with customizable prompt randomization (presets, orientation, lighting, camera, outfits, etc).

## Supported providers
- xAI Grok Imagine (`grok-imagine-image`, `grok-imagine-image-pro`)
- Google Gemini Image / Nano Banana (`gemini-2.5-flash-image`, `gemini-3-pro-image-preview`)
- OpenAI Image (`gpt-image-1.5`, `gpt-image-1`, `gpt-image-1-mini`, `dall-e-3` legacy)

## Notes
- The app uses provider-compatible image generation endpoints that return `b64_json` image payloads.
- Output images are written to the selected output folder with a provider-specific filename prefix.