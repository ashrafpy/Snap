# Snap Photo Editor

> **Snapseed 4.2.1 parity** meets native Linux Vulkan performance in a fast, color-accurate, non-destructive RAW & image editing studio powered by `iced 0.14`, `wgpu 27.0`, Linear Rec.2020, and Oklab/Oklch.

---

## 1. Overview & Philosophy

Snap brings the fluid dual-axis canvas gesture workflow and creative toolkit of **Snapseed v4.2.1** to desktop Linux. Built in Rust (2024 edition) with `iced` and `wgpu`, Snap processes all edits non-destructively in unclamped scene-referred **Linear Rec.2020 `Rgba16Float`** on the GPU with **Oklab / Oklch** perceptual color math.

---

## 2. Complete Feature Set (Phases 1–4 — All 36 Snapseed 4.2.1 Tools)

### Core & Color (10 Tools)
1. **RAW Develop**: Exposure (`-8..+8 EV`), highlights, shadows, contrast, structure, temperature, tint, saturation, and WB presets.
2. **Tune Image & Auto Wand**: Brightness, contrast, saturation, ambiance, highlights, shadows, warmth, and histogram auto-balance.
3. **Details**: Separable edge-aware bilateral Structure & Sharpening.
4. **Curves**: 5-channel Fritsch-Carlson monotonic cubic splines (`RGB`, `Red`, `Green`, `Blue`, `Luminance`) + 10 presets.
5. **White Balance**: Temperature, tint, Auto gray-world WB, and interactive Neutral Eyedropper.
6. **Color / HSL (4.0)**: 8-band color mixer (*Red, Orange, Yellow, Green, Cyan, Blue, Purple, Magenta*) x Hue, Saturation, Luminance.
7. **3D LUT (4.2)**: Hardware trilinear 3D LUTs (`.cube`, `.3dl`, HaldCLUT `.png`, `.zip` bundles) + 6 built-in film LUTs + Input Color Space selector (*sRGB, Rec.709, Apple/Sony/Canon Log*).
8. **Color Grade (4.2)**: Interactive 3-Way + Global Color Wheels (*Shadows, Midtones, Highlights, Global*) x Hue, Saturation, Luminance + Balance & Blending.
9. **Dehaze (4.0)**: Dark-channel atmospheric transmission recovery & warmth offset.
10. **Tonal Contrast**: Multi-band local contrast across High, Mid, and Low tones with Shadow & Highlight protection.

### Geometry & Canvas (4 Snapseed Tools via 3 `ToolId`s)
11. **Crop & Rotate**: Aspect ratio presets, interactive straighten (`-45..+45 deg`) with auto-inscribed bounds, 90 deg CW/CCW, Flip H/V.
12. **Perspective**: 3x3 inverse homography vertical/horizontal keystone, rotation, scale, and *Smart Mirror* edge fill.
13. **Expand**: Outward canvas margin expansion (`0..25%`) with *Black*, *White*, or *Smart Edge* mirrored patch synthesis.

### Local Adjustments & Retouching (3 Tools)
14. **Selective (Nik U-Point)**: Up to 16 interactive pins weighted by GPU Oklab $\Delta E$ color similarity x smooth radial falloff (*Brightness, Contrast, Saturation, Structure*).
15. **Brush**: Interactive painted `256x256` signed adjustment map for *Dodge & Burn*, *Exposure*, *Temperature*, and *Saturation*.
16. **Healing**: Multi-directional boundary exemplar patch synthesis & feathering for spot/blemish removal.

### Film, Creative & Blur Optics (15 Tools)
17. **Black & White**: 6 optical color filters (*Neutral, Red, Orange, Yellow, Green, Blue*), brightness, contrast, grain.
18. **Vignette**: Draggable center, inner/outer radius, outer & inner brightness.
19. **Halation (4.0)**: 2-pass separable red-orange film emulsion highlight scatter preserving HDR `> 1.0`.
20. **Bloom (4.0)**: 2-pass Pro-Mist soft highlight diffusion preserving HDR headroom.
21. **Grainy Film (4.0)**: 8 analog film stocks (*K100, K400, F200, Velvia50, Cinestill 800T, Ilford HP5, Cross Process, Polaroid*) + organic halide grain.
22. **Noir**: 4 silver-gelatin B&W presets (*Dark, Light, Film, Harsh*), matte lifted blacks (*Wash*), contrast, and grain.
23. **HDR Scape**: 2-pass edge-preserving bilateral log-luminance local tone mapping (*Nature, People, Fine, Strong*).
24. **Glamour Glow**: 2-pass soft-light Gaussian halo with 5 styles, glow, warmth, and saturation.
25. **Drama**: 6 dramatic micro-contrast & highlight compression presets.
26. **Vintage**: 12 retro split-tone color styles with center-weighted vignette.
27. **Double Exposure**: Secondary image overlay with 6 blend modes (*Default, Lighten, Darken, Add, Subtract, Blend*).
28. **Retrolux**: 13 retro styles with deterministic seed-driven scratches, light leaks, and shuffle.
29. **Grunge**: 5 procedural multi-octave FBM grunge texture styles with seed shuffle.
30. **Lens Blur (4.2)**: Elliptical & Linear tilt-shift focus + 32-tap Golden-Angle Vogel spiral **Bokeh Boost** optical highlight discs.
31. **Motion Blur (4.2)**: 24-tap *Full Scene* directional & *Radial Pan* streak blur.

