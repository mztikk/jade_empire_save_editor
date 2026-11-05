//! The screenshot that the game stores next to a savegame.
//!
//! `SaveImage.tga` is a plain TGA file (512×512 in every known savegame). It is
//! decoded here so the desktop application can show a preview without pulling
//! in an image decoding dependency.

use std::path::Path;

use crate::error::{Error, Result};

/// Decoded savegame screenshot in RGBA8888 order, top row first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// RGBA pixels, `width * height * 4` bytes.
    pub rgba: Vec<u8>,
}

/// Name of the screenshot next to `save.sac`.
pub const SAVE_IMAGE_NAME: &str = "SaveImage.tga";
/// Largest accepted screenshot dimension per side.
const MAX_DIMENSION: u32 = 4096;
/// Largest accepted total pixel count (16 Mi pixels).
const MAX_PIXELS: usize = 16 * 1024 * 1024;

impl SaveImage {
    /// Loads and decodes `SaveImage.tga` from the folder of a savegame.
    pub fn load(save_path: impl AsRef<Path>) -> Result<Self> {
        let path = save_path.as_ref();
        let folder = if path.is_dir() {
            path.to_path_buf()
        } else {
            path.parent()
                .map_or_else(|| path.to_path_buf(), Path::to_path_buf)
        };
        let image_path = folder.join(SAVE_IMAGE_NAME);
        let bytes = std::fs::read(&image_path).map_err(|source| Error::Io {
            path: image_path.clone(),
            source,
        })?;
        Self::parse(&bytes)
    }

    /// Decodes a TGA image from memory.
    ///
    /// Supports the uncompressed and RLE true colour and greyscale variants
    /// that the game could produce; anything else is rejected with an error.
    #[allow(clippy::too_many_lines)] // one linear decoder for the whole TGA format
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 18 {
            return Err(Error::Corrupt("the screenshot is too short".to_owned()));
        }
        let id_length = usize::from(bytes[0]);
        let colour_map = bytes[1];
        let image_type = bytes[2];
        let width = u32::from(u16::from_le_bytes([bytes[12], bytes[13]]));
        let height = u32::from(u16::from_le_bytes([bytes[14], bytes[15]]));
        let depth = bytes[16];
        let descriptor = bytes[17];
        if width == 0 || height == 0 {
            return Err(Error::Corrupt("the screenshot has no pixels".to_owned()));
        }
        if colour_map != 0 {
            return Err(Error::Corrupt(
                "colour mapped screenshots are not supported".to_owned(),
            ));
        }
        let compressed = match image_type {
            2 | 3 => false,
            10 | 11 => true,
            other => {
                return Err(Error::Corrupt(format!(
                    "unsupported screenshot type {other}"
                )));
            }
        };
        if image_type == 3 && depth != 8 {
            return Err(Error::Corrupt(format!(
                "greyscale screenshots must use 8 bit pixels, found {depth}"
            )));
        }
        let bytes_per_pixel = match depth {
            8 | 24 | 32 => usize::from(depth) / 8,
            other => {
                return Err(Error::Corrupt(format!(
                    "unsupported screenshot depth {other}"
                )));
            }
        };
        if width > MAX_DIMENSION || height > MAX_DIMENSION {
            return Err(Error::Corrupt(format!(
                "screenshot dimensions {width}x{height} exceed the {MAX_DIMENSION} pixel limit"
            )));
        }
        let pixel_count = (width as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| Error::Corrupt("screenshot dimensions overflow".to_owned()))?;
        if pixel_count > MAX_PIXELS {
            return Err(Error::Corrupt(format!(
                "screenshot has {pixel_count} pixels, above the {MAX_PIXELS} pixel limit"
            )));
        }
        let needed = pixel_count
            .checked_mul(bytes_per_pixel)
            .ok_or_else(|| Error::Corrupt("screenshot data size overflows".to_owned()))?;

        let mut position = 18 + id_length;
        let mut raw = Vec::with_capacity(needed);
        if compressed {
            while raw.len() < needed {
                let packet = *bytes
                    .get(position)
                    .ok_or_else(|| Error::Corrupt("truncated RLE packet".to_owned()))?;
                position += 1;
                let count = usize::from(packet & 0x7f) + 1;
                let remaining_pixels = (needed - raw.len()) / bytes_per_pixel;
                if count > remaining_pixels {
                    return Err(Error::Corrupt(
                        "RLE packet writes past the end of the screenshot".to_owned(),
                    ));
                }
                if packet & 0x80 != 0 {
                    let end = position
                        .checked_add(bytes_per_pixel)
                        .ok_or_else(|| Error::Corrupt("RLE pixel overflows".to_owned()))?;
                    let pixel = bytes
                        .get(position..end)
                        .ok_or_else(|| Error::Corrupt("truncated RLE pixel".to_owned()))?;
                    position = end;
                    for _ in 0..count {
                        raw.extend_from_slice(pixel);
                    }
                } else {
                    let count_bytes = count
                        .checked_mul(bytes_per_pixel)
                        .ok_or_else(|| Error::Corrupt("RLE run overflows".to_owned()))?;
                    let end = position
                        .checked_add(count_bytes)
                        .ok_or_else(|| Error::Corrupt("RLE run overflows".to_owned()))?;
                    let pixels = bytes
                        .get(position..end)
                        .ok_or_else(|| Error::Corrupt("truncated RLE run".to_owned()))?;
                    position = end;
                    raw.extend_from_slice(pixels);
                }
            }
        } else {
            let end = position
                .checked_add(needed)
                .ok_or_else(|| Error::Corrupt("screenshot data overflows".to_owned()))?;
            raw = bytes
                .get(position..end)
                .ok_or_else(|| Error::Corrupt("the screenshot data is truncated".to_owned()))?
                .to_vec();
        }

