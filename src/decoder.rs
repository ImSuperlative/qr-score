use crate::error::{QrScoreError, Result};
use crate::types::{DecodeResult, ErrorCorrectionLevel, QrMetadata};
use image::{DynamicImage, GrayImage};
use rxing::common::{GlobalHistogramBinarizer, HybridBinarizer};
use rxing::{
    BarcodeFormat, Binarizer, BinaryBitmap, DecodeHints, Luma8LuminanceSource,
    MultiFormatReader, Reader, RXingResultMetadataType, RXingResultMetadataValue,
};
use std::collections::HashSet;

#[derive(Debug, Clone)]
struct RawDecode {
    content: String,
    error_correction: Option<ErrorCorrectionLevel>,
}

impl RawDecode {
    fn into_result(self) -> DecodeResult {
        DecodeResult {
            content: self.content,
            metadata: Some(QrMetadata {
                error_correction: self.error_correction.unwrap_or(ErrorCorrectionLevel::M),
            }),
        }
    }
}

fn rxing_hints() -> DecodeHints {
    DecodeHints {
        AlsoInverted: Some(true),
        TryHarder: Some(true),
        PossibleFormats: Some(HashSet::from([BarcodeFormat::QR_CODE])),
        ..DecodeHints::default()
    }
}

fn decode_rxing_with<B, F>(luma_data: &[u8], width: u32, height: u32, make_binarizer: F) -> Result<RawDecode>
where
    B: Binarizer + 'static,
    F: FnOnce(Luma8LuminanceSource) -> B + Send + 'static,
{
    let luma = luma_data.to_vec();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let source = Luma8LuminanceSource::new(luma, width, height);
        let binarizer = make_binarizer(source);
        let mut bitmap = BinaryBitmap::new(binarizer);
        let mut reader = MultiFormatReader::default();
        reader.decode_with_hints(&mut bitmap, &rxing_hints())
    }));

    let r = result
        .map_err(|_| QrScoreError::DecodeFailed)?
        .map_err(|_| QrScoreError::DecodeFailed)?;

    let ec = r
        .getRXingResultMetadata()
        .get(&RXingResultMetadataType::ERROR_CORRECTION_LEVEL)
        .and_then(|v| match v {
            RXingResultMetadataValue::ErrorCorrectionLevel(s) => parse_ec_level(s),
            _ => None,
        });

    Ok(RawDecode {
        content: r.getText().to_string(),
        error_correction: ec,
    })
}

fn decode_rqrr(luma_data: &[u8], width: u32, height: u32) -> Result<RawDecode> {
    let luma = GrayImage::from_raw(width, height, luma_data.to_vec())
        .ok_or(QrScoreError::DecodeFailed)?;

    let mut prepared = rqrr::PreparedImage::prepare(luma);
    let grids = prepared.detect_grids();
    let grid = grids.first().ok_or(QrScoreError::DecodeFailed)?;
    let (meta, content) = grid.decode().map_err(|_| QrScoreError::DecodeFailed)?;

    Ok(RawDecode {
        content,
        error_correction: Some(convert_rqrr_ec(meta.ecc_level)),
    })
}

/// Try all decoders: rxing hybrid, rxing global histogram, rqrr normal, rqrr inverted.
pub fn try_decode(img: &DynamicImage) -> Result<DecodeResult> {
    let luma = img.to_luma8();
    let (width, height) = luma.dimensions();
    let luma_data = luma.into_raw();

    if let Ok(r) = decode_rxing_with(&luma_data, width, height, HybridBinarizer::new) {
        return Ok(r.into_result());
    }

    if let Ok(r) = decode_rxing_with(&luma_data, width, height, GlobalHistogramBinarizer::new) {
        return Ok(r.into_result());
    }

    if let Ok(r) = decode_rqrr(&luma_data, width, height) {
        return Ok(r.into_result());
    }

    let inverted: Vec<u8> = luma_data.iter().map(|&v| 255 - v).collect();
    if let Ok(r) = decode_rqrr(&inverted, width, height) {
        return Ok(r.into_result());
    }

    Err(QrScoreError::DecodeFailed)
}

/// Decode from raw image bytes (PNG, JPEG, etc.)
pub fn multi_decode(image_bytes: &[u8]) -> Result<DecodeResult> {
    let img = image::load_from_memory(image_bytes)
        .map_err(|e| QrScoreError::ImageLoad(e.to_string()))?;
    try_decode(&img)
}

fn parse_ec_level(s: &str) -> Option<ErrorCorrectionLevel> {
    match s {
        "L" => Some(ErrorCorrectionLevel::L),
        "M" => Some(ErrorCorrectionLevel::M),
        "Q" => Some(ErrorCorrectionLevel::Q),
        "H" => Some(ErrorCorrectionLevel::H),
        _ => None,
    }
}

fn convert_rqrr_ec(level: u16) -> ErrorCorrectionLevel {
    match level {
        0 => ErrorCorrectionLevel::M,
        1 => ErrorCorrectionLevel::L,
        2 => ErrorCorrectionLevel::H,
        3 => ErrorCorrectionLevel::Q,
        _ => ErrorCorrectionLevel::M,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_qr() -> Vec<u8> {
        use image::Luma;

        let code = qrcode::QrCode::new(b"https://example.com").unwrap();
        let img = code.render::<Luma<u8>>().build();

        let mut buf = Vec::new();
        let dyn_img = DynamicImage::ImageLuma8(img);
        dyn_img
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        buf
    }

    #[test]
    fn decode_simple_qr() {
        let qr_bytes = create_test_qr();
        let result = multi_decode(&qr_bytes).unwrap();
        assert_eq!(result.content, "https://example.com");
    }

    #[test]
    fn decode_provides_metadata() {
        let qr_bytes = create_test_qr();
        let result = multi_decode(&qr_bytes).unwrap();
        assert!(result.metadata.is_some());
    }

    #[test]
    fn decode_invalid_image_returns_error() {
        let result = multi_decode(b"not an image at all");
        assert!(result.is_err());
    }

    #[test]
    fn parse_ec_level_all_variants() {
        assert_eq!(parse_ec_level("L"), Some(ErrorCorrectionLevel::L));
        assert_eq!(parse_ec_level("M"), Some(ErrorCorrectionLevel::M));
        assert_eq!(parse_ec_level("Q"), Some(ErrorCorrectionLevel::Q));
        assert_eq!(parse_ec_level("H"), Some(ErrorCorrectionLevel::H));
        assert_eq!(parse_ec_level("X"), None);
        assert_eq!(parse_ec_level(""), None);
    }

    #[test]
    fn convert_rqrr_ec_all_variants() {
        assert_eq!(convert_rqrr_ec(0), ErrorCorrectionLevel::M);
        assert_eq!(convert_rqrr_ec(1), ErrorCorrectionLevel::L);
        assert_eq!(convert_rqrr_ec(2), ErrorCorrectionLevel::H);
        assert_eq!(convert_rqrr_ec(3), ErrorCorrectionLevel::Q);
        assert_eq!(convert_rqrr_ec(99), ErrorCorrectionLevel::M); // unknown → M
    }

    #[test]
    fn decode_blank_image_returns_error() {
        let blank = DynamicImage::new_luma8(100, 100);
        let mut buf = Vec::new();
        blank
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        let result = multi_decode(&buf);
        assert!(result.is_err());
    }
}