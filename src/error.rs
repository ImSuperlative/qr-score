use thiserror::Error;

#[derive(Debug, Error)]
pub enum QrScoreError {
    #[error("Failed to load image: {0}")]
    ImageLoad(String),

    #[error("No QR code found in image")]
    DecodeFailed,

    #[error("Invalid SVG: {0}")]
    InvalidSvg(String),

    #[error("Failed to render SVG to PNG")]
    RenderFailed,

    #[error("Image too large: {width}x{height} exceeds maximum {max_dimension}x{max_dimension}")]
    DimensionsTooLarge {
        width: u32,
        height: u32,
        max_dimension: u32,
    },

    #[error("Dimension overflow: {width} x {height} overflows")]
    DimensionOverflow { width: u32, height: u32 },
}

pub type Result<T> = std::result::Result<T, QrScoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_image_load() {
        let err = QrScoreError::ImageLoad("invalid format".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Failed to load image"));
        assert!(msg.contains("invalid format"));
    }

    #[test]
    fn error_display_decode_failed() {
        let err = QrScoreError::DecodeFailed;
        assert!(err.to_string().contains("No QR code"));
    }

    #[test]
    fn error_display_invalid_svg() {
        let err = QrScoreError::InvalidSvg("bad xml".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Invalid SVG"));
        assert!(msg.contains("bad xml"));
    }

    #[test]
    fn error_display_render_failed() {
        assert!(QrScoreError::RenderFailed.to_string().contains("render SVG"));
    }

    #[test]
    fn error_display_dimensions_too_large() {
        let err = QrScoreError::DimensionsTooLarge { width: 20000, height: 20000, max_dimension: 10000 };
        let msg = err.to_string();
        assert!(msg.contains("20000"));
        assert!(msg.contains("10000"));
    }

    #[test]
    fn error_display_dimension_overflow() {
        let err = QrScoreError::DimensionOverflow { width: u32::MAX, height: u32::MAX };
        assert!(err.to_string().contains("overflow"));
    }
}