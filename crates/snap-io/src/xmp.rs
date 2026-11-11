use std::fs;
use std::path::Path;

use snap_core::config::AppConfig;
use snap_core::curves::{CurveChannel, CurvePoint, CurvesParams};
use snap_core::looks::LookPreset;
use snap_core::ops::{
    BwColorFilter, BwParams, BwPreset, ColorHslParams, DetailsParams, EditOp, HslChannelShift,
    RawDevelopParams, TuneParams, VignetteParams, WbPreset, WhiteBalanceParams,
};
use snap_core::stack::EditStack;

use crate::error::IoError;

/// Check if a given path has an Adobe `.xmp` extension (case-insensitive).
pub fn is_xmp_path(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("xmp"))
        .unwrap_or(false)
}

/// Convert a Kelvin color temperature in the 2000K..12000K range into a normalized [-100..+100]
/// adjustment relative to a 5500K daylight neutral point.
pub fn kelvin_to_temp(kelvin: f32) -> f32 {
    let k = kelvin.clamp(2000.0, 12000.0);
    if k < 5500.0 {
        ((k - 5500.0) / 3500.0 * 100.0).clamp(-100.0, 100.0)
    } else {
        ((k - 5500.0) / 6500.0 * 100.0).clamp(-100.0, 100.0)
    }
}

