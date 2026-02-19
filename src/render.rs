use std::panic;

pub fn svg_to_png(tree: &usvg::Tree, size: u32) -> Option<Vec<u8>> {
    let mut pixmap = tiny_skia::Pixmap::new(size, size)?;

    let svg_size = tree.size();
    let sx = size as f32 / svg_size.width();
    let sy = size as f32 / svg_size.height();
    let scale = sx.min(sy);

    resvg::render(
        tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    pixmap.encode_png().ok()
}

/// High-quality render with DPI and zoom (used for `--render` CLI mode).
pub fn svg_to_png_hq(svg_data: &[u8], dpi: f32, zoom: f32) -> Option<Vec<u8>> {
    let mut opts = usvg::Options::default();
    opts.dpi = dpi;
    let tree = usvg::Tree::from_data(svg_data, &opts).ok()?;

    let svg_size = tree.size();
    let w = (svg_size.width() * zoom).ceil() as u32;
    let h = (svg_size.height() * zoom).ceil() as u32;

    let mut pixmap = tiny_skia::Pixmap::new(w, h)?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(zoom, zoom),
        &mut pixmap.as_mut(),
    );

    pixmap.encode_png().ok()
}

pub fn load_config(config_path: Option<&str>, render_size_override: Option<u32>) -> crate::TestConfig {
    let mut config = 'load: {
        let Some(path) = config_path else { break 'load crate::TestConfig::default() };
        let contents = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Warning: failed to read config {}: {}", path, e);
                break 'load crate::TestConfig::default();
            }
        };
        match toml::from_str(&contents) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Warning: failed to parse config {}: {}", path, e);
                crate::TestConfig::default()
            }
        }
    };

    config.render_size = render_size_override.unwrap_or(config.render_size);
    config
}

/// Full pipeline: SVG bytes -> parse -> render -> validate.
pub fn score_svg_bytes(
    svg_data: &[u8],
    config_path: Option<&str>,
    render_size_override: Option<u32>,
) -> crate::Result<crate::ValidationResult> {
    let tree = usvg::Tree::from_data(svg_data, &usvg::Options::default())
        .map_err(|e| crate::QrScoreError::InvalidSvg(e.to_string()))?;

    let mut config = load_config(config_path, render_size_override);
    let svg_size = tree.size();
    let native = svg_size.width().max(svg_size.height()) as u32;
    config.native_size = Some(native);
    let render_size = config.render_size.max(native);

    let png_bytes = svg_to_png(&tree, render_size).ok_or(crate::QrScoreError::RenderFailed)?;

    panic::catch_unwind(|| crate::validate(&png_bytes, &config))
        .map_err(|_| crate::QrScoreError::DecodeFailed)?
}