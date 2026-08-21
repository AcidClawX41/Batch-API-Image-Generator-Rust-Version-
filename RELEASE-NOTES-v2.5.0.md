## ⚡ Batch API Image Generator v2.5.0

The release that makes the app usable outside a dark desktop, adds a fifth
provider, and fixes the crashes nobody had noticed yet.

### 🎨 Three switchable skins — Dark · Light · Cyberpunk

The app was unreadable under a light Ubuntu theme: `std-widgets.slint` takes its
palette from the desktop. Slint's `Palette` global only lets you write
`color-scheme` at runtime — its colours are read-only — so three skins required
a custom widget set. Now the look is identical on Windows, macOS and Linux, and
switchable from the header.

### ☁️ Kie.AI — 11 new models

GPT Image 2, Nano Banana 2 / Pro / 2 Lite, Seedream 4.0 / 5.0 Pro / 5.0 Lite,
Grok Imagine, Flux.2 Pro and Qwen Image 3.0 — text-to-image and image-to-image.
**51 models across 5 providers.**

### 🎰 Super Randomizer

Instead of ticking the boxes yourself, each generation draws a random number of
categories (1 to all 21) and a random subset of them. It sits next to the manual
mode, it does not replace it — and your manual selection is snapshotted, so
turning it off gives you back exactly the combination you had.

### 📝 Prompt bank

Save up to 5 prompts and optionally have one picked at random per generation.
The Randomizer and Super Randomizer then apply on top of whichever was drawn.

### 🔔 Desktop notifications

XDG/D-Bus on Linux (Wayland and XWayland alike), WinRT toasts on Windows,
`UNUserNotificationCenter` on macOS. Per-event switches for success, server
timeout and content-policy rejection — success off by default, because in a long
Burst it would be hundreds of alerts.

Diagnose them without opening the window:

```bash
./xai-imagine-generator --test-notificacion
```

### 💾 Preferences that survive a restart

Skin, output folder, model, interval, every randomizer switch, the prompt bank
and the notification settings.

> API keys are deliberately **not** persisted. Writing them to a plain-text
> `config.json` would be a security regression; the system keyring is the right
> place and is still pending.

---

### 🏗 One table instead of three copies

Until v2.4 the endpoint and the reference-image field name were guessed from the
model identifier with a cascade of `contains()` / `ends_with()` checks, and the
same information was duplicated by hand in three places that nothing kept in
sync. It broke — the `// ← was missing` comment next to `ends_with("/edit-fast")`
was the scar.

Kie.AI made it untenable: its models use **three different names** for the same
field (`input_urls`, `image_urls`, `image_input`) with no rule derivable from the
identifier.

Every model is now declared once in `src/models.rs`. The dropdown labels, the
routing and the UI hints all come from that table, so the window cannot show one
model and send another.

---

### 🐛 Fixes

- **Four UTF-8 panics.** `&s[..n]` on a multibyte boundary crashed the app on any
  accented prompt.
- **Burst overwrote images** generated within the same second.
- **Stop did not cancel the in-flight request.** The app said STOPPED and a
  billed image appeared minutes later.
- **WAN 2.2 and Qwen Image Edit** sent `images` (array) where the provider
  requires `image` (single) — HTTP 400 on every request. Verified model by model
  against the provider docs.
- **A CDN hiccup threw away an already-paid generation.** The download now
  retries transient failures (timeout, dropped connection, 5xx, 429) and does not
  retry 404/403 on a signed URL. If it gives up, the image URL stays in the log
  so you can rescue it.
- **Notification failures were invisible.** They went to stderr, which nobody
  sees when launching from the desktop, and the log said "sent" merely because
  the thread had started. It now reports what actually happened.
- **The randomizer grid had no fixed columns** — `horizontal-stretch` only
  distributes surplus space, so every checkbox took its own text width.
- **The prompt box was nearly uneditable**: an overlaying `TouchArea` swallowed
  clicks, so the caret could not be placed.
- **The log grew O(n²)** and was never trimmed.
- **One HTTP client** is now shared instead of one per request; regexes compile
  once.

---

### 📦 Downloads

| Platform | File |
|---|---|
| Linux x86-64 | `batch-image-generator-linux-x86_64` |
| Windows x86-64 | `batch-image-generator-windows-x86_64.exe` |
| macOS Apple Silicon | `batch-image-generator-macos-arm64` |
| macOS Intel | `batch-image-generator-macos-x86_64` |
| Desktop entry + notification guide | `batch-image-generator-packaging.tar.gz` |

On Linux and macOS, `chmod +x` the binary before running it. GNOME users: install
the `.desktop` file from the packaging archive — without it GNOME does not know
which app a notification belongs to and may send it straight to the tray instead
of showing a banner.

---

### ⏭ Known limitations

- **Seed is fixed at `-1`.** A good image out of a Burst is not reproducible.
  Saving the prompt and seed alongside each image is the top item for v2.6.
- **API keys are not persisted** — three keys to re-enter each run, until the
  keyring lands.
- Kie.AI's own worker occasionally returns `code 524` (generate task timeout).
  That one is on their side; the app now reports it distinctly.

**Full changelog:** [`CHANGELOG-v2.5.0.md`](CHANGELOG-v2.5.0.md)
