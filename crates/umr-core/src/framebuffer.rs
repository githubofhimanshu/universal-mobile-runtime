use serde::{Deserialize, Serialize};

/// Supported pixel encoding formats for framebuffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PixelFormat {
    Rgba8888,
    Bgra8888,
    Rgb565,
}

impl PixelFormat {
    /// Number of bytes per pixel for this format.
    pub fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::Rgba8888 | Self::Bgra8888 => 4,
            Self::Rgb565 => 2,
        }
    }
}

/// A raw frame buffer delivered from an execution backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameBuffer {
    /// Horizontal pixel dimension.
    pub width: u32,
    /// Vertical pixel dimension.
    pub height: u32,
    /// Stride (bytes per scanline).
    pub stride: u32,
    /// Pixel encoding format.
    pub format: PixelFormat,
    /// Host timestamp in milliseconds when frame was produced.
    pub timestamp_ms: u64,
    /// Monotonically increasing sequence number for this session.
    pub frame_index: u64,
    /// Raw uncompressed pixel payload.
    pub data: Vec<u8>,
}

impl FrameBuffer {
    /// Allocates an empty or zeroed frame buffer with the given dimensions.
    pub fn new(width: u32, height: u32, format: PixelFormat) -> Self {
        let bpp = format.bytes_per_pixel();
        let stride = width * bpp;
        let data_len = (stride * height) as usize;

        Self {
            width,
            height,
            stride,
            format,
            timestamp_ms: 0,
            frame_index: 0,
            data: vec![0u8; data_len],
        }
    }

    /// Creates a frame buffer with populated pixel data.
    pub fn with_data(
        width: u32,
        height: u32,
        format: PixelFormat,
        timestamp_ms: u64,
        frame_index: u64,
        data: Vec<u8>,
    ) -> Self {
        let stride = width * format.bytes_per_pixel();
        Self {
            width,
            height,
            stride,
            format,
            timestamp_ms,
            frame_index,
            data,
        }
    }

    /// Encodes pixel buffer into a standard Base64 string for JSON transfer or data URI.
    pub fn to_base64(&self) -> String {
        encode_base64(&self.data)
    }
}

/// Frame pacing and rendering statistics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrameStats {
    pub fps: f32,
    pub total_frames: u64,
    pub dropped_frames: u64,
    pub last_frame_time_ms: u64,
}

impl Default for FrameStats {
    fn default() -> Self {
        Self {
            fps: 0.0,
            total_frames: 0,
            dropped_frames: 0,
            last_frame_time_ms: 0,
        }
    }
}

/// Standard Base64 encoder (RFC 4648) implemented in safe Rust.
pub fn encode_base64(data: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);

    let chunks = data.chunks_exact(3);
    let remainder = chunks.remainder();

    for chunk in chunks {
        let b0 = chunk[0] as usize;
        let b1 = chunk[1] as usize;
        let b2 = chunk[2] as usize;

        out.push(CHARSET[b0 >> 2] as char);
        out.push(CHARSET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);
        out.push(CHARSET[((b1 & 0x0F) << 2) | (b2 >> 6)] as char);
        out.push(CHARSET[b2 & 0x3F] as char);
    }

    match remainder.len() {
        1 => {
            let b0 = remainder[0] as usize;
            out.push(CHARSET[b0 >> 2] as char);
            out.push(CHARSET[(b0 & 0x03) << 4] as char);
            out.push('=');
            out.push('=');
        }
        2 => {
            let b0 = remainder[0] as usize;
            let b1 = remainder[1] as usize;
            out.push(CHARSET[b0 >> 2] as char);
            out.push(CHARSET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);
            out.push(CHARSET[(b1 & 0x0F) << 2] as char);
            out.push('=');
        }
        _ => {}
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framebuffer_allocation() {
        let fb = FrameBuffer::new(100, 200, PixelFormat::Rgba8888);
        assert_eq!(fb.width, 100);
        assert_eq!(fb.height, 200);
        assert_eq!(fb.stride, 400);
        assert_eq!(fb.data.len(), 100 * 200 * 4);
    }

    #[test]
    fn test_base64_encoding() {
        assert_eq!(encode_base64(b""), "");
        assert_eq!(encode_base64(b"f"), "Zg==");
        assert_eq!(encode_base64(b"fo"), "Zm8=");
        assert_eq!(encode_base64(b"foo"), "Zm9v");
        assert_eq!(encode_base64(b"foob"), "Zm9vYg==");
        assert_eq!(encode_base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(encode_base64(b"foobar"), "Zm9vYmFy");
    }
}