/// Parse an Adobe Camera Raw / Lightroom `.xmp` XML/RDF preset string into a `LookPreset`.
///
/// Supports both attribute syntax (`crs:Exposure2012="+0.65"`) and child-element syntax
/// (`<crs:Exposure2012>+0.65</crs:Exposure2012>`), tone curves, 8-band HSL, monochrome, and vignettes.
pub fn parse_xmp_str(xml: &str, fallback_name: &str) -> Result<LookPreset, IoError> {
    let lower = xml.to_ascii_lowercase();
    if !lower.contains("<x:xmpmeta") || !lower.contains("<rdf:rdf") || !lower.contains("crs:") {
        return Err(IoError::XmpParse(
            "Input is not a valid Adobe XMP preset (missing <x:xmpmeta, <rdf:RDF, or crs:)"
                .to_string(),
        ));
    }

    let root = parse_xml_tree(xml)?;

    let name = extract_preset_name(&root, fallback_name);
    let id = generate_look_id(&name);

    let mut stack = EditStack::default();

    // 1. Exposure and Tone Parameters
    let exposure = get_float_prop(&root, &["Exposure2012", "Exposure"]).unwrap_or(0.0);
    let contrast = get_float_prop(&root, &["Contrast2012", "Contrast"]).unwrap_or(0.0);
    let highlights = get_float_prop(&root, &["Highlights2012", "Highlights"]).unwrap_or(0.0);
    let shadows = get_float_prop(&root, &["Shadows2012", "Shadows"]).unwrap_or(0.0);
    let whites = get_float_prop(&root, &["Whites2012", "Whites"]).unwrap_or(0.0);
    let blacks = get_float_prop(&root, &["Blacks2012", "Blacks"]).unwrap_or(0.0);
    let vibrance = get_float_prop(&root, &["Vibrance"]).unwrap_or(0.0);
    let saturation = get_float_prop(&root, &["Saturation"]).unwrap_or(0.0);

    let temp_raw = get_float_prop(&root, &["Temperature"]);
    let inc_temp = get_float_prop(&root, &["IncrementalTemperature"]);
    let tint_raw = get_float_prop(&root, &["Tint"]);
    let inc_tint = get_float_prop(&root, &["IncrementalTint"]);

    let (is_kelvin, kelvin_temp, kelvin_tint) = if let Some(k) = temp_raw {
        if k > 100.0 {
            let t = kelvin_to_temp(k);
            let tint = tint_raw.or(inc_tint).unwrap_or(0.0).clamp(-100.0, 100.0);
            (true, t, tint)
        } else {
            (false, 0.0, 0.0)
        }
    } else {
        (false, 0.0, 0.0)
    };

    let non_kelvin_temp = inc_temp.or(temp_raw).unwrap_or(0.0).clamp(-100.0, 100.0);
    let non_kelvin_tint = inc_tint.or(tint_raw).unwrap_or(0.0).clamp(-100.0, 100.0);

    // 1a. RawDevelop (if exposure != 0 or Kelvin WB present)
    if exposure.abs() > 1e-4 || is_kelvin {
        let raw_params = RawDevelopParams {
            exposure_ev: exposure.clamp(-8.0, 8.0),
            highlights: 0.0,
            shadows: 0.0,
            contrast: 0.0,
            structure: 0.0,
            temperature: if is_kelvin { kelvin_temp } else { 0.0 },
            tint: if is_kelvin { kelvin_tint } else { 0.0 },
            saturation: 0.0,
            wb_preset: if is_kelvin {
                WbPreset::Custom
            } else {
                WbPreset::AsShot
            },
        };
        if !raw_params.is_identity() {
            stack.push_op(EditOp::RawDevelop(raw_params));
        }
    }

    // 1b. WhiteBalance (if not Kelvin, and tint != 0)
    let mut warmth = 0.0;
    if !is_kelvin {
        if non_kelvin_tint.abs() > 1e-4 {
            let wb = WhiteBalanceParams {
                temperature: non_kelvin_temp,
                tint: non_kelvin_tint,
            };
            if !wb.is_identity() {
                stack.push_op(EditOp::WhiteBalance(wb));
            }
        } else {
            warmth = non_kelvin_temp;
        }
    }

    // 1c. TuneImage (mapping whites/blacks/vibrance into brightness/ambiance/saturation)
    let brightness = (whites * 0.35 + blacks * 0.25).clamp(-100.0, 100.0);
    let ambiance = (vibrance * 0.5 + (whites - blacks) * 0.1).clamp(-100.0, 100.0);
    let sat_mapped = (saturation + vibrance * 0.3).clamp(-100.0, 100.0);

    let tune = TuneParams {
        brightness,
        contrast: contrast.clamp(-100.0, 100.0),
        saturation: sat_mapped,
        ambiance,
        highlights: highlights.clamp(-100.0, 100.0),
        shadows: shadows.clamp(-100.0, 100.0),
        warmth,
    };
    if !tune.is_identity() {
        stack.push_op(EditOp::TuneImage(tune));
    }

    // 2. Tone Curves
    let mut curves = CurvesParams::default();
    if let Some(rgb_curve) = extract_curve_channel(&root, &["ToneCurvePV2012", "ToneCurve"]) {
        curves.rgb = rgb_curve;
    }
    if let Some(red_curve) = extract_curve_channel(&root, &["ToneCurvePV2012Red", "ToneCurveRed"]) {
        curves.red = red_curve;
    }
    if let Some(green_curve) =
        extract_curve_channel(&root, &["ToneCurvePV2012Green", "ToneCurveGreen"])
    {
        curves.green = green_curve;
    }
    if let Some(blue_curve) =
        extract_curve_channel(&root, &["ToneCurvePV2012Blue", "ToneCurveBlue"])
    {
        curves.blue = blue_curve;
    }
    if !curves.is_identity() {
        stack.push_op(EditOp::Curves(curves));
    }

    // 3. 8-band HSL
    // Adobe names: Red, Orange, Yellow, Green, Aqua, Blue, Purple, Magenta
    // Snap bands: 0:Red, 1:Orange, 2:Yellow, 3:Green, 4:Cyan (Aqua), 5:Blue, 6:Purple, 7:Magenta
    const HSL_NAMES: [&str; 8] = [
        "Red", "Orange", "Yellow", "Green", "Aqua", "Blue", "Purple", "Magenta",
    ];
    let mut bands = [HslChannelShift::default(); 8];
    for (i, name) in HSL_NAMES.iter().enumerate() {
        let hue_key = format!("HueAdjustment{name}");
        let sat_key = format!("SaturationAdjustment{name}");
        let lum_key = format!("LuminanceAdjustment{name}");

        let hue = get_float_prop(&root, &[&hue_key]).unwrap_or(0.0).clamp(-100.0, 100.0);
        let sat = get_float_prop(&root, &[&sat_key]).unwrap_or(0.0).clamp(-100.0, 100.0);
        let lum = get_float_prop(&root, &[&lum_key]).unwrap_or(0.0).clamp(-100.0, 100.0);

        bands[i] = HslChannelShift {
            hue,
            saturation: sat,
            luminance: lum,
        };
    }
    let color_hsl = ColorHslParams { bands };
    if !color_hsl.is_identity() {
        stack.push_op(EditOp::ColorHsl(color_hsl));
    }

    // 4. Details (Clarity, Texture, Sharpness)
    let clarity = get_float_prop(&root, &["Clarity2012", "Clarity"]).unwrap_or(0.0);
    let texture = get_float_prop(&root, &["Texture"]).unwrap_or(0.0);
    let sharpness = get_float_prop(&root, &["Sharpness"]).unwrap_or(0.0);

    let structure = (clarity * 0.7 + texture * 0.5).clamp(-100.0, 100.0);
    let sharpening = sharpness.clamp(0.0, 100.0);
    let details = DetailsParams {
        structure,
        sharpening,
    };
    if !details.is_identity() {
        stack.push_op(EditOp::Details(details));
    }

    // 5. Grayscale Conversion (Monochrome)
    let convert_to_grayscale =
        get_bool_prop(&root, &["ConvertToGrayscale"]).unwrap_or(false);
    if convert_to_grayscale {
        let grain = get_float_prop(&root, &["GrainAmount"])
            .unwrap_or(0.0)
            .clamp(0.0, 100.0);
        let bw = BwParams {
            preset: BwPreset::Neutral,
            color_filter: BwColorFilter::Neutral,
            brightness: 0.0,
            contrast: 0.0,
            grain,
        };
        stack.push_op(EditOp::BlackAndWhite(bw));
    }

    // 6. Post-Crop Vignette
    let vignette_amount =
        get_float_prop(&root, &["PostCropVignetteAmount"]).unwrap_or(0.0);
    if vignette_amount.abs() > 1e-4 {
        let midpoint = get_float_prop(&root, &["PostCropVignetteMidpoint"])
            .unwrap_or(50.0)
            .clamp(0.0, 100.0);
        let norm_mid = midpoint / 100.0;
        let inner_radius = (norm_mid * 0.5).clamp(0.05, 0.8);
        let outer_radius = (0.5 + norm_mid * 0.45).clamp(0.6, 1.0);
        let vignette = VignetteParams {
            center: [0.5, 0.5],
            inner_radius,
            outer_radius,
            outer_brightness: vignette_amount.clamp(-100.0, 100.0),
            inner_brightness: 0.0,
        };
        if !vignette.is_identity() {
            stack.push_op(EditOp::Vignette(vignette));
        }
    }

    Ok(LookPreset {
        id,
        name,
        built_in: false,
        stack,
    })
}

