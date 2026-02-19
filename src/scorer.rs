use crate::decoder::try_decode;
use crate::types::{StressResults, TestConfig};
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, RgbImage};
use rayon::prelude::*;
use std::collections::BTreeMap;

pub fn validate(img: &DynamicImage, config: &TestConfig) -> (StressResults, u8) {
    let stress = run_stress_tests(img, config);
    let score = calculate_score(&stress, &config.weights);
    (stress, score)
}

fn run_stress_tests(img: &DynamicImage, config: &TestConfig) -> StressResults {
    let contrast_ratio = measure_contrast(img);

    let native = config.native_size.unwrap_or(100);
    let variants: Vec<(&str, DynamicImage)> = vec![
        ("downscale_1x", resize_to(img, native)),
        ("downscale_2x", resize_to(img, native * 2)),
        ("downscale_3x", resize_to(img, native * 3)),
        ("downscale_4x", resize_to(img, native * 4)),
        ("blur_light", apply_blur(img, config.blur_light_sigma)),
        ("blur_heavy", apply_blur(img, config.blur_heavy_sigma)),
        ("contrast_up", adjust_contrast(img, config.contrast)),
        ("contrast_down", adjust_contrast(img, -config.contrast)),
        ("contrast_strict_up", adjust_contrast(img, config.contrast_strict)),
        ("contrast_strict_down", adjust_contrast(img, -config.contrast_strict)),
        ("luminance_up", adjust_luminance(img, config.luminance)),
        ("luminance_down", adjust_luminance(img, -config.luminance)),
        ("luminance_strict_up", adjust_luminance(img, config.luminance_strict)),
        ("luminance_strict_down", adjust_luminance(img, -config.luminance_strict)),
        ("hue_up", shift_hue(img, config.hue)),
        ("hue_down", shift_hue(img, -config.hue)),
        ("hue_strict_up", shift_hue(img, config.hue_strict)),
        ("hue_strict_down", shift_hue(img, -config.hue_strict)),
        ("saturation_up", adjust_saturation(img, config.saturation)),
        ("saturation_down", adjust_saturation(img, -config.saturation)),
        ("saturation_strict_up", adjust_saturation(img, config.saturation_strict)),
        ("saturation_strict_down", adjust_saturation(img, -config.saturation_strict)),
    ];

    let tests: BTreeMap<String, bool> = variants
        .par_iter()
        .map(|(name, variant)| {
            let passed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                try_decode(variant).is_ok()
            }))
            .unwrap_or(false);
            (name.to_string(), passed)
        })
        .collect();

    StressResults {
        tests,
        contrast_ratio,
    }
}

fn calculate_score(stress: &StressResults, weights: &crate::types::Weights) -> u8 {
    let total_weight: u32 = weights.tests.values().sum::<u32>() + weights.contrast_ratio;

    if total_weight == 0 {
        return 0;
    }

    let test_score: f32 = stress.tests.iter()
        .filter(|&(_, &passed)| passed)
        .filter_map(|(name, _)| weights.tests.get(name))
        .map(|&w| w as f32)
        .sum();

    let normalized = (stress.contrast_ratio / 0.7).clamp(0.0, 1.0);
    let score = test_score + normalized * weights.contrast_ratio as f32;

    ((score / total_weight as f32) * 100.0).round().min(100.0) as u8
}

fn resize_to(img: &DynamicImage, size: u32) -> DynamicImage {
    let (w, h) = img.dimensions();
    let max_dim = w.max(h);
    if max_dim <= size {
        return img.clone();
    }
    img.resize(size, size, FilterType::Triangle)
}

fn apply_blur(img: &DynamicImage, sigma: f32) -> DynamicImage {
    img.blur(sigma)
}

fn adjust_contrast(img: &DynamicImage, amount: f32) -> DynamicImage {
    img.adjust_contrast(amount)
}

fn adjust_luminance(img: &DynamicImage, amount: i32) -> DynamicImage {
    img.brighten(amount)
}

fn shift_hue(img: &DynamicImage, degrees: f32) -> DynamicImage {
    img.huerotate(degrees as i32)
}

fn adjust_saturation(img: &DynamicImage, amount: f32) -> DynamicImage {
    let rgb = img.to_rgb8();
    let (width, height) = rgb.dimensions();
    let factor = 1.0 + amount / 100.0;

    let data: Vec<u8> = rgb
        .as_raw()
        .chunks_exact(3)
        .flat_map(|px| {
            let r = px[0] as f32;
            let g = px[1] as f32;
            let b = px[2] as f32;
            let gray = 0.299 * r + 0.587 * g + 0.114 * b;
            [
                (gray + (r - gray) * factor).clamp(0.0, 255.0) as u8,
                (gray + (g - gray) * factor).clamp(0.0, 255.0) as u8,
                (gray + (b - gray) * factor).clamp(0.0, 255.0) as u8,
            ]
        })
        .collect();

    match RgbImage::from_raw(width, height, data) {
        Some(img) => DynamicImage::ImageRgb8(img),
        None => img.clone(),
    }
}

