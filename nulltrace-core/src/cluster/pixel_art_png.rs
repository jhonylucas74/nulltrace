//! PNG validation and canonical encoding for Hackerboard avatars / faction emblems (BYTEA in Postgres).
//! Legacy NTPX blobs are converted to canonical PNG on ingest.

use super::pixel_art_binary::{validate_pixel_art_binary, PixelArtBinaryError};
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder, RgbaImage};

/// Max encoded PNG size accepted from VM or clients (abuse prevention).
pub const HACKERBOARD_PNG_MAX_BYTES: usize = 256 * 1024;

fn allowed_dim(n: u32) -> bool {
    n == 16 || n == 32
}

fn is_png_signature(data: &[u8]) -> bool {
    data.len() >= 8
        && data[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PixelArtPngError {
    TooLarge,
    InvalidPng,
    BadDimensions,
}

impl std::fmt::Display for PixelArtPngError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PixelArtPngError::TooLarge => write!(f, "PNG file too large"),
            PixelArtPngError::InvalidPng => write!(f, "Invalid PNG image"),
            PixelArtPngError::BadDimensions => {
                write!(f, "Image dimensions must be 16 or 32 for width and height")
            }
        }
    }
}

impl std::error::Error for PixelArtPngError {}

/// Decode PNG, enforce 16/32 dimensions, re-encode as canonical 8-bit RGBA PNG.
pub fn validate_and_canonical_png(data: &[u8]) -> Result<Vec<u8>, PixelArtPngError> {
    if data.len() > HACKERBOARD_PNG_MAX_BYTES {
        return Err(PixelArtPngError::TooLarge);
    }
    let img = image::load_from_memory(data)
        .map_err(|_| PixelArtPngError::InvalidPng)?
        .to_rgba8();
    let (w, h) = img.dimensions();
    if !allowed_dim(w) || !allowed_dim(h) {
        return Err(PixelArtPngError::BadDimensions);
    }
    let raw = img.into_raw();
    let mut out = Vec::new();
    let encoder = PngEncoder::new(&mut out);
    encoder
        .write_image(&raw, w, h, ExtendedColorType::Rgba8)
        .map_err(|_| PixelArtPngError::InvalidPng)?;
    Ok(out)
}

/// Convert validated NTPX blob to canonical PNG (same visual).
pub fn ntpx_to_canonical_png(data: &[u8]) -> Result<Vec<u8>, PixelArtBinaryError> {
    validate_pixel_art_binary(data)?;
    let w = u16::from_le_bytes([data[4], data[5]]);
    let h = u16::from_le_bytes([data[6], data[7]]);
    let wu = w as u32;
    let hu = h as u32;
    let mut img = RgbaImage::new(wu, hu);
    let mut o = 8usize;
    for y in 0..hu {
        for x in 0..wu {
            let r = data[o];
            let g = data[o + 1];
            let b = data[o + 2];
            o += 3;
            img.put_pixel(x, y, image::Rgba([r, g, b, 255]));
        }
    }
    let raw = img.into_raw();
    let mut out = Vec::new();
    let encoder = PngEncoder::new(&mut out);
    encoder
        .write_image(&raw, wu, hu, ExtendedColorType::Rgba8)
        .map_err(|_| PixelArtBinaryError::TooLarge)?;
    Ok(out)
}

/// Accept PNG (canonicalized) or legacy NTPX (converted to PNG). Stored format is always PNG bytes.
pub fn validated_hackerboard_image_bytes(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.is_empty() {
        return Err("File too short".to_string());
    }
    if is_png_signature(data) {
        return validate_and_canonical_png(data).map_err(|e| e.to_string());
    }
    ntpx_to_canonical_png(data).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::pixel_art_binary::{PIXEL_ART_MAGIC, PIXEL_ART_MAX_BYTES};

    fn sample_ntpx_16() -> Vec<u8> {
        let mut v = Vec::with_capacity(8 + 16 * 16 * 3);
        v.extend_from_slice(PIXEL_ART_MAGIC);
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.resize(8 + 16 * 16 * 3, 0);
        v[8] = 0xAB;
        v[9] = 0xCD;
        v[10] = 0xEF;
        v
    }

    #[test]
    fn round_trip_ntpx_to_png_and_validate() {
        let ntpx = sample_ntpx_16();
        let png = ntpx_to_canonical_png(&ntpx).unwrap();
        assert!(png.len() < PIXEL_ART_MAX_BYTES);
        let again = validate_and_canonical_png(&png).unwrap();
        assert_eq!(again, png);
    }

    #[test]
    fn hackerboard_accepts_png_or_ntpx() {
        let ntpx = sample_ntpx_16();
        let png = ntpx_to_canonical_png(&ntpx).unwrap();
        let a = validated_hackerboard_image_bytes(&png).unwrap();
        let b = validated_hackerboard_image_bytes(&ntpx).unwrap();
        assert_eq!(a, b);
    }
}
