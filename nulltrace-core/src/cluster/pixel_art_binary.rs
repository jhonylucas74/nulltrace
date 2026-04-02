//! Canonical binary pixel-art format (VM files + BYTEA in Postgres).
//! Layout: magic `NTPX`, u16 LE width, u16 LE height, then width*height*3 RGB bytes (row-major).

// Used by the game client when writing VM files (see HACKERBOARD.md).
#[allow(dead_code)]
pub const PIXEL_ART_MIME: &str = "application/x-nulltrace-pixel-art";

pub const PIXEL_ART_MAGIC: &[u8; 4] = b"NTPX";
/// Cap for abuse prevention (32×32×3 + 8 ≈ 3 KiB; this leaves headroom).
pub const PIXEL_ART_MAX_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PixelArtBinaryError {
    TooLarge,
    TooShort,
    BadMagic,
    BadDimensions,
    LengthMismatch,
}

impl std::fmt::Display for PixelArtBinaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PixelArtBinaryError::TooLarge => write!(f, "Pixel art file too large"),
            PixelArtBinaryError::TooShort => write!(f, "Pixel art file too short"),
            PixelArtBinaryError::BadMagic => write!(f, "Invalid pixel art format (magic)"),
            PixelArtBinaryError::BadDimensions => {
                write!(f, "Pixel art dimensions must be 16 or 32 for width and height")
            }
            PixelArtBinaryError::LengthMismatch => write!(f, "Pixel art size does not match header"),
        }
    }
}

impl std::error::Error for PixelArtBinaryError {}

fn allowed_dim(n: u16) -> bool {
    n == 16 || n == 32
}

/// Validates NTPX blob; returns `Ok(())` if `data` is a valid canonical file.
pub fn validate_pixel_art_binary(data: &[u8]) -> Result<(), PixelArtBinaryError> {
    if data.len() > PIXEL_ART_MAX_BYTES {
        return Err(PixelArtBinaryError::TooLarge);
    }
    if data.len() < 8 {
        return Err(PixelArtBinaryError::TooShort);
    }
    if data[..4] != PIXEL_ART_MAGIC[..] {
        return Err(PixelArtBinaryError::BadMagic);
    }
    let w = u16::from_le_bytes([data[4], data[5]]);
    let h = u16::from_le_bytes([data[6], data[7]]);
    if !allowed_dim(w) || !allowed_dim(h) {
        return Err(PixelArtBinaryError::BadDimensions);
    }
    let expected = 8usize
        .saturating_add((w as usize).saturating_mul(h as usize).saturating_mul(3));
    if data.len() != expected {
        return Err(PixelArtBinaryError::LengthMismatch);
    }
    Ok(())
}

/// Returns an owned copy after validation (for storing in BYTEA).
pub fn validated_pixel_art_bytes(data: &[u8]) -> Result<Vec<u8>, PixelArtBinaryError> {
    validate_pixel_art_binary(data)?;
    Ok(data.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_16() -> Vec<u8> {
        let mut v = Vec::with_capacity(8 + 16 * 16 * 3);
        v.extend_from_slice(PIXEL_ART_MAGIC);
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.resize(8 + 16 * 16 * 3, 0);
        v
    }

    #[test]
    fn valid_16() {
        let b = sample_16();
        assert!(validate_pixel_art_binary(&b).is_ok());
    }

    #[test]
    fn bad_magic() {
        let mut b = sample_16();
        b[0] = b'X';
        assert_eq!(validate_pixel_art_binary(&b), Err(PixelArtBinaryError::BadMagic));
    }

    #[test]
    fn bad_dim() {
        let mut v = Vec::with_capacity(8 + 8 * 8 * 3);
        v.extend_from_slice(PIXEL_ART_MAGIC);
        v.extend_from_slice(&8u16.to_le_bytes());
        v.extend_from_slice(&8u16.to_le_bytes());
        v.resize(8 + 8 * 8 * 3, 0);
        assert_eq!(
            validate_pixel_art_binary(&v),
            Err(PixelArtBinaryError::BadDimensions)
        );
    }

    #[test]
    fn length_mismatch() {
        let mut b = sample_16();
        b.pop();
        assert_eq!(
            validate_pixel_art_binary(&b),
            Err(PixelArtBinaryError::LengthMismatch)
        );
    }
}