/// Import an Adobe `.xmp` preset file from the specified filesystem path.
pub fn import_xmp_preset(path: &Path) -> Result<LookPreset, IoError> {
    if !is_xmp_path(path) {
        return Err(IoError::UnsupportedFormat(format!(
            "Expected .xmp file, got: {}",
            path.display()
        )));
    }
    let content = fs::read_to_string(path)?;
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("custom_look");
    parse_xmp_str(&content, stem)
}

/// Install an Adobe `.xmp` preset to a custom target looks directory.
///
/// Imports the `.xmp` preset and saves the resulting `LookPreset` JSON as `<id>.json`
/// in `dest_dir`.
pub fn install_xmp_preset_to(path: &Path, dest_dir: &Path) -> Result<LookPreset, IoError> {
    let preset = import_xmp_preset(path)?;
    fs::create_dir_all(dest_dir)?;
    let filename = format!("{}.json", preset.id);
    let dest_file = dest_dir.join(filename);
    let json = serde_json::to_string_pretty(&preset)?;
    fs::write(dest_file, json)?;
    Ok(preset)
}

/// Install an Adobe `.xmp` preset into the application looks directory (`AppConfig::looks_dir()`).
pub fn install_xmp_preset(path: &Path) -> Result<LookPreset, IoError> {
    install_xmp_preset_to(path, &AppConfig::looks_dir())
}

// ---------------------------------------------------------------------------
// Pure-Rust XML/RDF Parsing without external dependencies
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct XmlNode {
    pub tag: String,
    pub attributes: Vec<(String, String)>,
    pub children: Vec<XmlNode>,
    pub text: String,
}

impl XmlNode {
    pub fn tag_local_name(&self) -> &str {
        if let Some(idx) = self.tag.find(':') {
            &self.tag[idx + 1..]
        } else {
            &self.tag
        }
    }

    pub fn get_attribute(&self, name: &str) -> Option<&str> {
        for (k, v) in &self.attributes {
            let local_k = if let Some(idx) = k.find(':') {
                &k[idx + 1..]
            } else {
                k.as_str()
            };
            if local_k.eq_ignore_ascii_case(name) || k.eq_ignore_ascii_case(name) {
                return Some(v.as_str());
            }
        }
        None
    }

