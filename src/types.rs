use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressResults {
    #[serde(flatten)]
    pub tests: BTreeMap<String, bool>,
    #[serde(skip)]
    pub contrast_ratio: f32,
}

impl Default for StressResults {
    fn default() -> Self {
        Self {
            tests: BTreeMap::new(),
            contrast_ratio: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Weights {
    #[serde(flatten)]
    pub tests: BTreeMap<String, u32>,
    pub contrast_ratio: u32,
}

impl Default for Weights {
    fn default() -> Self {
        let tests = BTreeMap::from([
            ("downscale_1x".into(), 1),
            ("downscale_2x".into(), 2),
            ("downscale_3x".into(), 2),
            ("downscale_4x".into(), 2),
            ("blur_light".into(), 2),
            ("blur_heavy".into(), 1),
            ("contrast_up".into(), 2),
            ("contrast_down".into(), 2),
            ("contrast_strict_up".into(), 1),
            ("contrast_strict_down".into(), 1),
            ("luminance_up".into(), 2),
            ("luminance_down".into(), 2),
            ("luminance_strict_up".into(), 1),
            ("luminance_strict_down".into(), 1),
            ("hue_up".into(), 1),
            ("hue_down".into(), 1),
            ("hue_strict_up".into(), 1),
            ("hue_strict_down".into(), 1),
            ("saturation_up".into(), 1),
            ("saturation_down".into(), 1),
            ("saturation_strict_up".into(), 1),
            ("saturation_strict_down".into(), 1),
        ]);
        Self {
            tests,
            contrast_ratio: 70,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    #[serde(default = "default_render_size")]
    pub render_size: u32,
    #[serde(skip)]
    pub native_size: Option<u32>,
    #[serde(default = "default_blur_light_sigma")]
    pub blur_light_sigma: f32,
    #[serde(default = "default_blur_heavy_sigma")]
    pub blur_heavy_sigma: f32,
    #[serde(default = "default_contrast")]
    pub contrast: f32,
    #[serde(default = "default_contrast_strict")]
    pub contrast_strict: f32,
    #[serde(default = "default_luminance")]
    pub luminance: i32,
    #[serde(default = "default_luminance_strict")]
    pub luminance_strict: i32,
    #[serde(default = "default_hue")]
    pub hue: f32,
    #[serde(default = "default_hue_strict")]
    pub hue_strict: f32,
    #[serde(default = "default_saturation")]
    pub saturation: f32,
    #[serde(default = "default_saturation_strict")]
    pub saturation_strict: f32,
    #[serde(default)]
    pub weights: Weights,
}

fn default_render_size() -> u32 { 400 }
fn default_blur_light_sigma() -> f32 { 1.0 }
fn default_blur_heavy_sigma() -> f32 { 2.0 }
fn default_contrast() -> f32 { 30.0 }
fn default_contrast_strict() -> f32 { 50.0 }
fn default_luminance() -> i32 { 20 }
fn default_luminance_strict() -> i32 { 40 }
fn default_hue() -> f32 { 45.0 }
fn default_hue_strict() -> f32 { 90.0 }
fn default_saturation() -> f32 { 30.0 }
fn default_saturation_strict() -> f32 { 50.0 }

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            render_size: default_render_size(),
            native_size: None,
            blur_light_sigma: default_blur_light_sigma(),
            blur_heavy_sigma: default_blur_heavy_sigma(),
            contrast: default_contrast(),
            contrast_strict: default_contrast_strict(),
            luminance: default_luminance(),
            luminance_strict: default_luminance_strict(),
            hue: default_hue(),
            hue_strict: default_hue_strict(),
            saturation: default_saturation(),
            saturation_strict: default_saturation_strict(),
            weights: Weights::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub score: u8,
    pub decodable: bool,
    pub content: Option<String>,
    pub metadata: Option<QrMetadata>,
    pub stress_results: StressResults,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrMetadata {
    pub error_correction: ErrorCorrectionLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ErrorCorrectionLevel {
    L,
    #[default]
    M,
    Q,
    H,
}

impl fmt::Display for ErrorCorrectionLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::L => write!(f, "L"),
            Self::M => write!(f, "M"),
            Self::Q => write!(f, "Q"),
            Self::H => write!(f, "H"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodeResult {
    pub content: String,
    pub metadata: Option<QrMetadata>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = TestConfig::default();
        assert_eq!(config.render_size, 400);
        assert_eq!(config.contrast, 30.0);
        assert_eq!(config.contrast_strict, 50.0);
        assert_eq!(config.luminance, 20);
        assert_eq!(config.luminance_strict, 40);
        assert_eq!(config.hue, 45.0);
        assert_eq!(config.hue_strict, 90.0);
        assert!(config.native_size.is_none());
    }

    #[test]
    fn weights_default_sum_to_100() {
        let w = Weights::default();
        let sum: u32 = w.tests.values().sum::<u32>() + w.contrast_ratio;
        assert_eq!(sum, 100);
    }

    #[test]
    fn stress_results_default_all_false() {
        let sr = StressResults::default();
        assert!(sr.tests.is_empty());
        assert_eq!(sr.contrast_ratio, 0.0);
    }

    #[test]
    fn error_correction_level_display() {
        assert_eq!(format!("{}", ErrorCorrectionLevel::L), "L");
        assert_eq!(format!("{}", ErrorCorrectionLevel::M), "M");
        assert_eq!(format!("{}", ErrorCorrectionLevel::Q), "Q");
        assert_eq!(format!("{}", ErrorCorrectionLevel::H), "H");
    }

    #[test]
    fn weights_default_has_expected_keys() {
        let w = Weights::default();
        let expected = [
            "downscale_1x", "downscale_2x", "downscale_3x", "downscale_4x",
            "blur_light", "blur_heavy",
            "contrast_up", "contrast_down", "contrast_strict_up", "contrast_strict_down",
            "luminance_up", "luminance_down", "luminance_strict_up", "luminance_strict_down",
            "hue_up", "hue_down", "hue_strict_up", "hue_strict_down",
            "saturation_up", "saturation_down", "saturation_strict_up", "saturation_strict_down",
        ];
        for key in expected {
            assert!(w.tests.contains_key(key), "missing weight key: {}", key);
        }
        assert_eq!(w.tests.len(), 22);
    }

    #[test]
    fn config_from_toml() {
        let toml_str = r#"
            render_size = 512
            contrast = 25.0
            contrast_strict = 60.0
        "#;
        let config: TestConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.render_size, 512);
        assert_eq!(config.contrast, 25.0);
        assert_eq!(config.contrast_strict, 60.0);
        assert_eq!(config.luminance, 20);
        assert_eq!(config.hue, 45.0);
    }
}
