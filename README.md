# Snap Photo Editor

> **Snapseed 4.2-inspired tactile UX** meets native Linux Vulkan performance in a lightning-fast, non-destructive RAW & image editing suite powered by `wgpu`, Linear Rec.2020, and Oklab.

---

## 1. Overview & Philosophy

Snap brings the legendary, fluid touch-and-gesture workflow of classic mobile editing (inspired by Snapseed 4.2) to desktop Linux. Built from the ground up in Rust using `iced` and `wgpu`, Snap achieves **60–144+ fps** buttery-smooth rendering across high-refresh displays while executing all image adjustments in a wide-gamut **Linear Rec.2020** working space processed through perceptually uniform **Oklab** and **Oklch** color spaces.

---

## 2. Features in v0.1 (Phase 1)

### All 10 Essential Editing Tools
1. **RAW Develop**: Exposure, highlights, shadows, blacks, whites, and lens correction.
2. **Tune Image & Magic Wand**: Brightness, contrast, saturation, ambience, highlights, shadows, warmth, plus AI-assisted Magic Wand auto-enhancement.
3. **Details**: Halo-free bilateral Structure (micro-contrast) & Sharpening.
4. **Curves**: 5-channel monotonic spline (Master, Red, Green, Blue, RGB) with 10 built-in presets.
5. **White Balance**: Temperature, tint, and interactive Neutral Eyedropper sampling.
6. **Color / HSL**: 8-band color mixer (Red, Orange, Yellow, Green, Cyan, Blue, Purple, Magenta) adjusting Hue, Saturation, and Lightness.
7. **3D LUT**: Import `.cube`, `.3dl`, and Hald `.png`/`.zip` 3D LUTs with strength and color space controls, plus 6 built-in film LUTs.
8. **Black & White**: Optical color filters (Red, Orange, Yellow, Green, Blue) and adjustable film grain simulation.
9. **Vignette**: Outer brightness, inner brightness, feathering, and center point adjustments.
10. **Crop & Rotate**: Aspect ratio constraints, free rotation, straightening, and auto-inscribed bounding box calculation.

### Ecosystem & Workflow Features
- **11 Built-in Snapseed Looks + Custom Looks**: Instantly apply professional looks or save your own custom presets.
- **Customizable Favorites Bar**: Pin your most-used tools for rapid access.
- **"View Edits" Non-Destructive Stack**: Full layer inspection, re-ordering, parameter modification, and in-tool session Undo/Redo (`Ctrl+Z` / `Ctrl+Y`).
- **`.snap` Sidecars**: Sidecar files saving your complete non-destructive edit stack alongside your RAW or raster files.
- **Full-Resolution GPU Export**: Export 8-bit sRGB (JPEG/PNG) or 16-bit linear/gamma TIFF images leveraging GPU compute shaders.

---

## 3. Build & Run

### Prerequisites
- Linux with Vulkan support and a Vulkan-capable GPU.
- Rust toolchain (`stable`).

### Commands
- **Interactive GUI**:
  ```bash
  cargo run --release
  cargo run --release -- photo.dng
  ```
- **Headless CLI Export**:
  ```bash
  snap --export-cli <INPUT> <OUTPUT> [--look <NAME>] [--lut <PATH>]
  ```
- **Local Installation**:
  ```bash
  ./scripts/install-local.sh
  ```

---

## 4. Canvas Gestures & Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Space` (Hold) | Compare / Bypass Edits (Preview original) |
| `Enter` | Apply active tool adjustment |
| `Escape` | Cancel active tool adjustment |
| `Ctrl + Z` | Undo |
| `Ctrl + Shift + Z` or `Ctrl + Y` | Redo |
| `Ctrl + S` | Save `.snap` sidecar |
| `Ctrl + O` | Open file dialog |
| `Ctrl + E` | Switch to Export tab |
| `Ctrl + Shift + C` | Copy edit stack |
| `Ctrl + Shift + V` | Paste edit stack |
| `F` or `Ctrl + 0` | Fit view to canvas |
| `1` or `Ctrl + 1` | 100% actual pixels zoom |

---

## 5. Workspace Architecture

Snap is organized into a clean, modular multi-crate Cargo workspace:
- **`snap-core`**: Core math, color science (`Linear Rec.2020` <-> `Oklab`/`Oklch`), curves, edits, look presets, and non-destructive `EditStack`.
- **`snap-io`**: File loading and decoding (RAW via rawloader/image, raster images, `.cube`/`.3dl`/Hald `.png` 3D LUTs, and `.snap` sidecars).
- **`snap-gpu`**: High-performance GPU pipeline and compute shaders (`wgpu`) for real-time canvas rendering and headless batch exports.
- **`snap-app`**: `iced`-based GUI application, canvas gestures, tool panes, favorites bar, and event handling.
