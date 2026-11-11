use std::path::{Path, PathBuf};
use std::time::Instant;
use iced::keyboard::key::Named;
use iced::keyboard::{Key, Modifiers};
use iced::window;
use iced::{Size, Subscription};

use snap_app::app::{Message, SnapApp, Tab};
use snap_app::views;
use snap_core::config::{AppConfig, ExportFormat, ExportOptions, ResizeLongEdge};
use snap_core::looks::built_in_looks;
use snap_core::ops::{EditOp, Lut3dParams};
use snap_core::stack::EditStack;
use snap_gpu::headless::HeadlessGpu;

fn print_usage() {
    println!(
        r#"Snap Photo Editor v{}
Usage:
  snap [OPTIONS] [FILE]

Arguments:
  [FILE]                       Initial photo (.jpg, .png, .dng, .cr2, .arw...), 3D LUT (.cube, .3dl), or preset (.xmp) to load

Options:
  --export-cli <IN> <OUT>      Headless GPU single photo export (no GUI)
  --batch-dir <IN> <OUT>       Headless GPU folder batch export (no GUI)
  --capture-ui-previews <DIR>  Capture 12 studio UI preview PNGs to <DIR>
  --look <NAME_OR_SNAP>        Apply a named look preset or .snap sidecar file
  --xmp <PATH>                 Apply an Adobe XMP preset (.xmp)
  --lut <PATH>                 Apply a 3D LUT file (.cube, .3dl, .png)
  --format <FORMAT>            Export format for batch mode: jpg, png, tiff (default: jpg)
  --quality <0-100>            Export quality for JPEG (default: 92)
  -h, --help                   Print help information
  -V, --version                Print version information

Examples:
  snap photo.jpg                                      # Launch GUI opening photo.jpg and auto-loading photo.jpg.snap sidecar
  snap film_lut.cube                                  # Install LUT and launch GUI with 3D LUT tool active
  snap preset.xmp                                     # Install and apply Adobe XMP preset
  snap --export-cli raw.dng out.jpg --look "Portrait" # Export single photo with look
  snap --export-cli raw.dng out.jpg --xmp look.xmp    # Export single photo with XMP preset
  snap --batch-dir ./raw_in ./jpg_out --look "Pop" --format jpg --quality 92
"#,
        snap_core::version()
    );
}

fn resolve_stack_from_look_or_file(look_or_file: &str) -> Option<EditStack> {
    let p = Path::new(look_or_file);
    if p.is_file() {
        if let Ok(content) = std::fs::read_to_string(p) {
            if let Ok(sidecar) = serde_json::from_str::<snap_io::SnapSidecar>(&content) {
                return Some(sidecar.stack);
            }
            if let Ok(stack) = serde_json::from_str::<EditStack>(&content) {
                return Some(stack);
            }
        }
    }
    let custom_looks = AppConfig::load_custom_looks().unwrap_or_default();
    built_in_looks()
        .into_iter()
        .chain(custom_looks)
        .find(|l| l.name.eq_ignore_ascii_case(look_or_file) || l.id.eq_ignore_ascii_case(look_or_file))
        .map(|l| l.stack)
}

