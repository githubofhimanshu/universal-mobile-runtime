use std::time::Instant;

use umr_core::{ExecutionMode, FrameBuffer, FrameStats, PixelFormat};

use crate::input::ActiveTouchState;

/// Display metrics configuring the framebuffer dimension and density.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayMetrics {
    pub width: u32,
    pub height: u32,
    pub density_dpi: u32,
}

impl Default for DisplayMetrics {
    fn default() -> Self {
        Self {
            width: 360,
            height: 640,
            density_dpi: 160,
        }
    }
}

/// Simulated Android Display Compositor generating RGBA8888 frames.
pub struct FrameBufferPipeline {
    metrics: DisplayMetrics,
    frame_index: u64,
    start_time: Instant,
    last_frame_instant: Instant,
    smoothed_fps: f32,
}

impl FrameBufferPipeline {
    pub fn new(metrics: DisplayMetrics) -> Self {
        let now = Instant::now();
        Self {
            metrics,
            frame_index: 0,
            start_time: now,
            last_frame_instant: now,
            smoothed_fps: 60.0,
        }
    }

    pub fn metrics(&self) -> &DisplayMetrics {
        &self.metrics
    }

    pub fn stats(&self) -> FrameStats {
        FrameStats {
            fps: self.smoothed_fps,
            total_frames: self.frame_index,
            dropped_frames: 0,
            last_frame_time_ms: self.last_frame_instant.duration_since(self.start_time).as_millis() as u64,
        }
    }

