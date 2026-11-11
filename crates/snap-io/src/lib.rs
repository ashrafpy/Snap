pub mod decode;
pub mod error;
pub mod export;
pub mod lut;
pub mod sidecar;
pub mod xmp;

// Clean top-level re-exports
pub use decode::{is_raw_path, is_supported_image_path, load_image, RASTER_EXTENSIONS, RAW_EXTENSIONS};
pub use error::IoError;
pub use export::export_linear_image;
pub use lut::{
    built_in_luts, install_lut_file, install_lut_file_to, is_supported_lut_path, load_all_luts,
    load_lut_from_path, parse_3dl_lut, parse_cube_lut, parse_hald_png_lut,
};
pub use sidecar::{load_sidecar, save_sidecar, sidecar_path_for, SnapSidecar};
pub use xmp::{
    import_xmp_preset, install_xmp_preset, install_xmp_preset_to, is_xmp_path, parse_xmp_str,
};

pub use snap_core;
