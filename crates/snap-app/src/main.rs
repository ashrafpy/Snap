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
  [FILE]                       Initial photo (.jpg, .png, .dng, .cr2, .arw...) or 3D LUT (.cube, .3dl) to load

Options:
  --export-cli <IN> <OUT>      Headless GPU batch export (no GUI)
  --capture-ui-previews <DIR>  Capture 7 studio UI preview PNGs to <DIR>
  --look <NAME>                Apply a named look preset during CLI export
  --lut <PATH>                 Apply a 3D LUT file during CLI export
  -h, --help                   Print help information
  -V, --version                Print version information

Examples:
  snap photo.jpg               # Launch GUI opening photo.jpg and auto-loading photo.jpg.snap sidecar
  snap film_lut.cube           # Install LUT and launch GUI with 3D LUT tool active
  snap --export-cli raw.dng out.jpg --look "Portrait"
"#,
        snap_core::version()
    );
}

fn run_cli_export(
    input_path: &Path,
    output_path: &Path,
    look_name: Option<&str>,
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

    if let Some(name) = look_name {
        let custom_looks = AppConfig::load_custom_looks().unwrap_or_default();
        let found = built_in_looks()
            .into_iter()
            .chain(custom_looks)
            .find(|l| l.name.eq_ignore_ascii_case(name) || l.id.eq_ignore_ascii_case(name));

        if let Some(look) = found {
            println!("Snap CLI: Applying look '{}'", look.name);
            stack = look.stack;
        } else {
            eprintln!("Snap CLI Warning: Look '{name}' not found");
        }
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

        let mut lut_path = None;
        if let Some(lut_pos) = args.iter().position(|a| a == "--lut") {
            if args.len() > lut_pos + 1 {
                lut_path = Some(Path::new(&args[lut_pos + 1]));
            }
        }

        run_cli_export(&input_path, &output_path, look_name, lut_path)?;
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
