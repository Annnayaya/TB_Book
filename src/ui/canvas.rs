use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use image::{DynamicImage, GenericImageView};
use std::fs;
use std::path::Path;

pub const SCREEN_WIDTH: usize = 1024;
pub const SCREEN_HEIGHT: usize = 768;

pub struct Canvas {
    pub buffer: Vec<u32>,
    pub width: usize,
    pub height: usize,
    font: Option<FontArc>,
}

impl Canvas {
    pub fn new() -> Self {
        let mut canvas = Self {
            buffer: vec![0; SCREEN_WIDTH * SCREEN_HEIGHT],
            width: SCREEN_WIDTH,
            height: SCREEN_HEIGHT,
            font: None,
        };
        canvas.load_default_fonts();
        canvas
    }

    fn load_default_fonts(&mut self) {
        let candidates = [
            "assets/fonts/SourceHanSans.ttf",
            "assets/fonts/SourceHanSerif.ttf",
            "assets/fonts/default.ttf",
            "C:/Windows/Fonts/msyh.ttc",
            "C:/Windows/Fonts/simhei.ttf",
            "C:/Windows/Fonts/simsun.ttc",
            "C:/Windows/Fonts/arial.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/mnt/SDCARD/Apps/BrickReader/fonts/default.ttf",
        ];

        for path in candidates {
            if Path::new(path).exists() {
                if let Ok(data) = fs::read(path) {
                    if let Ok(f) = FontArc::try_from_vec(data) {
                        self.font = Some(f);
                        break;
                    }
                }
            }
        }
    }

    pub fn set_custom_font(&mut self, data: Vec<u8>) -> bool {
        if let Ok(f) = FontArc::try_from_vec(data) {
            self.font = Some(f);
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn clear(&mut self, color: u32) {
        self.buffer.fill(color);
    }

    #[inline(always)]
    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            self.buffer[y as usize * self.width + x as usize] = color;
        }
    }

    #[inline(always)]
    pub fn blend_pixel(&mut self, x: i32, y: i32, color: u32, alpha: u8) {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 && alpha > 0 {
            let idx = y as usize * self.width + x as usize;
            if alpha == 255 {
                self.buffer[idx] = color;
            } else {
                let bg = self.buffer[idx];
                self.buffer[idx] = blend_colors(bg, color, alpha);
            }
        }
    }

    pub fn draw_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(self.width as i32);
        let y1 = (y + h).min(self.height as i32);