        let rgba_len = pixel_count
            .checked_mul(4)
            .ok_or_else(|| Error::Corrupt("screenshot output size overflows".to_owned()))?;
        let mut rgba = Vec::with_capacity(rgba_len);
        for pixel in raw.chunks_exact(bytes_per_pixel) {
            match bytes_per_pixel {
                3 => {
                    rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], 255]);
                }
                4 => {
                    rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
                }
                _ => {
                    rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], 255]);
                }
            }
        }

        // TGA stores rows bottom up unless bit 5 of the descriptor is set.
        if descriptor & 0x20 == 0 {
            let row = (width as usize) * 4;
            let mut flipped = vec![0_u8; rgba.len()];
            for y in 0..height as usize {
                let source = (height as usize - 1 - y) * row;
                let target = y * row;
                flipped[target..target + row].copy_from_slice(&rgba[source..source + row]);
            }
            rgba = flipped;
        }

        Ok(Self {
            width,
            height,
            rgba,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tga(width: u16, height: u16, pixels: &[u8], depth: u8, top_down: bool) -> Vec<u8> {
        let mut file = vec![0_u8; 18];
        file[2] = 2;
        file[12..14].copy_from_slice(&width.to_le_bytes());
        file[14..16].copy_from_slice(&height.to_le_bytes());
        file[16] = depth;
        file[17] = if top_down { 0x20 } else { 0 };
        file.extend_from_slice(pixels);
        file
    }

    #[test]
    fn decodes_bgra_bottom_up() {
        // 2x2, bottom row first. Each pixel is BGR: blue, green / red, white.
        let pixels = [
            255, 0, 0, 0, 255, 0, // blue, green (bottom row)
            0, 0, 255, 255, 255, 255, // red, white (top row)
        ];
        let file = tga(2, 2, &pixels, 24, false);
        let image = SaveImage::parse(&file).expect("decode");
        assert_eq!((image.width, image.height), (2, 2));
        // Top-left must be red.
        assert_eq!(&image.rgba[..4], &[255, 0, 0, 255]);
        assert_eq!(&image.rgba[4..8], &[255, 255, 255, 255]);
        assert_eq!(&image.rgba[8..12], &[0, 0, 255, 255]);
    }

    #[test]
    fn rejects_hostile_dimensions_and_rle_overruns() {
        let mut huge = vec![0_u8; 18];
        huge[2] = 2;
        huge[12..14].copy_from_slice(&u16::MAX.to_le_bytes());
        huge[14..16].copy_from_slice(&u16::MAX.to_le_bytes());
        huge[16] = 32;
        assert!(SaveImage::parse(&huge).is_err());

        let mut overrun = vec![0_u8; 18];
        overrun[2] = 10;
        overrun[12..14].copy_from_slice(&1_u16.to_le_bytes());
        overrun[14..16].copy_from_slice(&1_u16.to_le_bytes());
        overrun[16] = 32;
        overrun[17] = 0x28;
        overrun.push(0x81);
        overrun.extend_from_slice(&[1, 2, 3, 4]);
        assert!(SaveImage::parse(&overrun).is_err());
    }

    #[test]
    fn rejects_greyscale_with_a_colour_depth() {
        let mut file = vec![0_u8; 18];
        file[2] = 3;
        file[12..14].copy_from_slice(&1_u16.to_le_bytes());
        file[14..16].copy_from_slice(&1_u16.to_le_bytes());
        file[16] = 24;
        assert!(SaveImage::parse(&file).is_err());
    }

    #[test]
    fn decodes_rle() {
        // Two pixels: one run-length packet with two red pixels.
        let mut file = vec![0_u8; 18];
        file[2] = 10;
        file[12..14].copy_from_slice(&2_u16.to_le_bytes());
        file[14..16].copy_from_slice(&1_u16.to_le_bytes());
        file[16] = 32;
        file[17] = 0x28; // top down + alpha bits
        file.push(0x81); // run of 2
        file.extend_from_slice(&[0, 0, 255, 255]);
        let image = SaveImage::parse(&file).expect("decode");
        assert_eq!(&image.rgba[..8], &[255, 0, 0, 255, 255, 0, 0, 255]);
    }
}
