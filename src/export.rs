//! Image export formats.
//!
//! Writes wallpaper images as PNG, PPM, BMP or SVG. PNG, PPM and BMP go
//! through the [`image`] crate, so PNG output is deflate-compressed and every
//! file can be read back by any image viewer. SVG is a simple `<rect>` grid
//! written by hand.

use std::path::Path;
use std::time::Instant;

// ── ExportFormat ──────────────────────────────────────────────────────────────

/// Supported export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// PNG (lossless, deflate-compressed).
    Png,
    /// Portable Pixmap (P6 binary).
    Ppm,
    /// 24-bit BMP.
    Bmp,
    /// SVG: one `<rect>` per sampled pixel block.
    Svg,
}

// ── ExportError ───────────────────────────────────────────────────────────────

/// Errors from export operations.
#[derive(Debug)]
pub enum ExportError {
    /// Creating or writing the file failed.
    Io(std::io::Error),
    /// Width or height is zero, or the pixel buffer does not hold
    /// `width * height` pixels.
    InvalidDimensions,
    /// The encoder rejected the image.
    UnsupportedFormat(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::Io(e) => write!(f, "IO error: {e}"),
            ExportError::InvalidDimensions => write!(
                f,
                "invalid image dimensions (zero size, or pixel count does not match width x height)"
            ),
            ExportError::UnsupportedFormat(s) => write!(f, "unsupported format: {s}"),
        }
    }
}

impl std::error::Error for ExportError {}

impl From<std::io::Error> for ExportError {
    fn from(e: std::io::Error) -> Self {
        ExportError::Io(e)
    }
}

impl From<image::ImageError> for ExportError {
    fn from(e: image::ImageError) -> Self {
        match e {
            image::ImageError::IoError(io) => ExportError::Io(io),
            other => ExportError::UnsupportedFormat(other.to_string()),
        }
    }
}

// ── ExportStats ───────────────────────────────────────────────────────────────

/// Statistics from a completed export.
#[derive(Debug, Clone)]
pub struct ExportStats {
    /// Size of the written file in bytes (read back from the file system).
    pub bytes_written: u64,
    /// Format that was written.
    pub format: ExportFormat,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Wall-clock time spent encoding and writing.
    pub elapsed_ms: u64,
}

// ── ImageExporter ─────────────────────────────────────────────────────────────

/// Exports wallpaper images in multiple formats.
pub struct ImageExporter;

impl ImageExporter {
    /// Export `pixels` (row-major, RGB triplets) as `format` to `path`.
    ///
    /// Returns [`ExportError::InvalidDimensions`] if either side is zero or
    /// `pixels.len() != width * height`.
    pub fn export(
        pixels: &[[u8; 3]],
        width: u32,
        height: u32,
        format: ExportFormat,
        path: &Path,
    ) -> Result<ExportStats, ExportError> {
        if width == 0 || height == 0 || pixels.len() as u64 != width as u64 * height as u64 {
            return Err(ExportError::InvalidDimensions);
        }
        let start = Instant::now();
        match format {
            ExportFormat::Svg => write_svg(pixels, width, height, path)?,
            ExportFormat::Png | ExportFormat::Ppm | ExportFormat::Bmp => {
                let flat: Vec<u8> = pixels.iter().flatten().copied().collect();
                let img = image::RgbImage::from_raw(width, height, flat)
                    .ok_or(ExportError::InvalidDimensions)?;
                match format {
                    ExportFormat::Ppm => {
                        // The image crate's default PNM subtype is PAM (P7);
                        // ask for a binary pixmap (P6) explicitly.
                        use image::codecs::pnm::{PnmEncoder, PnmSubtype, SampleEncoding};
                        let file = std::io::BufWriter::new(std::fs::File::create(path)?);
                        let mut out = file;
                        PnmEncoder::new(&mut out)
                            .with_subtype(PnmSubtype::Pixmap(SampleEncoding::Binary))
                            .encode(
                                img.as_raw().as_slice(),
                                width,
                                height,
                                image::ExtendedColorType::Rgb8,
                            )?;
                        std::io::Write::flush(&mut out)?;
                    }
                    ExportFormat::Png => img.save_with_format(path, image::ImageFormat::Png)?,
                    _ => img.save_with_format(path, image::ImageFormat::Bmp)?,
                }
            }
        }
        let bytes_written = std::fs::metadata(path)?.len();
        let elapsed_ms = start.elapsed().as_millis() as u64;
        Ok(ExportStats {
            bytes_written,
            format,
            width,
            height,
            elapsed_ms,
        })
    }
}