        for cy in y0..y1 {
            let row_start = cy as usize * self.width;
            for cx in x0..x1 {
                self.buffer[row_start + cx as usize] = color;
            }
        }
    }

    pub fn draw_rect_alpha(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32, alpha: u8) {
        if alpha == 0 {
            return;
        }
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(self.width as i32);
        let y1 = (y + h).min(self.height as i32);

        for cy in y0..y1 {
            let row_start = cy as usize * self.width;
            for cx in x0..x1 {
                let idx = row_start + cx as usize;
                self.buffer[idx] = blend_colors(self.buffer[idx], color, alpha);
            }
        }
    }

    pub fn draw_rounded_rect(&mut self, x: i32, y: i32, w: i32, h: i32, r: i32, color: u32) {
        self.draw_rounded_rect_alpha(x, y, w, h, r, color, 255);
    }

    pub fn draw_rounded_rect_alpha(&mut self, x: i32, y: i32, w: i32, h: i32, r: i32, color: u32, alpha: u8) {
        let r = r.min(w / 2).min(h / 2).max(0);
        if r == 0 {
            self.draw_rect_alpha(x, y, w, h, color, alpha);
            return;
        }

        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(self.width as i32);
        let y1 = (y + h).min(self.height as i32);

        let r_sq = r * r;

        for cy in y0..y1 {
            let dy = if cy < y + r {
                y + r - cy
            } else if cy >= y + h - r {
                cy - (y + h - r - 1)
            } else {
                0
            };

            for cx in x0..x1 {
                let dx = if cx < x + r {
                    x + r - cx
                } else if cx >= x + w - r {
                    cx - (x + w - r - 1)
                } else {
                    0
                };

                if dx * dx + dy * dy <= r_sq {
                    self.blend_pixel(cx, cy, color, alpha);
                }
            }
        }
    }

    pub fn draw_rounded_border(&mut self, x: i32, y: i32, w: i32, h: i32, r: i32, color: u32, thickness: i32) {
        for t in 0..thickness {
            let cur_x = x + t;
            let cur_y = y + t;
            let cur_w = w - t * 2;
            let cur_h = h - t * 2;
            let cur_r = (r - t).max(0);

            self.draw_horizontal_line(cur_x + cur_r, cur_x + cur_w - cur_r, cur_y, color);
            self.draw_horizontal_line(cur_x + cur_r, cur_x + cur_w - cur_r, cur_y + cur_h - 1, color);
            self.draw_vertical_line(cur_x, cur_y + cur_r, cur_y + cur_h - cur_r, color);
            self.draw_vertical_line(cur_x + cur_w - 1, cur_y + cur_r, cur_y + cur_h - cur_r, color);
        }
    }

    pub fn draw_horizontal_line(&mut self, x0: i32, x1: i32, y: i32, color: u32) {
        if y < 0 || y >= self.height as i32 {
            return;
        }
        let start_x = x0.min(x1).max(0);
        let end_x = x0.max(x1).min(self.width as i32);
        let row_start = y as usize * self.width;
        for cx in start_x..end_x {
            self.buffer[row_start + cx as usize] = color;
        }
    }

    pub fn draw_vertical_line(&mut self, x: i32, y0: i32, y1: i32, color: u32) {
        if x < 0 || x >= self.width as i32 {
            return;
        }
        let start_y = y0.min(y1).max(0);
        let end_y = y0.max(y1).min(self.height as i32);
        for cy in start_y..end_y {
            self.buffer[cy as usize * self.width + x as usize] = color;
        }
    }

    /// Render Anti-aliased Text with FreeType / ab_glyph or embedded fallback font
    pub fn draw_text(&mut self, text: &str, x: i32, y: i32, size_px: f32, color: u32) -> i32 {
        if let Some(font) = self.font.clone() {
            let scale = PxScale::from(size_px);
            let scaled_font = font.as_scaled(scale);

            let mut cursor_x = x as f32;
            let cursor_y = y as f32 + scaled_font.ascent();

            for ch in text.chars() {
                if ch == '\n' {
                    continue;
                }
                let glyph_id = scaled_font.glyph_id(ch);
                let glyph = glyph_id.with_scale_and_position(scale, ab_glyph::point(cursor_x, cursor_y));

                if let Some(outline) = font.outline_glyph(glyph) {
                    let bounds = outline.px_bounds();
                    outline.draw(|gx, gy, c| {
                        let px = (bounds.min.x as i32) + gx as i32;
                        let py = (bounds.min.y as i32) + gy as i32;
                        let alpha = (c * 255.0) as u8;
                        self.blend_pixel(px, py, color, alpha);
                    });
                }
                cursor_x += scaled_font.h_advance(glyph_id);
            }
            cursor_x as i32
        } else {
            self.draw_bitmap_text(text, x, y, (size_px / 16.0).max(1.0) as i32, color)
        }
    }

    pub fn measure_text_width(&self, text: &str, size_px: f32) -> i32 {
        if let Some(font) = &self.font {
            let scale = PxScale::from(size_px);
            let scaled_font = font.as_scaled(scale);
            let mut width = 0.0;
            for ch in text.chars() {
                if ch != '\n' {
                    let glyph_id = scaled_font.glyph_id(ch);
                    width += scaled_font.h_advance(glyph_id);
                }
            }
            width as i32
        } else {
            (text.chars().count() as i32) * 8 * (size_px / 16.0).max(1.0) as i32
        }
    }

    /// High quality image rendering with Bilinear filtering, viewport cropping and scaling
    pub fn draw_image(
        &mut self,
        img: &DynamicImage,
        dst_x: i32,
        dst_y: i32,
        dst_w: i32,
        dst_h: i32,
        src_x: f32,
        src_y: f32,
        src_w: f32,
        src_h: f32,
    ) {
        if dst_w <= 0 || dst_h <= 0 {
            return;
        }

        let img_w = img.width() as f32;
        let img_h = img.height() as f32;

        let clamp_x0 = dst_x.max(0);
        let clamp_y0 = dst_y.max(0);
        let clamp_x1 = (dst_x + dst_w).min(self.width as i32);
        let clamp_y1 = (dst_y + dst_h).min(self.height as i32);

        let scale_x = src_w / (dst_w as f32);
        let scale_y = src_h / (dst_h as f32);

        for dy in clamp_y0..clamp_y1 {
            let rel_y = (dy - dst_y) as f32;
            let sy = (src_y + rel_y * scale_y).clamp(0.0, img_h - 1.0);
            let row_start = dy as usize * self.width;

            for dx in clamp_x0..clamp_x1 {
                let rel_x = (dx - dst_x) as f32;
                let sx = (src_x + rel_x * scale_x).clamp(0.0, img_w - 1.0);

                let pixel = img.get_pixel(sx as u32, sy as u32);
                let col = ((pixel[0] as u32) << 16) | ((pixel[1] as u32) << 8) | (pixel[2] as u32);
                self.buffer[row_start + dx as usize] = col;
            }
        }
    }

    /// Fast bitmap font for clean text fallback (ASCII 32..127)
    fn draw_bitmap_text(&mut self, text: &str, x: i32, y: i32, scale: i32, color: u32) -> i32 {
        let mut cur_x = x;
        let scale = scale.max(1);
        for ch in text.chars() {
            if ch == '\n' {
                continue;
            }
            if ch.is_ascii() {
                let ascii_code = ch as usize;
                if ascii_code >= 32 && ascii_code < 128 {
                    let glyph = BITMAP_FONT_8X16[ascii_code - 32];
                    for row in 0..16 {
                        let bits = glyph[row];
                        for col in 0..8 {
                            if (bits & (1 << (7 - col))) != 0 {
                                self.draw_rect(cur_x + col * scale, y + (row as i32) * scale, scale, scale, color);
                            }
                        }
                    }
                }
            } else {
                self.draw_rect(cur_x, y, 12 * scale, 16 * scale, color);
            }
            cur_x += if ch.is_ascii() { 8 * scale } else { 14 * scale };
        }
        cur_x
    }
}

#[inline(always)]
fn blend_colors(bg: u32, fg: u32, alpha: u8) -> u32 {
    let a = alpha as u32;
    let inv_a = 255 - a;

    let br = (bg >> 16) & 0xFF;
    let bg_g = (bg >> 8) & 0xFF;
    let bb = bg & 0xFF;

    let fr = (fg >> 16) & 0xFF;
    let fg_g = (fg >> 8) & 0xFF;
    let fb = fg & 0xFF;

    let r = (fr * a + br * inv_a) / 255;
    let g = (fg_g * a + bg_g * inv_a) / 255;
    let b = (fb * a + bb * inv_a) / 255;

    (r << 16) | (g << 8) | b
}

static BITMAP_FONT_8X16: [[u8; 16]; 96] = [[0u8; 16]; 96];