fn run_cli_export(
    input_path: &Path,
    output_path: &Path,
    look_name_or_snap: Option<&str>,
    xmp_path: Option<&Path>,
    lut_path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let start_t = Instant::now();
    println!("Snap CLI: Loading image from {}...", input_path.display());
    let source = snap_io::load_image(input_path)?;
    println!("Snap CLI: Loaded {}x{} image", source.width, source.height);

    let mut stack = if let Ok(Some(sidecar_stack)) = snap_io::load_sidecar(input_path) {
        println!("Snap CLI: Loaded existing sidecar {}", input_path.display());
        sidecar_stack
    } else {
        EditStack::default()
    };

    if let Some(look_arg) = look_name_or_snap {
        if let Some(s) = resolve_stack_from_look_or_file(look_arg) {
            println!("Snap CLI: Applying look/recipe '{}'", look_arg);
            stack = s;
        } else {
            eprintln!("Snap CLI Warning: Look or sidecar recipe '{look_arg}' not found");
        }
    }

    if let Some(xmp_p) = xmp_path {
        println!("Snap CLI: Importing XMP preset from {}...", xmp_p.display());
        let preset = snap_io::import_xmp_preset(xmp_p)?;
        println!("Snap CLI: Applied XMP preset '{}'", preset.name);
        stack = preset.stack;
    }

    let mut needed_luts = snap_io::load_all_luts();
    if let Some(lut_p) = lut_path {
        println!("Snap CLI: Loading LUT from {}...", lut_p.display());
        let luts = snap_io::load_lut_from_path(lut_p)?;
        if let Some(lut) = luts.into_iter().next() {
            println!("Snap CLI: Loaded LUT '{}'", lut.name);
            stack.push_op(EditOp::Lut3d(Lut3dParams {
                lut_id: lut.id.clone(),
                lut_name: lut.name.clone(),
                strength: 100.0,
                input_space: lut.input_space,
            }));
            needed_luts.push(lut);
        }
    }

    println!("Snap CLI: Initializing Headless GPU pipeline...");
    let mut headless = HeadlessGpu::new()?;

    // Upload any referenced DoubleExposure secondary photos
    for node in &stack.nodes {
        if let EditOp::DoubleExposure(ref p) = node.op {
            if let Some(ref path_str) = p.secondary_path {
                if let Ok(sec_img) = snap_io::load_image(Path::new(path_str)) {
                    headless.upload_secondary_image(path_str, &sec_img);
                }
            }
        }
    }

    println!("Snap CLI: Running GPU compute stack...");
    let exported = headless.export_full_res(&source, &stack, &needed_luts)?;

    println!("Snap CLI: Saving output to {}...", output_path.display());
    let ext = output_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg")
        .to_ascii_lowercase();

    let format = match ext.as_str() {
        "png" => ExportFormat::Png8,
        "tif" | "tiff" => ExportFormat::Tiff16,
        _ => ExportFormat::Jpeg { quality: 95 },
    };

    let options = ExportOptions {
        format,
        resize: ResizeLongEdge::Original,
        preserve_exif: true,
    };

    snap_io::export_linear_image(&exported, output_path, &options)?;
    let dur = start_t.elapsed();
    println!(
        "Snap CLI: Successfully exported {} ({}x{}) in {:.2}s!",
        output_path.display(),
        exported.width,
        exported.height,
        dur.as_secs_f32()
    );

    Ok(())
}

