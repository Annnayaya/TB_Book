use image::DynamicImage;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReadingDirection {
    RightToLeft, // 日漫 (RTL) - 默认从右向左
    LeftToRight, // 港台/美漫 (LTR) - 从左向右
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ZoomMode {
    FitScreen,
    FitWidth,
    FitHeight,
    Fixed(f32),
}

pub struct ComicArchive {
    pub title: String,
    pub path: PathBuf,
    pub page_entries: Vec<String>,
    pub current_page: usize,
    pub current_image: Option<DynamicImage>,
    pub reading_direction: ReadingDirection,
    pub auto_crop: bool,

    // Zoom & Pan state
    pub zoom_level: f32,
    pub pan_x: f32,
    pub pan_y: f32,
    pub target_pan_x: f32,
    pub target_pan_y: f32,

    // Minimap display timer
    pub minimap_timer: f32,
}

impl ComicArchive {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path_buf = path.as_ref().to_path_buf();
        let file = File::open(&path_buf).map_err(|e| e.to_string())?;
        let mut zip = ZipArchive::new(file).map_err(|e| e.to_string())?;

        let mut image_files = Vec::new();
        for i in 0..zip.len() {
            if let Ok(entry) = zip.by_index(i) {
                let name = entry.name().to_string();
                let lower = name.to_lowercase();
                if (lower.ends_with(".jpg")
                    || lower.ends_with(".jpeg")
                    || lower.ends_with(".png")
                    || lower.ends_with(".webp")
                    || lower.ends_with(".bmp"))
                    && !name.starts_with("__MACOSX")
                {
                    image_files.push(name);
                }
            }
        }

        image_files.sort_by(|a, b| human_sort::compare(a, b));

        if image_files.is_empty() {
            return Err("压缩包内未找到有效漫画图片文件".to_string());
        }

        let title = path_buf
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("未命名漫画")
            .to_string();

        let mut comic = Self {
            title,
            path: path_buf,
            page_entries: image_files,
            current_page: 0,
            current_image: None,
            reading_direction: ReadingDirection::RightToLeft,
            auto_crop: false,
            zoom_level: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            target_pan_x: 0.0,
            target_pan_y: 0.0,
            minimap_timer: 0.0,
        };

        comic.load_current_page();
        Ok(comic)
    }

    pub fn load_current_page(&mut self) {
        if self.current_page >= self.page_entries.len() {
            return;
        }

        let entry_name = &self.page_entries[self.current_page];
        if let Ok(file) = File::open(&self.path) {
            if let Ok(mut zip) = ZipArchive::new(file) {
                if let Ok(mut zip_entry) = zip.by_name(entry_name) {
                    let mut buffer = Vec::new();
                    if zip_entry.read_to_end(&mut buffer).is_ok() {
                        if let Ok(img) = image::load_from_memory(&buffer) {
                            self.current_image = Some(if self.auto_crop {
                                auto_crop_margins(&img)
                            } else {
                                img
                            });
                        }
                    }
                }
            }
        }

        self.reset_zoom();
    }

    pub fn reset_zoom(&mut self) {
        self.zoom_level = 1.0;
        self.pan_x = 0.0;
        self.pan_y = 0.0;
        self.target_pan_x = 0.0;
        self.target_pan_y = 0.0;
        self.minimap_timer = 0.0;
    }

    pub fn set_zoom_preset(&mut self, mode: ZoomMode, screen_w: f32, screen_h: f32) {
        if let Some(img) = &self.current_image {
            let (iw, ih) = (img.width() as f32, img.height() as f32);
            let base_scale = (screen_w / iw).min(screen_h / ih);

            match mode {
                ZoomMode::FitScreen => {
                    self.reset_zoom();
                }
                ZoomMode::FitWidth => {
                    let target_scale = screen_w / iw;
                    self.zoom_level = (target_scale / base_scale).max(1.0);
                    self.center_viewport(screen_w, screen_h);
                    self.minimap_timer = 2.0;
                }
                ZoomMode::FitHeight => {
                    let target_scale = screen_h / ih;
                    self.zoom_level = (target_scale / base_scale).max(1.0);
                    self.center_viewport(screen_w, screen_h);
                    self.minimap_timer = 2.0;
                }
                ZoomMode::Fixed(z) => {
                    self.zoom_level = z.clamp(1.0, 5.0);
                    if self.zoom_level <= 1.05 {
                        self.reset_zoom();
                    } else {
                        self.center_viewport(screen_w, screen_h);
                        self.minimap_timer = 2.0;
                    }
                }
            }
        }
    }

    pub fn zoom_in(&mut self, step: f32, screen_w: f32, screen_h: f32) {
        self.zoom_level = (self.zoom_level + step).min(5.0);
        self.clamp_pan(screen_w, screen_h);
        self.minimap_timer = 2.0;
    }

    pub fn zoom_out(&mut self, step: f32, screen_w: f32, screen_h: f32) {
        self.zoom_level = (self.zoom_level - step).max(1.0);
        if self.zoom_level <= 1.05 {
            self.reset_zoom();
        } else {
            self.clamp_pan(screen_w, screen_h);
            self.minimap_timer = 2.0;
        }
    }

    pub fn pan(&mut self, dx: f32, dy: f32, screen_w: f32, screen_h: f32) {
        if self.zoom_level > 1.0 {
            self.pan_x += dx / self.zoom_level;
            self.pan_y += dy / self.zoom_level;
            self.clamp_pan(screen_w, screen_h);
            self.minimap_timer = 2.0;
        }
    }

    fn clamp_pan(&mut self, screen_w: f32, screen_h: f32) {
        if let Some(img) = &self.current_image {
            let (iw, ih) = (img.width() as f32, img.height() as f32);
            let base_scale = (screen_w / iw).min(screen_h / ih);
            let cur_scale = base_scale * self.zoom_level;

            let view_w_in_img = screen_w / cur_scale;
            let view_h_in_img = screen_h / cur_scale;

            let max_pan_x = (iw - view_w_in_img).max(0.0);
            let max_pan_y = (ih - view_h_in_img).max(0.0);

            self.pan_x = self.pan_x.clamp(0.0, max_pan_x);
            self.pan_y = self.pan_y.clamp(0.0, max_pan_y);
        }
    }

    fn center_viewport(&mut self, screen_w: f32, screen_h: f32) {
        if let Some(img) = &self.current_image {
            let (iw, ih) = (img.width() as f32, img.height() as f32);
            let base_scale = (screen_w / iw).min(screen_h / ih);
            let cur_scale = base_scale * self.zoom_level;

            let view_w_in_img = screen_w / cur_scale;
            let view_h_in_img = screen_h / cur_scale;

            self.pan_x = ((iw - view_w_in_img) / 2.0).max(0.0);
            self.pan_y = ((ih - view_h_in_img) / 2.0).max(0.0);
        }
    }

    pub fn next_page(&mut self) -> bool {
        if self.current_page + 1 < self.page_entries.len() {
            self.current_page += 1;
            self.load_current_page();
            true
        } else {
            false
        }
    }

    pub fn prev_page(&mut self) -> bool {
        if self.current_page > 0 {
            self.current_page -= 1;
            self.load_current_page();
            true
        } else {
            false
        }
    }

    pub fn jump_page(&mut self, page: usize) {
        if page < self.page_entries.len() {
            self.current_page = page;
            self.load_current_page();
        }
    }

    pub fn toggle_reading_direction(&mut self) {
        self.reading_direction = match self.reading_direction {
            ReadingDirection::RightToLeft => ReadingDirection::LeftToRight,
            ReadingDirection::LeftToRight => ReadingDirection::RightToLeft,
        };
    }

    pub fn toggle_auto_crop(&mut self) {
        self.auto_crop = !self.auto_crop;
        self.load_current_page();
    }
}

