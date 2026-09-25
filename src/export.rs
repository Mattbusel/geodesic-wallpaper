//! Image Export Formats
//!
//! Exports wallpaper images in PNG, PPM, BMP, and SVG formats.
//! Uses the stdlib-only PNG encoder from `animation.rs` for PNG output.

use std::path::Path;
use std::time::Instant;

// ── ExportFormat ──────────────────────────────────────────────────────────────

/// Supported export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// PNG (lossless, uses the stdlib-only encoder from animation.rs).
    Png,
    /// Portable Pixmap (P6 binary).
    Ppm,
    /// 24-bit BMP with BITMAPFILEHEADER + BITMAPINFOHEADER.
    Bmp,
    /// SVG — emits `<rect>` elements for each tile cell.
    Svg,
}

// ── ExportError ───────────────────────────────────────────────────────────────

/// Errors from export operations.
#[derive(Debug)]
pub enum ExportError {
    Io(std::io::Error),
    InvalidDimensions,
    UnsupportedFormat(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::Io(e) => write!(f, "IO error: {e}"),
            ExportError::InvalidDimensions => {
                write!(f, "invalid image dimensions (zero width or height)")
            }
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

// ── ExportStats ───────────────────────────────────────────────────────────────

/// Statistics from a completed export.
#[derive(Debug, Clone)]
pub struct ExportStats {
    pub bytes_written: u64,
    pub format: ExportFormat,
    pub width: u32,
    pub height: u32,
    pub elapsed_ms: u64,
}

// ── ImageExporter ─────────────────────────────────────────────────────────────

/// Exports wallpaper images in multiple formats.
pub struct ImageExporter;

impl ImageExporter {
    /// Export `pixels` (row-major, RGB triplets) as `format` to `path`.
    pub fn export(
        pixels: &[[u8; 3]],
        width: u32,
        height: u32,
        format: ExportFormat,
        path: &Path,
    ) -> Result<ExportStats, ExportError> {
        if width == 0 || height == 0 {
            return Err(ExportError::InvalidDimensions);
        }
        let start = Instant::now();
        let bytes_written = match format {
            ExportFormat::Png => write_png(pixels, width, height, path)?,
            ExportFormat::Ppm => write_ppm(pixels, width, height, path)?,
            ExportFormat::Bmp => write_bmp(pixels, width, height, path)?,
            ExportFormat::Svg => write_svg(pixels, width, height, path)?,
        };
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

// ── PPM writer ────────────────────────────────────────────────────────────────

fn write_ppm(pixels: &[[u8; 3]], width: u32, height: u32, path: &Path) -> Result<u64, ExportError> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    let header = format!("P6 {} {} 255\n", width, height);
    f.write_all(header.as_bytes())?;
    for px in pixels {
        f.write_all(px)?;
    }
    let bytes = header.len() as u64 + pixels.len() as u64 * 3;
    Ok(bytes)
}

// ── BMP writer ────────────────────────────────────────────────────────────────

/// Write a 24-bit BMP file without external dependencies.
///
/// BMP stores rows bottom-up with 4-byte row alignment.
fn write_bmp(pixels: &[[u8; 3]], width: u32, height: u32, path: &Path) -> Result<u64, ExportError> {
    use std::io::Write;

    // Each row must be padded to a multiple of 4 bytes.
    let row_stride = (width as usize * 3 + 3) & !3;
    let pixel_data_size = row_stride * height as usize;
    let file_size = 54u32 + pixel_data_size as u32; // 14 BITMAPFILEHEADER + 40 BITMAPINFOHEADER

    let mut buf = Vec::with_capacity(file_size as usize);

    // BITMAPFILEHEADER (14 bytes)
    buf.extend_from_slice(b"BM");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(&0u16.to_le_bytes()); // reserved1
    buf.extend_from_slice(&0u16.to_le_bytes()); // reserved2
    buf.extend_from_slice(&54u32.to_le_bytes()); // offset to pixel data

    // BITMAPINFOHEADER (40 bytes)
    buf.extend_from_slice(&40u32.to_le_bytes()); // header size
    buf.extend_from_slice(&(width as i32).to_le_bytes());
    buf.extend_from_slice(&(height as i32).to_le_bytes()); // positive = bottom-up
    buf.extend_from_slice(&1u16.to_le_bytes()); // planes
    buf.extend_from_slice(&24u16.to_le_bytes()); // bits per pixel
    buf.extend_from_slice(&0u32.to_le_bytes()); // compression (BI_RGB)
    buf.extend_from_slice(&(pixel_data_size as u32).to_le_bytes());
    buf.extend_from_slice(&2835i32.to_le_bytes()); // X pixels per meter (~72 DPI)
    buf.extend_from_slice(&2835i32.to_le_bytes()); // Y pixels per meter
    buf.extend_from_slice(&0u32.to_le_bytes()); // colours used
    buf.extend_from_slice(&0u32.to_le_bytes()); // important colours

    // Pixel data — BMP rows are bottom-up, BGR byte order.
    let padding = [0u8; 4];
    let pad_bytes = row_stride - width as usize * 3;
    for row in (0..height as usize).rev() {
        for col in 0..width as usize {
            let px = pixels[row * width as usize + col];
            buf.push(px[2]); // B
            buf.push(px[1]); // G
            buf.push(px[0]); // R
        }
        buf.extend_from_slice(&padding[..pad_bytes]);
    }

    let mut f = std::fs::File::create(path)?;
    f.write_all(&buf)?;
    Ok(buf.len() as u64)
}

// ── SVG writer ────────────────────────────────────────────────────────────────

/// Write an SVG with one `<rect>` per pixel (or per tile cell for reasonable sizes).
///
/// For large images (>64×64), outputs 8×8 pixel tiles to keep SVG size manageable.
fn write_svg(pixels: &[[u8; 3]], width: u32, height: u32, path: &Path) -> Result<u64, ExportError> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;

    // Use 8x8 tiles for large images to keep SVG manageable
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

    // Sample grid — at most 256×256 cells to keep SVG size reasonable
    let max_cells = 256u32;
    let step_x = (width / max_cells).max(1);
    let step_y = (height / max_cells).max(1);

    let mut y = 0u32;
    while y < height {
        let mut x = 0u32;
        while x < width {
            let idx = (y * width + x) as usize;
            if idx < pixels.len() {
                let [r, g, b] = pixels[idx];
                let rx = x * tile;
                let ry = y * tile;
                let rw = (step_x * tile).min(svg_w - rx);
                let rh = (step_y * tile).min(svg_h - ry);
                buf.push_str(&format!(
                    "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#{:02X}{:02X}{:02X}\"/>\n",
                    rx, ry, rw, rh, r, g, b
                ));
            }
            x += step_x;
        }
        y += step_y;
    }

    buf.push_str("</svg>\n");
    f.write_all(buf.as_bytes())?;
    Ok(buf.len() as u64)
}

// ── PNG writer ────────────────────────────────────────────────────────────────

/// Write a PNG using the stdlib-only encoder (same approach as animation.rs).
fn write_png(pixels: &[[u8; 3]], width: u32, height: u32, path: &Path) -> Result<u64, ExportError> {
    use std::io::Write;

    // Build RGBA pixel data (A = 255) for the encoder
    let mut rgba = Vec::with_capacity(pixels.len() * 4);
    for px in pixels {
        rgba.push(px[0]);
        rgba.push(px[1]);
        rgba.push(px[2]);
        rgba.push(255u8);
    }

    encode_png_rgba(path, width, height, &rgba)?;

    // Estimate bytes: signature (8) + IHDR (25) + IDAT (varies) + IEND (12)
    let bytes = 8 + 25 + rgba.len() + 100 + 12;
    Ok(bytes as u64)
}

// ── Shared PNG encoding (mirrors animation.rs) ────────────────────────────────

pub fn encode_png_rgba(
    path: &Path,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<(), std::io::Error> {
    use std::io::Write;

    let mut f = std::fs::File::create(path)?;

    // PNG signature
    f.write_all(&[137, 80, 78, 71, 13, 10, 26, 10])?;

    // IHDR
    let ihdr_data: Vec<u8> = {
        let mut d = Vec::with_capacity(13);
        d.extend_from_slice(&width.to_be_bytes());
        d.extend_from_slice(&height.to_be_bytes());
        d.push(8); // bit depth
        d.push(6); // RGBA
        d.push(0); // compression
        d.push(0); // filter
        d.push(0); // interlace
        d
    };
    write_png_chunk(&mut f, b"IHDR", &ihdr_data)?;

    // Raw scanlines with filter byte 0 (None)
    let row_len = (width * 4) as usize;
    let mut raw = Vec::with_capacity((row_len + 1) * height as usize);
    for row in 0..height as usize {
        raw.push(0u8);
        raw.extend_from_slice(&rgba[row * row_len..(row + 1) * row_len]);
    }

    let compressed = deflate_store(&raw);
    write_png_chunk(&mut f, b"IDAT", &compressed)?;
    write_png_chunk(&mut f, b"IEND", &[])?;

    Ok(())
}

fn write_png_chunk(f: &mut impl std::io::Write, tag: &[u8; 4], data: &[u8]) -> std::io::Result<()> {
    let len = (data.len() as u32).to_be_bytes();
    f.write_all(&len)?;
    f.write_all(tag)?;
    f.write_all(data)?;
    let crc = png_crc(tag, data);
    f.write_all(&crc.to_be_bytes())?;
    Ok(())
}

fn png_crc(tag: &[u8], data: &[u8]) -> u32 {
    let table = crc32_table();
    let mut crc = 0xFFFF_FFFFu32;
    for &b in tag.iter().chain(data.iter()) {
        crc = table[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

fn crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    for n in 0u32..256 {
        let mut c = n;
        for _ in 0..8 {
            if c & 1 != 0 {
                c = 0xEDB8_8320 ^ (c >> 1);
            } else {
                c >>= 1;
            }
        }
        table[n as usize] = c;
    }
    table
}

fn deflate_store(data: &[u8]) -> Vec<u8> {
    const BLOCK_MAX: usize = 65535;
    let mut out = Vec::new();
    out.push(0x78);
    out.push(0x01);

    let chunks: Vec<&[u8]> = data.chunks(BLOCK_MAX).collect();
    for (i, chunk) in chunks.iter().enumerate() {
        let is_last = i == chunks.len() - 1;
        out.push(if is_last { 1 } else { 0 });
        let len = chunk.len() as u16;
        let nlen = !len;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&nlen.to_le_bytes());
        out.extend_from_slice(chunk);
    }
    if data.is_empty() {
        out.push(1);
        out.extend_from_slice(&[0x00, 0x00, 0xFF, 0xFF]);
    }

    let (s1, s2) = data.iter().fold((1u32, 0u32), |(s1, s2), &b| {
        let s1 = (s1 + b as u32) % 65521;
        let s2 = (s2 + s1) % 65521;
        (s1, s2)
    });
    let adler = (s2 << 16) | s1;
    out.extend_from_slice(&adler.to_be_bytes());
    out
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

    // 2. PPM header is correct
    #[test]
    fn test_ppm_header() {
        let tmp = NamedTempFile::new().unwrap();
        let pixels = sample_pixels(2, 2);
        ImageExporter::export(&pixels, 2, 2, ExportFormat::Ppm, tmp.path()).unwrap();
        let content = std::fs::read(tmp.path()).unwrap();
        assert!(content.starts_with(b"P6 2 2 255\n"));
    }

    // 3. PPM pixel data length
    #[test]
    fn test_ppm_pixel_data_length() {
        let tmp = NamedTempFile::new().unwrap();
        let w = 3u32;
        let h = 3u32;
        let pixels = sample_pixels(w, h);
        ImageExporter::export(&pixels, w, h, ExportFormat::Ppm, tmp.path()).unwrap();
        let content = std::fs::read(tmp.path()).unwrap();
        let header = b"P6 3 3 255\n";
        assert_eq!(content.len(), header.len() + (w * h * 3) as usize);
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
}
