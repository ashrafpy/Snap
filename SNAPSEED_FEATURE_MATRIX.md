# Snapseed 4.2.1 (`4.2.1.1014141696`) Complete Feature Catalog & `Snap` Parity Matrix

> **Companion Plan**: See [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) for the crate architecture, GPU invariants, and 3-Wave parallel execution blueprint.
>
> Based on **Snapseed v4.2.1** (including the major 4.0–4.2 overhaul: **3D LUT / XMP Preset Import**, **Color Grade wheels**, **8-channel Color HSL**, **Dehaze**, **Halation & Bloom**, **Motion Blur**, **Lens Blur Bokeh Boost**, **Smart Masking**, and the **Customizable Favorites Bar**). Only the mobile-only camera viewfinder (*Snapseed Camera*) is omitted since `Snap` targets Linux desktop.

---

## 1. LUTs, Looks & Preset Engine (Snapseed 4.2 Highlight)

| Feature | Snapseed 4.2.1 Capabilities | Status | `Snap` Target Phase & Linux Upgrade |
| :--- | :--- | :---: | :--- |
| **3D LUT Import & Tool** | Import `.cube`, `.3dl`, and `.png` (HaldCLUT) via *Manage Looks* or *LUT* tool; `0–100%` Strength slider; non-destructive stack layer | `[x]` Done | **Phase 1** — Hardware trilinear `wgpu::TextureDimension::D3` (`Rgba16Float`), supports arbitrary cube sizes (`17^3`, `33^3`, `65^3`), HaldCLUT `.png`, `.3dl`, `.cube`, `.zip` packs, plus **Input Color Space selector** (*sRGB, Rec.709, Apple/Sony/Canon Log*) |
| **Sidecar & LUT Pack Import** | `.snapseed` stack recipes, `.zip` bundles | `[x]` Done | **Phase 1** — `.snap` JSON non-destructive sidecar + `.zip` LUT bundle extraction into `~/.local/share/snap/luts/` |
| **Adobe `.xmp` Preset Import** | `.xmp` (Lightroom / Camera Raw develop profiles) | `[x]` Done | **Phase 2** — `snap-io::xmp` parametric importer mapping Exposure, Contrast, Highlights/Shadows, WB, Clarity, 8-band HSL, Vignette, B&W, and Tone Curves into `LookPreset` |
| **Built-in & Custom "My Looks"** | 11 built-in Looks (*Portrait, Smooth, Pop, Accentuate, Faded Glow, Morning, Bright, Fine Art, Push, Structure, Silhouette*) + save any Edit Stack as a reusable Look | `[x]` Done | **Phase 1** — Stored cleanly in `~/.local/share/snap/looks/` with 1-click apply in the `Looks` tab |
| **Copy / Paste & Batch Recipes** | Copy edit stack from one photo and paste onto others / batch apply | `[x]` Done | **Phase 1** (`Copy` / `Paste` stack recipe + single-image `--export-cli`) + **Phase 3** (`--batch-dir <IN> <OUT> [--look\|--lut\|--xmp]` GPU folder batch export) |

---

## 2. Workspace, Navigation & Non-Destructive Workflow

| Feature | Snapseed 4.2.1 Capabilities | Status | `Snap` Implementation |
| :--- | :--- | :---: | :--- |
| **Dual-Axis Canvas Gesture** | Vertical drag = pick parameter HUD; Horizontal drag = adjust `-100..+100` | `[x]` Done | **Phase 1** — True `stack!` glass HUD overlays + right-sidebar studio sliders |
| **Hold to Compare** | Long-press photo for instant original preview | `[x]` Done | **Phase 1** — `Spacebar` hold + top-bar `Compare` toggle |
| **Customizable Favorites Bar** | Pin favorite tools (e.g., *Tune, LUT, Curves, Selective, Healing*) to top quick-access dock | `[x]` Done | **Phase 1** — Amber star pin on any tool card -> top Favorites Quick-Bar persisted in `~/.config/snap/config.json` |
| **In-Tool Undo / Redo (4.1)** | Granular per-slider/node undo & redo inside an active tool session before committing to stack | `[x]` Done | **Phase 1** — `ToolSessionHistory` + global 100-step `HistoryManager<EditStack>` |
| **View Edits (History Stack)** | Inspect every applied step; toggle, delete, opacity-blend, or re-open & tweak earlier parameters | `[x]` Done | **Phase 1** — `View Edits` inspector with enable eye, opacity slider, edit, and delete |
| **Per-Layer Mask Brush** | Paint any edit step in/out (`0%, 25%, 50%, 75%, 100%`), invert mask, translucent red overlay | `[x]` Done | **Phase 2** — `256x256` feathered `LayerMask` per `EditNode` + GPU `MASK_BLEND_WGSL` + ruby-red canvas mask preview |
| **Smart Masking (4.0)** | One-tap subject / sky / background isolation inside Edit Stack | `[x]` Done | **Phase 3** — 1-tap *Sky, Subject, Background, Highlights, Midtones, Shadows, Skin Tones* generators |
| **Live Histogram & Zoom Controls** | Real-time RGB/Luminance histogram + zoom/pan (`Fit`, `100%`, wheel zoom, middle-drag pan) | `[x]` Done | **Phase 1** — 256-bin atomic GPU histogram + darkroom instrument curve widget + `0.90` gallery mat fit |
| **Export & Sizing** | Save sidecar, Export JPG (`100%/95%/80%`), PNG, resize (`Original`, `4000px`..`800px`), preserve EXIF & ICC | `[x]` Done | **Phase 1** — Full-res GPU export to JPG, 8/16-bit PNG, and 16-bit TIFF with Lanczos3 resize |