    /// Renders the next composite frame.
    pub fn render_frame(
        &mut self,
        package_name: &str,
        execution_mode: ExecutionMode,
        active_touch: Option<ActiveTouchState>,
    ) -> FrameBuffer {
        self.frame_index += 1;
        let now = Instant::now();
        let delta = now.duration_since(self.last_frame_instant).as_secs_f32();
        self.last_frame_instant = now;

        if delta > 0.0001 {
            let instantaneous_fps = (1.0 / delta).clamp(1.0, 120.0);
            // Exponential smoothing
            self.smoothed_fps = self.smoothed_fps * 0.9 + instantaneous_fps * 0.1;
        }

        let w = self.metrics.width;
        let h = self.metrics.height;
        let timestamp_ms = now.duration_since(self.start_time).as_millis() as u64;

        let mut fb = FrameBuffer::new(w, h, PixelFormat::Rgba8888);
        let pixels = &mut fb.data;

        // 1. Dark Android Surface Background (#121212)
        fill_rect(pixels, w, 0, 0, w, h, 0x12, 0x12, 0x12, 0xFF);

        // 2. Android Status Bar (top 28px) (#1F1F1F)
        let status_h = 28;
        fill_rect(pixels, w, 0, 0, w, status_h, 0x1F, 0x1F, 0x1F, 0xFF);
        // Wi-Fi bars simulated (3 small vertical bars)
        fill_rect(pixels, w, w - 55, 16, 3, 6, 0xEE, 0xEE, 0xEE, 0xFF);
        fill_rect(pixels, w, w - 50, 13, 3, 9, 0xEE, 0xEE, 0xEE, 0xFF);
        fill_rect(pixels, w, w - 45, 10, 3, 12, 0xEE, 0xEE, 0xEE, 0xFF);
        // Battery outline (rectangle with tip)
        fill_rect(pixels, w, w - 30, 9, 18, 10, 0xAA, 0xAA, 0xAA, 0xFF);
        fill_rect(pixels, w, w - 28, 11, 14, 6, 0x4C, 0xAF, 0x50, 0xFF); // green fill
        fill_rect(pixels, w, w - 12, 12, 2, 4, 0xAA, 0xAA, 0xAA, 0xFF); // tip
        // Clock readout "12:00"
        draw_text(pixels, w, 12, 9, "12:00", 0xEE, 0xEE, 0xEE);

        // 3. Application Action Bar (height 48px) (#00796B Material Teal)
        let bar_y = status_h;
        let bar_h = 44;
        fill_rect(pixels, w, 0, bar_y, w, bar_h, 0x00, 0x79, 0x6B, 0xFF);
        // App title
        draw_text(pixels, w, 14, bar_y + 16, "UMR RUNTIME", 0xFF, 0xFF, 0xFF);

        // 4. Central Application Viewport Card (#1E1E1E)
        let card_x = 16;
        let card_y = bar_y + bar_h + 20;
        let card_w = w.saturating_sub(32);
        let card_h = 240;
        fill_rect(pixels, w, card_x, card_y, card_w, card_h, 0x24, 0x24, 0x24, 0xFF);
        // Inner card accent border
        fill_rect(pixels, w, card_x, card_y, card_w, 4, 0x00, 0xB0, 0xFF, 0xFF);

        // Card Text Content
        draw_text(pixels, w, card_x + 14, card_y + 20, "PACKAGE:", 0x9E, 0x9E, 0x9E);
        let clean_pkg = if package_name.len() > 24 {
            &package_name[..24]
        } else {
            package_name
        };
        draw_text(pixels, w, card_x + 14, card_y + 36, clean_pkg, 0x00, 0xE5, 0xFF);

        draw_text(pixels, w, card_x + 14, card_y + 60, "MODE:", 0x9E, 0x9E, 0x9E);
        let mode_str = match execution_mode {
            ExecutionMode::LocalNative => "LOCAL NATIVE",
            ExecutionMode::LocalVirtualized => "LOCAL VIRTUALIZED",
            ExecutionMode::LocalTranslated => "LOCAL TRANSLATED",
            ExecutionMode::LocalHle => "LOCAL HLE",
            ExecutionMode::RemoteStream => "REMOTE STREAM",
            ExecutionMode::Unsupported => "UNSUPPORTED",
        };
        draw_text(pixels, w, card_x + 14, card_y + 76, mode_str, 0x76, 0xFF, 0x03);

        draw_text(pixels, w, card_x + 14, card_y + 100, "FRAME TICK:", 0x9E, 0x9E, 0x9E);
        let tick_str = format!("#{}", self.frame_index);
        draw_text(pixels, w, card_x + 14, card_y + 116, &tick_str, 0xFF, 0xFF, 0xFF);

        draw_text(pixels, w, card_x + 14, card_y + 140, "PACING / FPS:", 0x9E, 0x9E, 0x9E);
        let fps_str = format!("{:.1} FPS", self.smoothed_fps);
        draw_text(pixels, w, card_x + 14, card_y + 156, &fps_str, 0xFF, 0xD6, 0x00);

        // Visual pulsing progress indicator bar inside card
        let pulse_pos = ((self.frame_index * 4) % (card_w as u64 - 28)) as u32;
        fill_rect(pixels, w, card_x + 14, card_y + 185, card_w - 28, 6, 0x42, 0x42, 0x42, 0xFF);
        fill_rect(pixels, w, card_x + 14 + pulse_pos, card_y + 185, 24, 6, 0x00, 0xE5, 0xFF, 0xFF);

        draw_text(pixels, w, card_x + 14, card_y + 208, "STATUS: RUNNING ACTIVE", 0x81, 0xC7, 0x84);

        // 5. Interactive Touch Visualization (if user touched or clicked)
        if let Some(touch) = active_touch {
            let tx = touch.px.clamp(16, w.saturating_sub(16));
            let ty = touch.py.clamp(16, h.saturating_sub(16));
            // Translucent outer circle
            draw_circle(pixels, w, h, tx, ty, 18, 0x00, 0xE5, 0xFF, 0x77);
            // Solid center dot
            draw_circle(pixels, w, h, tx, ty, 5, 0xFF, 0xFF, 0xFF, 0xFF);
            // Coordinate tag
            let coord_str = format!("({}, {})", touch.px, touch.py);
            draw_text(pixels, w, tx.saturating_add(10), ty.saturating_sub(12), &coord_str, 0x00, 0xE5, 0xFF);
        }

        // 6. Android Navigation Bar (bottom 44px) (#0A0A0A)
        let nav_h = 44;
        let nav_y = h.saturating_sub(nav_h);
        fill_rect(pixels, w, 0, nav_y, w, nav_h, 0x0A, 0x0A, 0x0A, 0xFF);

        // Navigation Buttons
        let center_x = w / 2;
        let back_x = center_x / 2;
        let recents_x = center_x + back_x;
        let btn_y = nav_y + (nav_h / 2);

        // Back button: Left-pointing triangle
        draw_triangle_left(pixels, w, back_x, btn_y, 7, 0xCC, 0xCC, 0xCC);
        // Home button: Circle
        draw_circle(pixels, w, h, center_x, btn_y, 7, 0xCC, 0xCC, 0xCC, 0xFF);
        // Recents button: Rounded Square
        fill_rect(pixels, w, recents_x - 6, btn_y - 6, 12, 12, 0xCC, 0xCC, 0xCC, 0xFF);

        fb.timestamp_ms = timestamp_ms;
        fb.frame_index = self.frame_index;
        fb
    }
}