/// Save tightly packed RGBA8 pixels as a compressed PNG.
///
/// Missing parent folders are created. Fails with
/// [`ExportError::InvalidDimensions`] if `rgba.len() != width * height * 4`.
/// Returns the size of the written file in bytes.
pub fn save_rgba_png(
    path: &Path,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<u64, ExportError> {
    if width == 0 || height == 0 || rgba.len() as u64 != width as u64 * height as u64 * 4 {
        return Err(ExportError::InvalidDimensions);
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    image::save_buffer_with_format(
        path,
        rgba,
        width,
        height,
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )?;
    Ok(std::fs::metadata(path)?.len())
}

/// Write `rgba` (RGBA8) to `path` as a PNG.
///
/// Kept for backwards compatibility; prefer [`save_rgba_png`].
pub fn encode_png_rgba(
    path: &Path,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<(), std::io::Error> {
    save_rgba_png(path, width, height, rgba)
        .map(|_| ())
        .map_err(|e| match e {
            ExportError::Io(io) => io,
            other => std::io::Error::new(std::io::ErrorKind::InvalidInput, other.to_string()),
        })
}

// ── SVG writer ────────────────────────────────────────────────────────────────

/// Write an SVG with one `<rect>` per sampled pixel block.
///
/// For large images (>64x64) each pixel is drawn 8 units wide and at most
/// 256x256 cells are emitted, to keep the file a manageable size.
fn write_svg(pixels: &[[u8; 3]], width: u32, height: u32, path: &Path) -> Result<(), ExportError> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;

    let tile = if width > 64 || height > 64 {
        8u32
    } else {
        1u32
    };
    let svg_w = width * tile;
    let svg_h = height * tile;

    let mut buf = String::new();
    buf.push_str(&format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
"#,
        svg_w, svg_h, svg_w, svg_h
    ));

    let max_cells = 256u32;
    let step_x = (width / max_cells).max(1);
    let step_y = (height / max_cells).max(1);

    let mut y = 0u32;
    while y < height {
        let mut x = 0u32;
        while x < width {
            let idx = (y * width + x) as usize;
            let [r, g, b] = pixels[idx];
            let rx = x * tile;
            let ry = y * tile;
            let rw = (step_x * tile).min(svg_w - rx);
            let rh = (step_y * tile).min(svg_h - ry);
            buf.push_str(&format!(
                "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#{:02X}{:02X}{:02X}\"/>\n",
                rx, ry, rw, rh, r, g, b
            ));
            x += step_x;
        }
        y += step_y;
    }

    buf.push_str("</svg>\n");
    f.write_all(buf.as_bytes())?;
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn sample_pixels(w: u32, h: u32) -> Vec<[u8; 3]> {
        (0..w * h)
            .map(|i| [(i % 256) as u8, ((i / 256) % 256) as u8, 128])
            .collect()
    }

    // 1. PPM export creates a file
    #[test]
    fn test_ppm_creates_file() {
        let tmp = NamedTempFile::new().unwrap();
        let pixels = sample_pixels(4, 4);
        let stats = ImageExporter::export(&pixels, 4, 4, ExportFormat::Ppm, tmp.path()).unwrap();
        assert!(tmp.path().exists());
        assert!(stats.bytes_written > 0);
    }

    // 2. PPM header is a valid binary PNM header
    #[test]
    fn test_ppm_header() {
        let tmp = NamedTempFile::new().unwrap();
        let pixels = sample_pixels(2, 2);
        ImageExporter::export(&pixels, 2, 2, ExportFormat::Ppm, tmp.path()).unwrap();
        let content = std::fs::read(tmp.path()).unwrap();
        assert!(content.starts_with(b"P6"), "{:?}", &content[..8]);
    }

    // 3. PPM round-trips: decoding gives back the same pixels
    #[test]
    fn test_ppm_roundtrip() {
        let tmp = NamedTempFile::new().unwrap();
        let (w, h) = (3u32, 3u32);
        let pixels = sample_pixels(w, h);
        ImageExporter::export(&pixels, w, h, ExportFormat::Ppm, tmp.path()).unwrap();
        let img = image::load_from_memory_with_format(
            &std::fs::read(tmp.path()).unwrap(),
            image::ImageFormat::Pnm,
        )
        .unwrap()
        .to_rgb8();
        assert_eq!(img.dimensions(), (w, h));
        let back: Vec<[u8; 3]> = img.pixels().map(|p| p.0).collect();
        assert_eq!(back, pixels);
    }

    // 4. BMP export creates a file
    #[test]
    fn test_bmp_creates_file() {
        let tmp = NamedTempFile::new().unwrap();
        let pixels = sample_pixels(4, 4);
        ImageExporter::export(&pixels, 4, 4, ExportFormat::Bmp, tmp.path()).unwrap();
        assert!(tmp.path().exists());
    }

    // 5. BMP starts with 'BM' magic
    #[test]
    fn test_bmp_magic() {
        let tmp = NamedTempFile::new().unwrap();
        let pixels = sample_pixels(4, 4);
        ImageExporter::export(&pixels, 4, 4, ExportFormat::Bmp, tmp.path()).unwrap();
        let content = std::fs::read(tmp.path()).unwrap();
        assert_eq!(&content[..2], b"BM");
    }

    // 6. BMP file size field matches actual file
    #[test]
    fn test_bmp_file_size_field() {
        let tmp = NamedTempFile::new().unwrap();
        let pixels = sample_pixels(4, 4);
        ImageExporter::export(&pixels, 4, 4, ExportFormat::Bmp, tmp.path()).unwrap();
        let content = std::fs::read(tmp.path()).unwrap();
        let size_field = u32::from_le_bytes([content[2], content[3], content[4], content[5]]);
        assert_eq!(size_field as usize, content.len());
    }

    // 7. BMP bits per pixel is 24
    #[test]
    fn test_bmp_bits_per_pixel() {
        let tmp = NamedTempFile::new().unwrap();
        let pixels = sample_pixels(4, 4);
        ImageExporter::export(&pixels, 4, 4, ExportFormat::Bmp, tmp.path()).unwrap();
        let content = std::fs::read(tmp.path()).unwrap();
        let bpp = u16::from_le_bytes([content[28], content[29]]);
        assert_eq!(bpp, 24);
    }

    // 8. PNG export creates a file
    #[test]
    fn test_png_creates_file() {
        let tmp = NamedTempFile::new().unwrap();
        let pixels = sample_pixels(4, 4);
        ImageExporter::export(&pixels, 4, 4, ExportFormat::Png, tmp.path()).unwrap();
        assert!(tmp.path().exists());
    }

    // 9. PNG starts with PNG signature
    #[test]
    fn test_png_signature() {
        let tmp = NamedTempFile::new().unwrap();
        let pixels = sample_pixels(4, 4);
        ImageExporter::export(&pixels, 4, 4, ExportFormat::Png, tmp.path()).unwrap();
        let content = std::fs::read(tmp.path()).unwrap();
        assert_eq!(&content[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }

    // 10. SVG export creates a file
    #[test]
    fn test_svg_creates_file() {
        let tmp = NamedTempFile::new().unwrap();
        let pixels = sample_pixels(4, 4);
        ImageExporter::export(&pixels, 4, 4, ExportFormat::Svg, tmp.path()).unwrap();
        assert!(tmp.path().exists());
    }

    // 11. SVG contains <svg> tag
    #[test]
    fn test_svg_tag() {
        let tmp = NamedTempFile::new().unwrap();
        let pixels = sample_pixels(4, 4);
        ImageExporter::export(&pixels, 4, 4, ExportFormat::Svg, tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(content.contains("<svg"));
        assert!(content.contains("</svg>"));
    }

    // 12. SVG contains rect elements
    #[test]
    fn test_svg_has_rects() {
        let tmp = NamedTempFile::new().unwrap();
        let pixels = sample_pixels(4, 4);
        ImageExporter::export(&pixels, 4, 4, ExportFormat::Svg, tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(content.contains("<rect"));
    }

    // 13. Invalid dimensions returns error
    #[test]
    fn test_invalid_dimensions() {
        let tmp = NamedTempFile::new().unwrap();
        let err = ImageExporter::export(&[], 0, 10, ExportFormat::Ppm, tmp.path()).unwrap_err();
        assert!(matches!(err, ExportError::InvalidDimensions));
    }

    // 14. ExportStats has correct dimensions
    #[test]
    fn test_stats_dimensions() {
        let tmp = NamedTempFile::new().unwrap();
        let pixels = sample_pixels(8, 6);
        let stats = ImageExporter::export(&pixels, 8, 6, ExportFormat::Ppm, tmp.path()).unwrap();
        assert_eq!(stats.width, 8);
        assert_eq!(stats.height, 6);
    }

    // 15. ExportStats has correct format
    #[test]
    fn test_stats_format() {
        let tmp = NamedTempFile::new().unwrap();
        let pixels = sample_pixels(4, 4);
        let stats = ImageExporter::export(&pixels, 4, 4, ExportFormat::Bmp, tmp.path()).unwrap();
        assert_eq!(stats.format, ExportFormat::Bmp);
    }

    // 16. ExportError display is non-empty
    #[test]
    fn test_error_display() {
        let e = ExportError::InvalidDimensions;
        assert!(!e.to_string().is_empty());
    }

    // 17. BMP width field matches
    #[test]
    fn test_bmp_width_field() {
        let tmp = NamedTempFile::new().unwrap();
        let pixels = sample_pixels(5, 3);
        ImageExporter::export(&pixels, 5, 3, ExportFormat::Bmp, tmp.path()).unwrap();
        let content = std::fs::read(tmp.path()).unwrap();
        let w = i32::from_le_bytes([content[18], content[19], content[20], content[21]]);
        assert_eq!(w, 5);
    }

    // 18. BMP height field matches
    #[test]
    fn test_bmp_height_field() {
        let tmp = NamedTempFile::new().unwrap();
        let pixels = sample_pixels(5, 3);
        ImageExporter::export(&pixels, 5, 3, ExportFormat::Bmp, tmp.path()).unwrap();
        let content = std::fs::read(tmp.path()).unwrap();
        let h = i32::from_le_bytes([content[22], content[23], content[24], content[25]]);
        assert_eq!(h, 3);
    }

    // 19. PPM export with 1x1 pixel
    #[test]
    fn test_ppm_one_pixel() {
        let tmp = NamedTempFile::new().unwrap();
        let pixels = vec![[255u8, 128u8, 0u8]];
        let stats = ImageExporter::export(&pixels, 1, 1, ExportFormat::Ppm, tmp.path()).unwrap();
        assert!(stats.bytes_written > 0);
    }

    // 20. BMP export with 1x1 pixel
    #[test]
    fn test_bmp_one_pixel() {
        let tmp = NamedTempFile::new().unwrap();
        let pixels = vec![[255u8, 128u8, 0u8]];
        ImageExporter::export(&pixels, 1, 1, ExportFormat::Bmp, tmp.path()).unwrap();
        let content = std::fs::read(tmp.path()).unwrap();
        assert_eq!(&content[..2], b"BM");
    }

    // 21. bytes_written is the real file size for every format (the old PNG
    //     path returned an estimate).
    #[test]
    fn test_bytes_written_matches_file_size() {
        for fmt in [
            ExportFormat::Png,
            ExportFormat::Ppm,
            ExportFormat::Bmp,
            ExportFormat::Svg,
        ] {
            let tmp = NamedTempFile::new().unwrap();
            let pixels = sample_pixels(37, 11);
            let stats = ImageExporter::export(&pixels, 37, 11, fmt, tmp.path()).unwrap();
            let on_disk = std::fs::metadata(tmp.path()).unwrap().len();
            assert_eq!(stats.bytes_written, on_disk, "{fmt:?}");
        }
    }

    // 22. PNG is actually compressed (the old encoder used stored deflate
    //     blocks, so a 256x256 gradient came out larger than the raw pixels).
    #[test]
    fn test_png_is_compressed_and_decodes() {
        let tmp = NamedTempFile::new().unwrap();
        let (w, h) = (256u32, 256u32);
        let pixels: Vec<[u8; 3]> = (0..w * h)
            .map(|i| [(i % w) as u8, (i / w) as u8, 64])
            .collect();
        let stats = ImageExporter::export(&pixels, w, h, ExportFormat::Png, tmp.path()).unwrap();
        assert!(
            stats.bytes_written < (w * h * 3) as u64 / 4,
            "PNG of a smooth gradient should be well under raw size, got {} bytes",
            stats.bytes_written
        );
        let img = image::load_from_memory_with_format(
            &std::fs::read(tmp.path()).unwrap(),
            image::ImageFormat::Png,
        )
        .unwrap()
        .to_rgb8();
        let back: Vec<[u8; 3]> = img.pixels().map(|p| p.0).collect();
        assert_eq!(back, pixels);
    }

    // 23. A pixel buffer that is too short is an error, not a panic (the old
    //     BMP writer indexed out of bounds).
    #[test]
    fn test_short_buffer_is_error() {
        for fmt in [
            ExportFormat::Png,
            ExportFormat::Ppm,
            ExportFormat::Bmp,
            ExportFormat::Svg,
        ] {
            let tmp = NamedTempFile::new().unwrap();
            let pixels = sample_pixels(4, 3);
            let err = ImageExporter::export(&pixels, 4, 4, fmt, tmp.path()).unwrap_err();
            assert!(matches!(err, ExportError::InvalidDimensions), "{fmt:?}");
        }
    }

    // 24. save_rgba_png creates parent folders and rejects wrong sizes.
    #[test]
    fn test_save_rgba_png() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a").join("b").join("x.png");
        let rgba = vec![200u8; 8 * 8 * 4];
        let n = save_rgba_png(&path, 8, 8, &rgba).unwrap();
        assert_eq!(n, std::fs::metadata(&path).unwrap().len());
        assert_eq!(image::open(&path).unwrap().width(), 8);
        assert!(save_rgba_png(&path, 8, 9, &rgba).is_err());
    }
}