    pub fn find_descendant_by_tags(&self, target_tags: &[&str]) -> Option<&XmlNode> {
        let local_tag = self.tag_local_name();
        for target in target_tags {
            if local_tag.eq_ignore_ascii_case(target) || self.tag.eq_ignore_ascii_case(target) {
                return Some(self);
            }
        }
        for child in &self.children {
            if let Some(found) = child.find_descendant_by_tags(target_tags) {
                return Some(found);
            }
        }
        None
    }

    pub fn find_all_descendants_by_tag<'a>(&'a self, target_tag: &str) -> Vec<&'a XmlNode> {
        let mut results = Vec::new();
        self.collect_descendants(target_tag, &mut results);
        results
    }

    fn collect_descendants<'a>(&'a self, target_tag: &str, results: &mut Vec<&'a XmlNode>) {
        let local_tag = self.tag_local_name();
        if local_tag.eq_ignore_ascii_case(target_tag) || self.tag.eq_ignore_ascii_case(target_tag) {
            results.push(self);
        }
        for child in &self.children {
            child.collect_descendants(target_tag, results);
        }
    }
}

fn get_str_prop(root: &XmlNode, names: &[&str]) -> Option<String> {
    // 1. Check attributes across all nodes
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        for name in names {
            if let Some(val) = node.get_attribute(name) {
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
        for child in node.children.iter().rev() {
            stack.push(child);
        }
    }

    // 2. Check child element tag and text
    if let Some(node) = root.find_descendant_by_tags(names) {
        let trimmed = node.text.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    None
}

fn get_float_prop(root: &XmlNode, names: &[&str]) -> Option<f32> {
    let s = get_str_prop(root, names)?;
    parse_float(&s)
}

fn get_bool_prop(root: &XmlNode, names: &[&str]) -> Option<bool> {
    let s = get_str_prop(root, names)?;
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Some(true),
        "false" | "0" | "no" => Some(false),
        _ => None,
    }
}

fn parse_float(s: &str) -> Option<f32> {
    let s = s.trim().trim_start_matches('+');
    s.parse::<f32>().ok()
}

fn extract_preset_name(root: &XmlNode, fallback_name: &str) -> String {
    if let Some(name_node) = root.find_descendant_by_tags(&["Name", "PresetName"]) {
        if let Some(li_node) = name_node.find_descendant_by_tags(&["li"]) {
            let t = li_node.text.trim();
            if !t.is_empty() {
                return t.to_string();
            }
        }
        let t = name_node.text.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }

    if let Some(name_attr) = get_str_prop(root, &["Name", "PresetName"]) {
        let t = name_attr.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }

    let fb = fallback_name.trim();
    if !fb.is_empty() {
        fb.to_string()
    } else {
        "Custom Look".to_string()
    }
}

fn generate_look_id(name: &str) -> String {
    let mut id = String::new();
    let mut prev_underscore = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            id.push(c.to_ascii_lowercase());
            prev_underscore = false;
        } else if !prev_underscore {
            id.push('_');
            prev_underscore = true;
        }
    }
    let trimmed = id.trim_matches('_');
    if trimmed.is_empty() {
        "custom_look".to_string()
    } else {
        trimmed.to_string()
    }
}

fn parse_curve_point(s: &str) -> Option<CurvePoint> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() == 2 {
        let x = parse_float(parts[0])?;
        let y = parse_float(parts[1])?;
        Some(CurvePoint::new(x / 255.0, y / 255.0))
    } else {
        None
    }
}

fn parse_curve_points_from_text(s: &str) -> Vec<CurvePoint> {
    let mut points = Vec::new();
    for line in s.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(pt) = parse_curve_point(line) {
            points.push(pt);
        }
    }
    if points.len() >= 2 {
        return points;
    }

    let tokens: Vec<&str> = s
        .split(|c: char| c == ',' || c.is_whitespace())
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .collect();

    let mut i = 0;
    let mut fallback_points = Vec::new();
    while i + 1 < tokens.len() {
        if let (Some(x), Some(y)) = (parse_float(tokens[i]), parse_float(tokens[i + 1])) {
            fallback_points.push(CurvePoint::new(x / 255.0, y / 255.0));
            i += 2;
        } else {
            i += 1;
        }
    }
    fallback_points
}