fn srgb_linearize(v: u8) -> f32 {
    let s = v as f32 / 255.0;
    if s <= 0.03928 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

fn relative_luminance(r: u8, g: u8, b: u8) -> f32 {
    0.2126 * srgb_linearize(r) + 0.7152 * srgb_linearize(g) + 0.0722 * srgb_linearize(b)
}

fn measure_contrast(img: &DynamicImage) -> f32 {
    let rgb = img.to_rgb8();
    let raw = rgb.as_raw();

    if raw.is_empty() {
        return 0.0;
    }

    // Compute relative luminance for every pixel, quantized to 1000 bins
    let mut histogram = [0u32; 1001];
    let total = raw.len() / 3;

    for px in raw.chunks_exact(3) {
        let lum = relative_luminance(px[0], px[1], px[2]);
        let bin = (lum * 1000.0).round().min(1000.0) as usize;
        histogram[bin] += 1;
    }

    // Find 5th and 95th percentile luminance
    let p5_target = total as u32 / 20;
    let p95_target = total as u32 - p5_target;

    let mut cumulative = 0u32;
    let mut p5 = 0.0f32;
    let mut p95 = 1.0f32;

    for (i, &count) in histogram.iter().enumerate() {
        let prev = cumulative;
        cumulative += count;
        if prev < p5_target && cumulative >= p5_target {
            p5 = i as f32 / 1000.0;
        }
        if prev < p95_target && cumulative >= p95_target {
            p95 = i as f32 / 1000.0;
            break;
        }
    }

    p95 - p5
}

pub fn grade_from_score(score: u8) -> &'static str {
    match score {
        80.. => "A",
        60.. => "B",
        40.. => "C",
        20.. => "D",
        _ => "F",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Weights;

    fn create_test_qr_image() -> DynamicImage {
        use image::Luma;
        let code = qrcode::QrCode::new(b"https://example.com").unwrap();
        let img = code.render::<Luma<u8>>().build();
        DynamicImage::ImageLuma8(img)
    }

    fn all_pass_stress() -> StressResults {
        let tests: BTreeMap<String, bool> = Weights::default()
            .tests
            .keys()
            .map(|k| (k.clone(), true))
            .collect();
        StressResults {
            tests,
            contrast_ratio: 1.0,
        }
    }

    #[test]
    fn score_all_pass_high_contrast_is_100() {
        assert_eq!(calculate_score(&all_pass_stress(), &Weights::default()), 100);
    }

    #[test]
    fn score_nothing_passes_is_zero() {
        assert_eq!(calculate_score(&StressResults::default(), &Weights::default()), 0);
    }

    #[test]
    fn score_low_contrast_ratio_penalizes() {
        let mut stress = all_pass_stress();
        stress.contrast_ratio = 0.02;
        let score = calculate_score(&stress, &Weights::default());
        assert!(score < 100, "low contrast ratio should reduce score, got {}", score);
    }

    #[test]
    fn clean_qr_scores_high() {
        let img = create_test_qr_image();
        let config = TestConfig::default();
        let (_stress, score) = validate(&img, &config);
        assert!(score >= 50, "clean QR should score >= 50, got {}", score);
    }

    #[test]
    fn contrast_measurement_bw() {
        let img = create_test_qr_image();
        let ratio = measure_contrast(&img);
        assert!(ratio > 0.9, "B&W QR contrast should be near 1.0, got {}", ratio);
    }

    #[test]
    fn score_zero_total_weight_is_zero() {
        let mut weights = Weights::default();
        weights.tests.clear();
        weights.contrast_ratio = 0;
        assert_eq!(calculate_score(&all_pass_stress(), &weights), 0);
    }

    #[test]
    fn score_partial_pass_is_between_bounds() {
        let mut stress = all_pass_stress();
        // fail all downscale tests
        for key in ["downscale_1x", "downscale_2x", "downscale_3x", "downscale_4x"] {
            stress.tests.insert(key.to_string(), false);
        }
        let score = calculate_score(&stress, &Weights::default());
        assert!(score > 0 && score < 100, "partial pass should score between 0 and 100, got {}", score);
    }

    #[test]
    fn contrast_measurement_uniform_image() {
        // A solid grey image has no contrast spread — p95 and p5 converge
        let img = DynamicImage::new_rgb8(100, 100); // all black
        let ratio = measure_contrast(&img);
        assert!(ratio < 0.01, "uniform image contrast should be near 0, got {}", ratio);
    }

    #[test]
    fn grade_boundaries() {
        assert_eq!(grade_from_score(100), "A");
        assert_eq!(grade_from_score(80), "A");
        assert_eq!(grade_from_score(79), "B");
        assert_eq!(grade_from_score(60), "B");
        assert_eq!(grade_from_score(59), "C");
        assert_eq!(grade_from_score(40), "C");
        assert_eq!(grade_from_score(39), "D");
        assert_eq!(grade_from_score(20), "D");
        assert_eq!(grade_from_score(19), "F");
        assert_eq!(grade_from_score(0), "F");
    }
}