---

## 3. All 36 Editing & Retouching Tools (with Sub-Parameters)

*(Note: Crop #10 and Rotate/Straighten #11 share unified interactive geometry in `ToolId::CropRotate`, yielding 35 `ToolId` enum variants covering all 36 Snapseed 4.2.1 tools.)*

### A. RAW & Core Tonal / Color Tools
| # | Tool | Sub-Parameters & Controls | Status | Phase |
| :- | :--- | :--- | :---: | :---: |
| 1 | **RAW Develop** | Exposure (`-8..+8 EV`), Highlights, Shadows, Contrast, Structure, Temp, Tint, Saturation | `[x]` Done | **Phase 1** |
| 2 | **Tune Image** | Brightness, Contrast, Saturation, Ambiance, Highlights, Shadows, Warmth (`-100..+100`) + Auto-Adjust Wand | `[x]` Done | **Phase 1** |
| 3 | **Details** | Structure (`-100..+100` separable edge-aware bilateral local contrast), Sharpening (`0..100`) | `[x]` Done | **Phase 1** |
| 4 | **Curves** | Channels (*RGB, Red, Green, Blue, Luminance*), interactive monotonic spline nodes + 10 presets (*Neutral, Hard/Soft Contrast, Brighten, Darken, High/Low Key, Fade, Punch, Cross Process*) | `[x]` Done | **Phase 1** |
| 5 | **White Balance** | Temperature (`-100..+100`), Tint (`-100..+100`), Neutral Gray Eyedropper, Auto WB | `[x]` Done | **Phase 1** |
| 6 | **3D LUT (4.2)** | 6 built-in `33^3` film LUTs + user-imported `.cube` / `.3dl` / Hald `.png` / `.zip` library, Strength (`0..100%`), Input Color Space | `[x]` Done | **Phase 1** |
| 7 | **Color HSL (4.0)** | 8 Channels (*Red, Orange, Yellow, Green, Cyan, Blue, Purple, Magenta*) x Hue, Saturation, Luminance (`-100..+100`) | `[x]` Done | **Phase 1** |
| 8 | **Color Grade (4.2)** | 3-Way Color Wheels (*Shadows, Midtones, Highlights* + *Global*), per-wheel Hue/Sat/Luminance, Balance & Blending | `[x]` Done | **Phase 2** |
| 9 | **Dehaze (4.0)** | Atmospheric Dehaze Strength (`-100..+100`) + Warmth Offset (`-100..+100`) with dark-channel transmission recovery | `[x]` Done | **Phase 2** |

### B. Geometry & Canvas Tools
| # | Tool | Sub-Parameters & Controls | Status | Phase |
| :- | :--- | :--- | :---: | :---: |
| 10 | **Crop** | *Free, Original, 1:1, DIN, 3:2, 4:3, 5:4, 7:5, 16:9* + interactive corner/edge drag | `[x]` Done | **Phase 1** |
| 11 | **Rotate / Straighten** | Fine straighten angle (`-45..+45 deg`) with auto-inscribed bounds, 90 deg CW/CCW, Flip Horizontal/Vertical | `[x]` Done | **Phase 1** |
| 12 | **Perspective** | Vertical/Horizontal Keystone (`-100..+100`), Rotate (`-45..+45`), Scale (`50..200%`), Edge Fill (*Black, White, SmartMirror*) | `[x]` Done | **Phase 3** |
| 13 | **Expand** | Outward canvas border padding (`0..25%` per side) + Edge Fill (*Black, White, SmartEdge* texture synthesis) | `[x]` Done | **Phase 3** |

### C. Local Adjustments & Retouching
| # | Tool | Sub-Parameters & Controls | Status | Phase |
| :- | :--- | :--- | :---: | :---: |
| 14 | **Selective (U-Point)** | Up to 16 Control Points on canvas; adjustable Radius + *Brightness, Contrast, Saturation, Structure* via GPU Oklab $\Delta E$ color similarity | `[x]` Done | **Phase 2** |
| 15 | **Brush** | *Dodge & Burn, Exposure, Temperature, Saturation* (`-1.0` to `+1.0` step, Radius, Feather + Eraser + Red mask preview) | `[x]` Done | **Phase 2** |
| 16 | **Healing** | Brush/spot blemish & object removal (`Vec<HealStroke>` multi-directional exemplar patch synthesis & boundary feathering) | `[x]` Done | **Phase 3** |

### D. Cinematic, Analog & Creative Looks
| # | Tool | Sub-Parameters & Controls | Status | Phase |
| :- | :--- | :--- | :---: | :---: |
| 17 | **Black & White** | Optical Color Filters (*Neutral, Red, Orange, Yellow, Green, Blue*), Brightness, Contrast, Grain | `[x]` Done | **Phase 1** |
| 18 | **Vignette** | Adjustable center (`x, y`), Radius, Outer Brightness, Inner Brightness | `[x]` Done | **Phase 1** |
| 19 | **Halation (4.0)** | Highlight Threshold, Red-Orange Film Halo Strength, Spatial Spread/Radius (2-pass separable scatter) | `[x]` Done | **Phase 2** |
| 20 | **Bloom (4.0)** | Pro-Mist Diffusion Strength, Dispersion Size, Highlight Luminance Range (2-pass separable glow) | `[x]` Done | **Phase 2** |
| 21 | **Tonal Contrast** | High Tones, Medium Tones, Low Tones (`-100..+100`), Protect Shadows (`0..100`), Protect Highlights (`0..100`) | `[x]` Done | **Phase 2** |
| 22 | **Grainy Film (4.0)** | 8 analog film stocks (*K100, K400, F200, Velvia50, Cinestill800T, IlfordHP5, CrossProcess, Polaroid*) + organic Halide Grain & Style Strength | `[x]` Done | **Phase 2** |
| 23 | **Noir** | *Dark, Light, Film, Harsh* presets + Brightness, Contrast, Wash (matte lifted blacks), Grain | `[x]` Done | **Phase 2** |
| 24 | **HDR Scape** | *Nature, People, Fine, Strong* edge-preserving local tone-mapping + Filter Strength, Brightness, Saturation | `[x]` Done | **Phase 3** |
| 25 | **Glamour Glow** | Styles 1–5 + Glow (`0..100`), Saturation (`-100..+100`), Warmth (`-100..+100`) | `[x]` Done | **Phase 3** |
| 26 | **Drama** | 6 presets (*Drama 1/2, Bright 1/2, Dark 1/2*) + Filter Strength, Saturation | `[x]` Done | **Phase 3** |
| 27 | **Vintage** | 12 color split-tone styles + Brightness, Saturation, Style Strength, Vignette, Center Size | `[x]` Done | **Phase 3** |
| 28 | **Lens Blur (4.2)** | Elliptical / Linear tilt-shift, Transition, Vignette, Blur Strength, **Bokeh Boost** (Vogel-disk specular highlight blooming) | `[x]` Done | **Phase 3** |
| 29 | **Motion Blur (4.2)** | Full Scene / Radial Panning directional streak, Angle (`0..360 deg`), Strength, Center (`x, y`) | `[x]` Done | **Phase 3** |
| 30 | **Double Exposure** | Secondary image overlay + 6 Blend Modes (*Default, Lighten, Darken, Add, Subtract, Blend*) + Opacity | `[x]` Done | **Phase 3** |
| 31 | **Retrolux** | 13 retro styles + Brightness, Saturation, Contrast, Style Strength, Scratches, Light Leaks, Seed Shuffle | `[x]` Done | **Phase 4** |
| 32 | **Grunge** | 5 styles + procedural FBM grunge overlays, Brightness, Contrast, Saturation, Texture Strength, Seed Shuffle | `[x]` Done | **Phase 4** |

### E. Portrait & Finishing Tools
| # | Tool | Sub-Parameters & Controls | Status | Phase |
| :- | :--- | :--- | :---: | :---: |
| 33 | **Portrait** | Draggable Face Center & Radius, Face Spotlight, Oklch Skin-Hue Bilateral Smoothing, Eye Clarity, Skin Tone preset | `[x]` Done | **Phase 4** |
| 34 | **Head Pose** | 2.5D radial/perspective face warp: Yaw X, Pitch Y, Smile, Focal Length | `[x]` Done | **Phase 4** |
| 35 | **Frames** | 8 styles (*Clean White, Matte Black, Polaroid, 35mm Film, Instant Date, Gallery Line, Rough Edge, Letterbox*) + Width, Inner Line, Aspect Pad | `[x]` Done | **Phase 4** |
| 36 | **Text** | 8 scalable vector typography/badge styles + Text/Subtext, draggable Center, Scale, Opacity, Knockout Invert Mask, Color Preset | `[x]` Done | **Phase 4** |