### Portrait, Frames & Typography (4 Tools)
32. **Portrait**: Draggable face spotlight, Oklch skin-hue bilateral frequency-separation smoothing, eye clarity, and skin tone presets.
33. **Head Pose**: Smooth 2.5D radial/perspective UV warp for Yaw, Pitch, Smile, and Focal Length.
34. **Frames**: 8 procedural borders (*Clean White, Matte Black, Polaroid, 35mm Film, Instant Date, Gallery Line, Rough Edge, Letterbox*).
35. **Text Overlay**: Pure-Rust anti-aliased vector stroke font + 8 Snapseed badge layouts (*Modern Bold, Editorial Serif, Minimal Box, Stamp Frame, Cinema Sub, Retro Badge, Vertical Tag, Signature*) with normal & Knockout Invert compositing.

### Non-Destructive Stack, Smart Masking & Ecosystem
- **Per-Layer Mask Brush & Ruby-Red Overlay**: Paint any edit step in/out (`0%..100%`), invert, clear, feather, and preview mask density directly on the canvas inside `View Edits`.
- **Smart Masking (4.0)**: 1-tap *Sky, Subject, Background, Highlights, Midtones, Shadows, and Skin* isolation powered by Oklch + spatial salience priors.
- **Adobe Lightroom / ACR `.xmp` Preset Import**: Zero-dependency `.xmp` importer mapping develop settings, 8-band HSL, vignette, B&W, and tone curves directly into non-destructive `LookPreset` stacks.
- **11 Built-in Looks, Custom Looks & `.snap` Sidecars**: Full stack serialization with compact RLE-encoded masks.
- **Headless Vulkan CLI & Folder Batch Export**: Single-file `--export-cli`, folder `--batch-dir`, and `--capture-ui-previews` screenshot harness.

---

## 3. Build & Run

### Commands
- **Interactive Darkroom Studio GUI**:
  ```bash
  cargo run --release
  cargo run --release -- photo.dng
  ```
- **Single-Image Headless GPU Export**:
  ```bash
  snap --export-cli <INPUT> <OUTPUT> [--look <NAME_OR_SNAP>] [--lut <PATH>] [--xmp <PATH>]
  ```
- **Folder GPU Batch Processing**:
  ```bash
  snap --batch-dir <INPUT_DIR> <OUTPUT_DIR> [--look <NAME_OR_SNAP>] [--lut <PATH>] [--xmp <PATH>] [--format jpg|png|tiff] [--quality 92]
  ```
- **Offscreen Vulkan Studio Previews (12 Scenarios at 1440x900)**:
  ```bash
  snap --capture-ui-previews /tmp/snap_previews
  ```
- **Offline Workspace Test Suite (Unit + XMP + Smart Mask + Live Vulkan GPU)**:
  ```bash
  cargo test --workspace --offline
  ```

---

## 4. Canvas Gestures & Keyboard Shortcuts

| Shortcut / Gesture | Action |
| :--- | :--- |
| Vertical Drag on Canvas | Snapseed parameter picker HUD |
| Horizontal Drag on Canvas | Live parameter adjustment |
| Mouse Wheel / Middle or `Alt` Drag | Smooth cursor-centered zoom / pan |
| `Space` (Hold) | Compare / Bypass Edits (Preview original) |
| `Enter` / `Escape` | Apply / Cancel active tool session |
| `Ctrl + Z` / `Ctrl + Y` (`Ctrl + Shift + Z`) | Undo / Redo (in-tool session or document stack) |
| `Ctrl + S` / `Ctrl + O` / `Ctrl + E` | Save `.snap` sidecar / Open file / Export tab |
| `Ctrl + Shift + C` / `Ctrl + Shift + V` | Copy / Paste non-destructive edit recipe |
| `F` (`Ctrl + 0`) / `1` (`Ctrl + 1`) | Fit view (`90%` gallery mat) / `100%` pixel zoom |

---

## 5. Workspace Architecture

- **`crates/snap-core`**: Domain model, `Linear Rec.2020` <-> `Oklab`/`Oklch` color science, Fritsch-Carlson splines, 35 `EditOp`s, `LayerMask` + 7 `SmartMaskKind`s, `BrushMap`, `EditStack`, and `LookPreset`s.
- **`crates/snap-io`**: RAW (`rawler`), JPEG (`zune-jpeg`), 8/16-bit PNG/TIFF/WebP (`image`), ICC (`moxcms`), EXIF (`kamadak-exif`), `.cube`/`.3dl`/Hald `.png`/`.zip` 3D LUTs, Lightroom `.xmp` presets, `.snap` sidecars, and Lanczos3 export.
- **`crates/snap-gpu`**: `wgpu 27.0` Vulkan compute engine (`Rgba16Float` ping-pong passes, hardware trilinear 3D LUT cache, 26 Phase 2–4 WGSL shaders, per-node `LayerMask` compositing, vector `text_raster`, atomic 256-bin histogram, `HeadlessGpu`).
- **`crates/snap-app`**: `iced 0.14` Wayland/X11 Darkroom Studio, 69 pure-vector `canvas::Program` icons, 3-Way Color Grade wheels, interactive viewport overlays, `--batch-dir` CLI, and offscreen Vulkan preview harness.

> For the complete 36-tool parity matrix, system architecture diagram, and GPU engineering invariants, see [product_spec.md](product_spec.md).