fn extract_curve_channel(root: &XmlNode, names: &[&str]) -> Option<CurveChannel> {
    if let Some(node) = root.find_descendant_by_tags(names) {
        let li_nodes = node.find_all_descendants_by_tag("li");
        let mut points = Vec::new();
        if !li_nodes.is_empty() {
            for li in li_nodes {
                if let Some(pt) = parse_curve_point(&li.text) {
                    points.push(pt);
                }
            }
        } else {
            points = parse_curve_points_from_text(&node.text);
        }
        if points.len() >= 2 {
            let channel = CurveChannel::new(points);
            if !channel.is_identity() {
                return Some(channel);
            }
        }
    }

    if let Some(attr_val) = get_str_prop(root, names) {
        let points = parse_curve_points_from_text(&attr_val);
        if points.len() >= 2 {
            let channel = CurveChannel::new(points);
            if !channel.is_identity() {
                return Some(channel);
            }
        }
    }

    None
}

fn tags_match(a: &str, b: &str) -> bool {
    if a.eq_ignore_ascii_case(b) {
        return true;
    }
    let local_a = if let Some(i) = a.find(':') {
        &a[i + 1..]
    } else {
        a
    };
    let local_b = if let Some(i) = b.find(':') {
        &b[i + 1..]
    } else {
        b
    };
    local_a.eq_ignore_ascii_case(local_b)
}

// ---------------------------------------------------------------------------
// XML Lexer & Tree Builder
// ---------------------------------------------------------------------------

struct XmlParser<'a> {
    chars: &'a str,
    pos: usize,
}

