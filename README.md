# 🐺 Wolf Screen Recorder

> **High-Performance, Hardware-Accelerated Screen Recorder & Snapshot Studio Engine for Linux, macOS, and Windows.**
>
> **100% Offline · Zero Telemetry · Absolute Privacy · Native Rust & Slint UI**

---

## 🛡️ 100% Privacy-First & Zero Telemetry Guarantee

Wolf Screen Recorder is built from the ground up on a **strict local-first, zero-trust privacy architecture**:

* 🚫 **Zero Network Telemetry**: Absolutely **no analytics**, **no tracking**, **no phone-home ping**, **no remote telemetry**, and **no third-party data collection**.
* 🔒 **Air-Gapped Execution**: All video encoding, screen capture, audio recording, vector annotations, and snapshot processing execute **100% locally on your machine's CPU/GPU and local storage**.
* 🌐 **Complete Offline Independence**: Never requires an active internet connection or external cloud API to function.
* 🔍 **Fully Auditable**: Built in safety-guaranteed **Rust** and released under the **GNU General Public License v3.0 (GPLv3)**. You can inspect the source code, build it locally, and audit network activity yourself.

---

## ✨ God-Tier Features & Architecture

### 🚀 Global System-Wide Hotkey Engine
* **System-Wide Key Listener**: Powered by a low-level `rdev` hook that responds instantly system-wide—even when the application window is minimized or hidden in the background.
* **Default Shortcuts**:
  * `Super + Shift + R` — Instant Video Recording Start / Stop toggle
  * `Super + Shift + S` — Fullscreen / Region Snapshot to Studio Editor
  * `Super + Shift + X` — Rubber-Band Screen Region Selector
  * `Escape` — Cancel active selection or dismiss modals
* **Customizable Shortcuts**: Rebind all hotkeys directly from the **Preferences ⚙️** panel.

### ✂️ CleanShot X-Style Rubber-Band Region Selector
* **Full-Monitor Live Backdrop**: Captures a live full-resolution snapshot of your screen via `xcap` and projects it onto a fullscreen overlay.
* **Bright Cutout & Dimming**: Displays a 45% translucent dark background while keeping your selected crop area **100% full brightness**.
* **Precision Drag Controls**: Includes 8×8px emerald (`#22d45e`) corner grab handles and a dynamic high-contrast dimension badge (`📐 W × H px`).
* **Coordinate Clamping**: Clamps selection box bounds `[0, Monitor_W]` and `[0, Monitor_H]` with automatic coordinate normalization for inverted drags.

### 🎨 Showcase Studio Annotation Editor
* **8 Professional Markup Tools**:
  1. 🏹 **Arrow Pointer**: Smooth vector direction arrows with auto-scaling heads.
  2. 🔲 **Highlight Box**: High-visibility outline frames for UI element callouts.
  3. ⭕ **Oval / Circle**: Rounded focus rings.
  4. ⬛ **Redact / Blur Box**: Solid redacting masks to censor passwords, API keys, and personal data.
  5. 💬 **Text Callout**: Interactive popover modal for custom dynamic text overlays.
  6. ① **Step Sequence Bubbles**: Consecutive numbered callout badges (①, ②, ③...).
  7. ✏️ **Freehand Curve**: Smooth digital pen strokes for quick sketching.
  8. 🔦 **Spotlight Focus**: Darkens surrounding image area while spotlighting key features.
* **Palette Swatches & Stroke Width Picker**: Instant tool color selection (Emerald, Crimson, Sky Blue, Amber, White) and stroke weight adjustments (`2px` to `8px`).

### ⚡ Native Hardware Auto-Detection Engine
* **GPU Hardware Encoding Pipeline**: Automatically detects your system GPU at startup (`sysinfo` + FFmpeg scanner) and routes video encoding to hardware acceleration:
  * `NVENC` for NVIDIA GPUs
  * `VideoToolbox` for Apple Silicon / macOS
  * `VA-API` for Intel & AMD on Linux
  * `QSV` for Intel QuickSync
* **Zero-Lag Fallback**: Smoothly tuned software fallback (`libx264` with dynamic thread auto-allocation) if hardware encoders are unavailable.

### 🎥 Picture-in-Picture Webcam Overlay & Audio Engine
* **Floating Webcam PIP**: Circular floating webcam overlay with real-time video feed streaming.
* **Microphone WAV PCM Recorder**: Synchronous multi-channel microphone recording mixed into post-processed video streams via `cpal`.

### 🌐 6-Language Localization Engine
* **Instant Internationalization**: Switch between **6 supported languages** with real-time UI dictionary updates:
  * 🇬🇧 English (`EN`) · 🇷🇴 Romanian (`RO`) · 🇪🇸 Spanish (`ES`)
  * 🇩🇪 German (`DE`) · 🇫🇷 French (`FR`) · 🇯🇵 Japanese (`JA`)

---

## 🛠️ Building & Running Locally

### Prerequisites
1. **Rust**: 1.75+ toolchain installed via [`rustup`](https://rustup.rs/)
2. **FFmpeg**: Installed and accessible in your system `PATH`
3. **OS**: Linux (X11 / Wayland), macOS (11+), or Windows (10/11)

### Build Commands

```bash
# Clone the repository
git clone https://github.com/wolfitway-debug/wolf-screen-recorder.git
cd wolf-screen-recorder

# Check code compilation & dependencies
cargo check

# Run locally in debug mode
cargo run

# Build optimized production release binary
cargo build --release
```

---

## 🔒 License & Legal Notice

This project is licensed under the **GNU General Public License v3.0 (GPLv3)**.

### 🛑 The Fork-Only Upstream Rule
**Notice to Contributors and Corporate Users:**
To maintain absolute intellectual property integrity and technical auditability, **WOLFITWAYAGENCY SRL maintains 100% sole copyright ownership of the primary upstream repository**.
* **Zero Direct Pull Requests**: Direct pull requests to this repository are disabled and will not be merged.
* **Fork-Only Modifications**: External developers, enterprises, and organizations are required to fork the repository for custom needs, internal adaptations, or specific feature extensions in accordance with GPLv3.

For enterprise licensing, corporate sponsorship, or commercial custom builds, access the Paddle merchant checkout link inside the application's **"Support the Dev" / "Susține Dezvoltatorul"** interface or contact `WOLFITWAYAGENCY SRL`.

---

*Copyright © WOLFITWAYAGENCY SRL. All Rights Reserved.*