// ----------------- Lightweight 2D Software Rasterizer -----------------

fn fill_rect(
    buf: &mut [u8],
    stride_w: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    r: u8,
    g: u8,
    b: u8,
    a: u8,
) {
    let total_pixels = buf.len() / 4;
    let max_y = (total_pixels as u32) / stride_w;

    let end_x = (x + w).min(stride_w);
    let end_y = (y + h).min(max_y);

    for row in y..end_y {
        let row_offset = (row * stride_w * 4) as usize;
        for col in x..end_x {
            let offset = row_offset + (col * 4) as usize;
            if offset + 3 < buf.len() {
                buf[offset] = r;
                buf[offset + 1] = g;
                buf[offset + 2] = b;
                buf[offset + 3] = a;
            }
        }
    }
}

fn draw_circle(
    buf: &mut [u8],
    stride_w: u32,
    stride_h: u32,
    cx: u32,
    cy: u32,
    radius: i32,
    r: u8,
    g: u8,
    b: u8,
    a: u8,
) {
    let r2 = radius * radius;
    let min_x = (cx as i32 - radius).max(0) as u32;
    let max_x = (cx as i32 + radius).min(stride_w as i32 - 1) as u32;
    let min_y = (cy as i32 - radius).max(0) as u32;
    let max_y = (cy as i32 + radius).min(stride_h as i32 - 1) as u32;

    for y in min_y..=max_y {
        let dy = y as i32 - cy as i32;
        let dy2 = dy * dy;
        let row_offset = (y * stride_w * 4) as usize;
        for x in min_x..=max_x {
            let dx = x as i32 - cx as i32;
            if dx * dx + dy2 <= r2 {
                let offset = row_offset + (x * 4) as usize;
                if offset + 3 < buf.len() {
                    if a == 0xFF {
                        buf[offset] = r;
                        buf[offset + 1] = g;
                        buf[offset + 2] = b;
                        buf[offset + 3] = a;
                    } else {
                        // Alpha blend
                        let alpha = a as u32;
                        let inv_alpha = 255 - alpha;
                        buf[offset] = ((r as u32 * alpha + buf[offset] as u32 * inv_alpha) / 255) as u8;
                        buf[offset + 1] = ((g as u32 * alpha + buf[offset + 1] as u32 * inv_alpha) / 255) as u8;
                        buf[offset + 2] = ((b as u32 * alpha + buf[offset + 2] as u32 * inv_alpha) / 255) as u8;
                        buf[offset + 3] = 0xFF;
                    }
                }
            }
        }
    }
}

fn draw_triangle_left(
    buf: &mut [u8],
    stride_w: u32,
    cx: u32,
    cy: u32,
    size: i32,
    r: u8,
    g: u8,
    b: u8,
) {
    for dy in -size..=size {
        let y = (cy as i32 + dy) as u32;
        let span = (dy.abs() * size) / size;
        let min_x = (cx as i32 - size + span) as u32;
        let max_x = (cx as i32 + size) as u32;
        for x in min_x..=max_x {
            let offset = ((y * stride_w + x) * 4) as usize;
            if offset + 3 < buf.len() {
                buf[offset] = r;
                buf[offset + 1] = g;
                buf[offset + 2] = b;
                buf[offset + 3] = 0xFF;
            }
        }
    }
}