impl<'a> XmlParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input,
            pos: 0,
        }
    }

    fn remaining(&self) -> &'a str {
        if self.pos < self.chars.len() {
            &self.chars[self.pos..]
        } else {
            ""
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn skip_whitespace(&mut self) {
        while !self.is_eof() {
            let rem = self.remaining();
            if let Some(c) = rem.chars().next() {
                if c.is_whitespace() {
                    self.pos += c.len_utf8();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    fn skip_comments_and_prolog(&mut self) {
        loop {
            self.skip_whitespace();
            let rem = self.remaining();
            if rem.starts_with("<!--") {
                if let Some(end) = rem.find("-->") {
                    self.pos += end + 3;
                } else {
                    self.pos = self.chars.len();
                    break;
                }
            } else if rem.starts_with("<?") {
                if let Some(end) = rem.find("?>") {
                    self.pos += end + 2;
                } else {
                    self.pos = self.chars.len();
                    break;
                }
            } else if rem.starts_with("<!DOCTYPE") || rem.starts_with("<!doctype") {
                if let Some(end) = rem.find('>') {
                    self.pos += end + 1;
                } else {
                    self.pos = self.chars.len();
                    break;
                }
            } else {
                break;
            }
        }
    }

    fn parse_node(&mut self) -> Result<Option<XmlNode>, IoError> {
        self.skip_comments_and_prolog();
        if self.is_eof() {
            return Ok(None);
        }

        let rem = self.remaining();
        if !rem.starts_with('<') {
            return Ok(None);
        }

        if rem.starts_with("</") {
            return Ok(None);
        }

        self.pos += 1;

        let mut tag = String::new();
        while !self.is_eof() {
            let rem = self.remaining();
            if let Some(c) = rem.chars().next() {
                if c.is_whitespace() || c == '/' || c == '>' {
                    break;
                } else {
                    tag.push(c);
                    self.pos += c.len_utf8();
                }
            } else {
                break;
            }
        }

        if tag.is_empty() {
            return Err(IoError::XmpParse("Empty XML tag name".to_string()));
        }

        let mut attributes = Vec::new();
        loop {
            self.skip_whitespace();
            if self.is_eof() {
                return Err(IoError::XmpParse(format!("Unclosed tag <{tag}")));
            }
            let rem = self.remaining();
            if rem.starts_with("/>") {
                self.pos += 2;
                return Ok(Some(XmlNode {
                    tag,
                    attributes,
                    children: Vec::new(),
                    text: String::new(),
                }));
            }
            if rem.starts_with('>') {
                self.pos += 1;
                break;
            }

            let mut key = String::new();
            while !self.is_eof() {
                let rem = self.remaining();
                if let Some(c) = rem.chars().next() {
                    if c.is_whitespace() || c == '=' || c == '/' || c == '>' {
                        break;
                    } else {
                        key.push(c);
                        self.pos += c.len_utf8();
                    }
                } else {
                    break;
                }
            }

            self.skip_whitespace();
            let mut val = String::new();
            if self.remaining().starts_with('=') {
                self.pos += 1;
                self.skip_whitespace();
                let rem = self.remaining();
                if rem.starts_with('"') {
                    self.pos += 1;
                    while !self.is_eof() {
                        let rem = self.remaining();
                        if let Some(c) = rem.chars().next() {
                            self.pos += c.len_utf8();
                            if c == '"' {
                                break;
                            } else {
                                val.push(c);
                            }
                        }
                    }
                } else if rem.starts_with('\'') {
                    self.pos += 1;
                    while !self.is_eof() {
                        let rem = self.remaining();
                        if let Some(c) = rem.chars().next() {
                            self.pos += c.len_utf8();
                            if c == '\'' {
                                break;
                            } else {
                                val.push(c);
                            }
                        }
                    }
                } else {
                    while !self.is_eof() {
                        let rem = self.remaining();
                        if let Some(c) = rem.chars().next() {
                            if c.is_whitespace() || c == '/' || c == '>' {
                                break;
                            } else {
                                val.push(c);
                                self.pos += c.len_utf8();
                            }
                        }
                    }
                }
            }

            if !key.is_empty() {
                attributes.push((key, unescape_xml(&val)));
            }
        }

        let mut children = Vec::new();
        let mut text_acc = String::new();

        loop {
            self.skip_comments_and_prolog();
            if self.is_eof() {
                return Err(IoError::XmpParse(format!("Unclosed tag <{tag}>")));
            }

            let rem = self.remaining();
            if rem.starts_with("</") {
                let closing_rem = &rem[2..];
                let end_bracket = closing_rem.find('>').ok_or_else(|| {
                    IoError::XmpParse(format!("Malformed closing tag in <{tag}>"))
                })?;
                let closing_tag = closing_rem[..end_bracket].trim();
                if tags_match(closing_tag, &tag) {
                    self.pos += 2 + end_bracket + 1;
                    break;
                } else {
                    // Closing tag belongs to parent/ancestor
                    break;
                }
            } else if rem.starts_with("<![CDATA[") {
                self.pos += 9;
                let cdata_rem = self.remaining();
                if let Some(end) = cdata_rem.find("]]>") {
                    text_acc.push_str(&cdata_rem[..end]);
                    self.pos += end + 3;
                } else {
                    text_acc.push_str(cdata_rem);
                    self.pos = self.chars.len();
                    break;
                }
            } else if rem.starts_with('<') {
                if let Some(child) = self.parse_node()? {
                    children.push(child);
                }
            } else {
                let next_angle = rem.find('<').unwrap_or(rem.len());
                let text_segment = &rem[..next_angle];
                text_acc.push_str(text_segment);
                self.pos += next_angle;
            }
        }

        Ok(Some(XmlNode {
            tag,
            attributes,
            children,
            text: unescape_xml(text_acc.trim()),
        }))
    }
}

fn unescape_xml(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            let mut entity = String::new();
            while let Some(&nc) = chars.peek() {
                chars.next();
                if nc == ';' {
                    break;
                }
                entity.push(nc);
                if entity.len() > 10 {
                    break;
                }
            }
            match entity.as_str() {
                "lt" => out.push('<'),
                "gt" => out.push('>'),
                "amp" => out.push('&'),
                "quot" => out.push('"'),
                "apos" => out.push('\''),
                _ => {
                    out.push('&');
                    out.push_str(&entity);
                    out.push(';');
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn parse_xml_tree(xml: &str) -> Result<XmlNode, IoError> {
    let mut parser = XmlParser::new(xml);
    let mut root_nodes = Vec::new();
    while !parser.is_eof() {
        parser.skip_comments_and_prolog();
        if parser.is_eof() {
            break;
        }
        if let Some(node) = parser.parse_node()? {
            root_nodes.push(node);
        } else {
            break;
        }
    }

    if root_nodes.is_empty() {
        return Err(IoError::XmpParse("Empty XML document".to_string()));
    }

    if root_nodes.len() == 1 {
        Ok(root_nodes.remove(0))
    } else {
        Ok(XmlNode {
            tag: "root".to_string(),
            attributes: Vec::new(),
            children: root_nodes,
            text: String::new(),
        })
    }
}