fn run_batch_dir(
    input_dir: &Path,
    output_dir: &Path,
    look_name_or_snap: Option<&str>,
    xmp_path: Option<&Path>,
    lut_path: Option<&Path>,
    format_str: Option<&str>,
    quality: Option<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let start_total = Instant::now();
    if !input_dir.is_dir() {
        return Err(format!("Input path {} is not a directory", input_dir.display()).into());
    }
    std::fs::create_dir_all(output_dir)?;

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && snap_io::is_supported_image_path(&path) {
            entries.push(path);
        }
    }
    entries.sort();

    if entries.is_empty() {
        println!("Snap CLI: No supported images found in {}", input_dir.display());
        return Ok(());
    }
    println!(
        "Snap CLI: Found {} supported image(s) in {}",
        entries.len(),
        input_dir.display()
    );

    let mut template_stack = None;
    if let Some(look_arg) = look_name_or_snap {
        if let Some(s) = resolve_stack_from_look_or_file(look_arg) {
            println!("Snap CLI: Using batch look/recipe '{}'", look_arg);
            template_stack = Some(s);
        } else {
            eprintln!("Snap CLI Warning: Look or sidecar recipe '{look_arg}' not found");
        }
    }
    if let Some(xmp_p) = xmp_path {
        println!("Snap CLI: Importing XMP preset from {}...", xmp_p.display());
        let preset = snap_io::import_xmp_preset(xmp_p)?;
        println!("Snap CLI: Using batch XMP preset '{}'", preset.name);
        template_stack = Some(preset.stack);
    }

    let mut needed_luts = snap_io::load_all_luts();
    let mut extra_lut_op = None;
    if let Some(lut_p) = lut_path {
        println!("Snap CLI: Loading LUT from {}...", lut_p.display());
        let luts = snap_io::load_lut_from_path(lut_p)?;
        if let Some(lut) = luts.into_iter().next() {
            println!("Snap CLI: Loaded LUT '{}'", lut.name);
            extra_lut_op = Some(EditOp::Lut3d(Lut3dParams {
                lut_id: lut.id.clone(),
                lut_name: lut.name.clone(),
                strength: 100.0,
                input_space: lut.input_space,
            }));
            needed_luts.push(lut);
        }
    }

    let q = quality.unwrap_or(92);
    let fmt_choice = format_str.unwrap_or("jpg").to_ascii_lowercase();
    let (export_fmt, out_ext) = match fmt_choice.as_str() {
        "png" => (ExportFormat::Png8, "png"),
        "tiff" | "tif" => (ExportFormat::Tiff16, "tif"),
        _ => (ExportFormat::Jpeg { quality: q }, "jpg"),
    };

    let export_options = ExportOptions {
        format: export_fmt,
        resize: ResizeLongEdge::Original,
        preserve_exif: true,
    };

    println!("Snap CLI: Initializing single Headless GPU pipeline for batch...");
    let mut headless = HeadlessGpu::new()?;

    if let Some(ref t_stack) = template_stack {
        for node in &t_stack.nodes {
            if let EditOp::DoubleExposure(ref p) = node.op {
                if let Some(ref path_str) = p.secondary_path {
                    if let Ok(sec) = snap_io::load_image(Path::new(path_str)) {
                        headless.upload_secondary_image(path_str, &sec);
                    }
                }
            }
        }
    }

    let total = entries.len();
    let mut successful = 0usize;

    for (idx, in_path) in entries.iter().enumerate() {
        let step_start = Instant::now();
        let file_stem = in_path.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
        let out_filename = format!("{file_stem}.{out_ext}");
        let out_path = output_dir.join(&out_filename);

        print!(
            "[{}/{}] Processing {} -> {}... ",
            idx + 1,
            total,
            in_path.file_name().and_then(|s| s.to_str()).unwrap_or(""),
            out_filename
        );

        let source = match snap_io::load_image(in_path) {
            Ok(img) => img,
            Err(e) => {
                println!("FAILED to load: {e}");
                continue;
            }
        };

        let mut stack = if let Some(ref t_stack) = template_stack {
            t_stack.clone()
        } else if let Ok(Some(sidecar_stack)) = snap_io::load_sidecar(in_path) {
            sidecar_stack
        } else {
            EditStack::default()
        };

        if let Some(ref lut_op) = extra_lut_op {
            stack.push_op(lut_op.clone());
        }

        for node in &stack.nodes {
            if let EditOp::DoubleExposure(ref p) = node.op {
                if let Some(ref path_str) = p.secondary_path {
                    if let Ok(sec) = snap_io::load_image(Path::new(path_str)) {
                        headless.upload_secondary_image(path_str, &sec);
                    }
                }
            }
        }

        match headless.export_full_res(&source, &stack, &needed_luts) {
            Ok(exported) => {
                if let Err(e) = snap_io::export_linear_image(&exported, &out_path, &export_options) {
                    println!("FAILED to export: {e}");
                    continue;
                }
                let dur = step_start.elapsed();
                println!("done ({:.2}s)", dur.as_secs_f32());
                successful += 1;
            }
            Err(e) => {
                println!("GPU processing error: {e}");
            }
        }
    }

    let total_dur = start_total.elapsed();
    println!(
        "Snap CLI: Batch complete! Successfully exported {}/{} images to {} in {:.2}s.",
        successful,
        total,
        output_dir.display(),
        total_dur.as_secs_f32()
    );

    Ok(())
}

fn handle_key_pressed(key: Key, modifiers: Modifiers) -> Option<Message> {
    let ctrl_or_cmd = modifiers.control() || modifiers.command();
    let shift = modifiers.shift();

    match key {
        Key::Named(Named::Space) => Some(Message::SetBypassEdits(true)),
        Key::Named(Named::Enter) => Some(Message::ApplyActiveTool),
        Key::Named(Named::Escape) => Some(Message::CancelActiveTool),

        Key::Character(ref s) => {
            let lower = s.to_ascii_lowercase();
            if ctrl_or_cmd {
                match lower.as_str() {
                    "z" => {
                        if shift {
                            Some(Message::Redo)
                        } else {
                            Some(Message::Undo)
                        }
                    }
                    "y" => Some(Message::Redo),
                    "s" => Some(Message::SaveSidecar),
                    "o" => Some(Message::OpenFileDialog),
                    "e" => Some(Message::SelectTab(Tab::Export)),
                    "c" if shift => Some(Message::CopyStack),
                    "v" if shift => Some(Message::PasteStack),
                    "0" => Some(Message::FitView),
                    "1" => Some(Message::Zoom100),
                    _ => None,
                }
            } else {
                match lower.as_str() {
                    "f" => Some(Message::FitView),
                    "1" => Some(Message::Zoom100),
                    _ => None,
                }
            }
        }

        _ => None,
    }
}