// ----------------- Tiny 3x5 Pixel Font -----------------
// 15 bits per glyph: 5 rows of 3 columns
fn get_glyph_bitmap(c: char) -> u16 {
    match c {
        '0' => 0b111_101_101_101_111,
        '1' => 0b010_110_010_010_111,
        '2' => 0b111_001_111_100_111,
        '3' => 0b111_001_111_001_111,
        '4' => 0b101_101_111_001_001,
        '5' => 0b111_100_111_001_111,
        '6' => 0b111_100_111_101_111,
        '7' => 0b111_001_010_010_010,
        '8' => 0b111_101_111_101_111,
        '9' => 0b111_101_111_001_111,
        ':' => 0b000_010_000_010_000,
        '.' => 0b000_000_000_000_010,
        '#' => 0b010_111_010_111_010,
        '(' => 0b010_100_100_100_010,
        ')' => 0b010_001_001_001_010,
        ',' => 0b000_000_000_010_100,
        '-' => 0b000_000_111_000_000,
        '/' => 0b001_001_010_100_100,
        ' ' => 0b000_000_000_000_000,
        'A' => 0b010_101_111_101_101,
        'B' => 0b110_101_110_101_110,
        'C' => 0b011_100_100_100_011,
        'D' => 0b110_101_101_101_110,
        'E' => 0b111_100_110_100_111,
        'F' => 0b111_100_110_100_100,
        'G' => 0b011_100_101_101_011,
        'H' => 0b101_101_111_101_101,
        'I' => 0b111_010_010_010_111,
        'J' => 0b001_001_001_101_010,
        'K' => 0b101_110_100_110_101,
        'L' => 0b100_100_100_100_111,
        'M' => 0b101_111_101_101_101,
        'N' => 0b110_101_101_101_101,
        'O' => 0b010_101_101_101_010,
        'P' => 0b111_101_111_100_100,
        'Q' => 0b010_101_101_111_011,
        'R' => 0b110_101_110_101_101,
        'S' => 0b011_100_010_001_110,
        'T' => 0b111_010_010_010_010,
        'U' => 0b101_101_101_101_111,
        'V' => 0b101_101_101_010_010,
        'W' => 0b101_101_101_111_101,
        'X' => 0b101_101_010_101_101,
        'Y' => 0b101_101_010_010_010,
        'Z' => 0b111_001_010_100_111,
        _ => 0b000_000_000_000_000,
    }
}

fn draw_text(
    buf: &mut [u8],
    stride_w: u32,
    start_x: u32,
    start_y: u32,
    text: &str,
    r: u8,
    g: u8,
    b: u8,
) {
    let mut cur_x = start_x;
    for ch in text.to_ascii_uppercase().chars() {
        let glyph = get_glyph_bitmap(ch);
        // Each glyph is 3 cols x 5 rows
        for row in 0..5 {
            let y = start_y + row;
            let row_offset = (y * stride_w * 4) as usize;
            for col in 0..3 {
                let shift = 14 - (row * 3 + col);
                if (glyph >> shift) & 1 == 1 {
                    let x = cur_x + col;
                    let offset = row_offset + (x * 4) as usize;
                    if offset + 3 < buf.len() {
                        buf[offset] = r;
                        buf[offset + 1] = g;
                        buf[offset + 2] = b;
                        buf[offset + 3] = 0xFF;
                    }
                }
            }
        }
        cur_x += 4; // 3 width + 1 spacing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framebuffer_pipeline_rendering() {
        let metrics = DisplayMetrics {
            width: 360,
            height: 640,
            density_dpi: 160,
        };
        let mut pipeline = FrameBufferPipeline::new(metrics);

        let fb = pipeline.render_frame(
            "org.umr.demo",
            ExecutionMode::LocalNative,
            Some(ActiveTouchState {
                px: 180,
                py: 320,
                action: umr_core::PointerAction::Down,
                received_at: Instant::now(),
            }),
        );

        assert_eq!(fb.width, 360);
        assert_eq!(fb.height, 640);
        assert_eq!(fb.data.len(), 360 * 640 * 4);
        assert_eq!(fb.frame_index, 1);

        // Check base64 encoding works on the generated frame
        let b64 = fb.to_base64();
        assert!(!b64.is_empty());
        assert_eq!(b64.len() % 4, 0);

        let stats = pipeline.stats();
        assert_eq!(stats.total_frames, 1);
    }
}

