use std::io::{self, Read, Write};
use std::process;

use clap::Parser;
use serde::Serialize;

#[derive(Parser)]
#[command(name = "qr-score", about = "Measure QR code scannability from SVG")]
struct Cli {
    /// Path to TOML config file
    #[arg(long = "config")]
    config_path: Option<String>,

    /// Override render size (base rasterization size in pixels)
    #[arg(long)]
    render_size: Option<u32>,

    /// Render SVG to PNG and output to stdout (replaces external resvg)
    #[arg(long)]
    render: bool,

    /// DPI for SVG unit conversion (default 96, used with --render)
    #[arg(long, default_value = "96")]
    dpi: f32,

    /// Zoom factor for render output (e.g., 20 = 20x, used with --render)
    #[arg(short = 'z', long, default_value = "1")]
    zoom: f32,

    /// Dump rendered PNG to this path instead of scoring
    #[arg(long)]
    dump_png: Option<String>,
}

#[derive(Serialize)]
struct Output {
    score: u8,
    grade: String,
    decodable: bool,
    content: Option<String>,
    results: qr_score::StressResults,
    contrast_ratio: u8,
    error_correction: Option<String>,
}

#[derive(Serialize)]
struct ErrorOutput {
    score: u8,
    grade: String,
    decodable: bool,
    error: String,
}

fn error_json(error: &str) -> String {
    serde_json::to_string(&ErrorOutput {
        score: 0,
        grade: "F".to_string(),
        decodable: false,
        error: error.to_string(),
    })
    .unwrap()
}

fn main() {
    let cli = Cli::parse();

    std::panic::set_hook(Box::new(|_| {}));

    let mut svg_data = Vec::new();
    if let Err(e) = io::stdin().read_to_end(&mut svg_data) {
        println!("{}", error_json(&format!("Failed to read stdin: {}", e)));
        process::exit(1);
    }

    if svg_data.is_empty() {
        eprintln!("No input provided");
        process::exit(0);
    }

    if cli.render {
        let Some(png) = qr_score::render::svg_to_png_hq(&svg_data, cli.dpi, cli.zoom) else {
            eprintln!("Failed to render SVG");
            process::exit(1);
        };
        if let Err(e) = io::stdout().write_all(&png) {
            eprintln!("Failed to write PNG: {}", e);
            process::exit(1);
        }
        return;
    }

    if let Some(ref dump_path) = cli.dump_png {
        let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())
            .unwrap_or_else(|e| { eprintln!("Invalid SVG: {}", e); process::exit(1) });
        let config = qr_score::render::load_config(cli.config_path.as_deref(), cli.render_size);
        let svg_size = tree.size();
        let native = svg_size.width().max(svg_size.height()) as u32;
        let render_size = config.render_size.max(native);

        let Some(png) = qr_score::render::svg_to_png(&tree, render_size) else {
            eprintln!("Failed to render SVG");
            process::exit(1);
        };
        if let Err(e) = std::fs::write(dump_path, &png) {
            eprintln!("Failed to write PNG: {}", e);
            process::exit(1);
        }
        eprintln!("Wrote {} bytes to {}", png.len(), dump_path);
        return;
    }

    match qr_score::render::score_svg_bytes(&svg_data, cli.config_path.as_deref(), cli.render_size) {
        Ok(result) => {
            let sr = &result.stress_results;
            let output = Output {
                score: result.score,
                grade: qr_score::scorer::grade_from_score(result.score).to_string(),
                decodable: result.decodable,
                content: result.content,
                contrast_ratio: (sr.contrast_ratio * 100.0).round() as u8,
                results: sr.clone(),
                error_correction: result.metadata.map(|m| m.error_correction.to_string()),
            };
            println!("{}", serde_json::to_string(&output).unwrap());
        }
        Err(e) => {
            println!("{}", error_json(&e.to_string()));
            process::exit(1);
        }
    }
}