fn subscription(_app: &SnapApp) -> Subscription<Message> {
    iced::event::listen_with(|event, _status, _window| match event {
        iced::Event::Window(window::Event::FileDropped(path)) => Some(Message::FileDropped(path)),
        iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key, modifiers, ..
        }) => handle_key_pressed(key, modifiers),
        iced::Event::Keyboard(iced::keyboard::Event::KeyReleased { key, .. }) => {
            if key == Key::Named(Named::Space) {
                Some(Message::SetBypassEdits(false))
            } else {
                None
            }
        }
        _ => None,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();

    // Check for help and version flags
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_usage();
        return Ok(());
    }

    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("snap {}", snap_core::version());
        return Ok(());
    }

    // Check for --capture-ui-previews mode
    if let Some(pos) = args.iter().position(|a| a == "--capture-ui-previews") {
        if args.len() < pos + 2 {
            eprintln!("Error: --capture-ui-previews requires <OUTPUT_DIR>");
            std::process::exit(1);
        }

        let out_dir = PathBuf::from(&args[pos + 1]);
        snap_app::ui_preview::capture_all_previews(&out_dir)?;
        return Ok(());
    }

    // Check for --export-cli mode
    if let Some(pos) = args.iter().position(|a| a == "--export-cli") {
        if args.len() < pos + 3 {
            eprintln!("Error: --export-cli requires <INPUT_PATH> and <OUTPUT_PATH>");
            std::process::exit(1);
        }

        let input_path = PathBuf::from(&args[pos + 1]);
        let output_path = PathBuf::from(&args[pos + 2]);

        let mut look_name = None;
        if let Some(l_pos) = args.iter().position(|a| a == "--look") {
            if args.len() > l_pos + 1 {
                look_name = Some(args[l_pos + 1].as_str());
            }
        }

        let mut xmp_path = None;
        if let Some(xmp_pos) = args.iter().position(|a| a == "--xmp") {
            if args.len() > xmp_pos + 1 {
                xmp_path = Some(Path::new(&args[xmp_pos + 1]));
            }
        }

        let mut lut_path = None;
        if let Some(lut_pos) = args.iter().position(|a| a == "--lut") {
            if args.len() > lut_pos + 1 {
                lut_path = Some(Path::new(&args[lut_pos + 1]));
            }
        }

        run_cli_export(&input_path, &output_path, look_name, xmp_path, lut_path)?;
        return Ok(());
    }

    // Check for --batch-dir mode
    if let Some(pos) = args.iter().position(|a| a == "--batch-dir") {
        if args.len() < pos + 3 {
            eprintln!("Error: --batch-dir requires <INPUT_DIR> and <OUTPUT_DIR>");
            std::process::exit(1);
        }

        let input_dir = PathBuf::from(&args[pos + 1]);
        let output_dir = PathBuf::from(&args[pos + 2]);

        let mut look_name = None;
        if let Some(l_pos) = args.iter().position(|a| a == "--look") {
            if args.len() > l_pos + 1 {
                look_name = Some(args[l_pos + 1].as_str());
            }
        }

        let mut xmp_path = None;
        if let Some(xmp_pos) = args.iter().position(|a| a == "--xmp") {
            if args.len() > xmp_pos + 1 {
                xmp_path = Some(Path::new(&args[xmp_pos + 1]));
            }
        }

        let mut lut_path = None;
        if let Some(lut_pos) = args.iter().position(|a| a == "--lut") {
            if args.len() > lut_pos + 1 {
                lut_path = Some(Path::new(&args[lut_pos + 1]));
            }
        }

        let mut format_str = None;
        if let Some(f_pos) = args.iter().position(|a| a == "--format") {
            if args.len() > f_pos + 1 {
                format_str = Some(args[f_pos + 1].as_str());
            }
        }

        let mut quality = None;
        if let Some(q_pos) = args.iter().position(|a| a == "--quality") {
            if args.len() > q_pos + 1 {
                quality = args[q_pos + 1].parse::<u8>().ok();
            }
        }

        run_batch_dir(
            &input_dir,
            &output_dir,
            look_name,
            xmp_path,
            lut_path,
            format_str,
            quality,
        )?;
        return Ok(());
    }

    // Extract optional image or LUT path
    let initial_path = args
        .iter()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .map(PathBuf::from);

    // Launch Iced application
    iced::application(
        move || SnapApp::boot(initial_path.clone()),
        SnapApp::update,
        views::view,
    )
    .title("Snap Photo Editor")
    .theme(|_app: &SnapApp| snap_app::theme::snap_theme())
    .window_size(Size::new(1400.0, 880.0))
    .antialiasing(true)
    .subscription(subscription)
    .run()?;

    Ok(())
}