mod human_sort {
    pub fn compare(a: &str, b: &str) -> std::cmp::Ordering {
        let (mut chars_a, mut chars_b) = (a.chars().peekable(), b.chars().peekable());
        loop {
            match (chars_a.peek(), chars_b.peek()) {
                (None, None) => return std::cmp::Ordering::Equal,
                (None, Some(_)) => return std::cmp::Ordering::Less,
                (Some(_), None) => return std::cmp::Ordering::Greater,
                (Some(ca), Some(cb)) if ca.is_ascii_digit() && cb.is_ascii_digit() => {
                    let num_a = take_number(&mut chars_a);
                    let num_b = take_number(&mut chars_b);
                    if num_a != num_b {
                        return num_a.cmp(&num_b);
                    }
                }
                (Some(ca), Some(cb)) => {
                    let cmp = ca.to_lowercase().cmp(cb.to_lowercase());
                    if cmp != std::cmp::Ordering::Equal {
                        return cmp;
                    }
                    chars_a.next();
                    chars_b.next();
                }
            }
        }
    }

    fn take_number<I: Iterator<Item = char>>(iter: &mut std::iter::Peekable<I>) -> u64 {
        let mut n = 0;
        while let Some(&ch) = iter.peek() {
            if let Some(d) = ch.to_digit(10) {
                n = n * 10 + (d as u64);
                iter.next();
            } else {
                break;
            }
        }
        n
    }
}

fn auto_crop_margins(img: &DynamicImage) -> DynamicImage {
    let (w, h) = (img.width(), img.height());
    if w < 100 || h < 100 {
        return img.clone();
    }
    img.crop_imm(0, 0, w, h)
}

#[cfg(test)]
mod tests {
    use super::ComicArchive;

    #[test]
    fn opens_and_decodes_packaged_sample_cbz() {
        let sample = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("books")
            .join("灌篮高手_样例.cbz");
        let comic = ComicArchive::open(sample).expect("sample CBZ should open");

        assert!(!comic.page_entries.is_empty());
        assert!(comic.current_image.is_some());
    }
}
