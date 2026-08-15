use crate::config::AppSettings;
use crate::core::comic_engine::ReadingDirection;
use crate::ui::canvas::{Canvas, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::ui::theme::ThemePalette;
use crate::ui::widgets::Widgets;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    FolderPicker,
}

pub struct SettingsView {
    pub tab: SettingsTab,
    pub selected_index: usize,

    // Folder Picker sub-state
    pub browse_path: PathBuf,
    pub folder_entries: Vec<PathBuf>,
    pub folder_selected_index: usize,
}

impl SettingsView {
    pub fn new(current_library_path: &Path) -> Self {
        let initial_path = if current_library_path.exists() {
            current_library_path.to_path_buf()
        } else {
            PathBuf::from(".")
        };

        let mut view = Self {
            tab: SettingsTab::General,
            selected_index: 0,
            browse_path: initial_path,
            folder_entries: Vec::new(),
            folder_selected_index: 0,
        };
        view.refresh_folder_entries();
        view
    }

    pub fn refresh_folder_entries(&mut self) {
        let mut entries = Vec::new();

        // Add parent directory option if available
        if let Some(parent) = self.browse_path.parent() {
            entries.push(parent.to_path_buf());
        }

        if let Ok(read_dir) = fs::read_dir(&self.browse_path) {
            for entry in read_dir.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if !name.starts_with('.') && name != "target" {
                            entries.push(entry.path());
                        }
                    }
                }
            }
        }

        entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
        self.folder_entries = entries;
        if self.folder_selected_index >= self.folder_entries.len() {
            self.folder_selected_index = self.folder_entries.len().saturating_sub(1);
        }
    }

    pub fn render(&self, canvas: &mut Canvas, settings: &AppSettings, palette: &ThemePalette) {
        canvas.clear(palette.background);

        match self.tab {
            SettingsTab::General => self.render_general_settings(canvas, settings, palette),
            SettingsTab::FolderPicker => self.render_folder_picker(canvas, palette),
        }
    }

    fn render_general_settings(&self, canvas: &mut Canvas, settings: &AppSettings, palette: &ThemePalette) {
        // Top Header
        Widgets::draw_header(canvas, "⚙️ 设置中心 (Settings)", palette);

        // Center card container
        let card_w = 860;
        let card_h = 600;
        let card_x = (SCREEN_WIDTH as i32 - card_w) / 2;
        let card_y = 65;

        canvas.draw_rounded_rect_alpha(card_x + 4, card_y + 6, card_w, card_h, 16, 0x000000, 50);
        canvas.draw_rounded_rect(card_x, card_y, card_w, card_h, 16, palette.card_bg);
        canvas.draw_rounded_border(card_x, card_y, card_w, card_h, 16, palette.border, 2);

        // Setting Items List
        let items: [(&str, String); 7] = [
            ("📂 书库目录 (Library Path)", settings.library_path.to_string_lossy().to_string()),
            ("🔤 默认字体大小 (Font Size)", format!("<  {:.0} px  >", settings.font_size)),
            ("📏 默认行距 (Line Spacing)", format!("<  {:.1} x  >", settings.line_spacing)),
            ("🎨 界面主题 (Theme)", format!("<  {}  >", settings.theme.name())),
            (
                "📖 漫画阅读顺序 (Manga Order)",
                match settings.default_reading_direction {
                    ReadingDirection::RightToLeft => "<  日漫 (RTL 从右向左)  >".to_string(),
                    ReadingDirection::LeftToRight => "<  国漫/美漫 (LTR 从左向右)  >".to_string(),
                },
            ),
            (
                "💡 RGB 氛围灯 (RGB Light)",
                if settings.rgb_led_enabled { "<  开启 (On)  >".to_string() } else { "<  关闭 (Off)  >".to_string() },
            ),
            ("💾 保存并返回书架 (Save & Exit)", "按 (A) 或 (B) 返回".to_string()),
        ];

        let item_h = 68;
        let item_start_y = card_y + 24;

        for (i, (label, val_str)) in items.iter().enumerate() {
            let iy = item_start_y + (i as i32) * item_h;
            let ix = card_x + 24;
            let iw = card_w - 48;
            let is_selected = i == self.selected_index;

            if is_selected {
                canvas.draw_rounded_rect(ix, iy, iw, item_h - 10, 10, palette.card_selected_bg);
                canvas.draw_rounded_border(ix, iy, iw, item_h - 10, 10, palette.accent, 2);
            } else {
                canvas.draw_rounded_rect_alpha(ix, iy, iw, item_h - 10, 10, palette.background, 120);
                canvas.draw_rounded_border(ix, iy, iw, item_h - 10, 10, palette.border, 1);
            }

            // Left Label
            let label_col = if is_selected { palette.accent } else { palette.text_primary };
            canvas.draw_text(label, ix + 20, iy + 16, 22.0, label_col);

            // Right Value
            let val_w = canvas.measure_text_width(val_str, 20.0);
            let val_x = ix + iw - val_w - 20;
            let val_col = if is_selected { palette.accent } else { palette.text_secondary };
            canvas.draw_text(val_str, val_x, iy + 16, 20.0, val_col);
        }

        // Bottom Navigation Instructions
        let tip_text = "(十字键 上/下) 切换选项    (左/右) 调整数值    (A) 确认/浏览目录    (B) 保存返回";
        let tw = canvas.measure_text_width(tip_text, 18.0);
        canvas.draw_text(
            tip_text,
            (SCREEN_WIDTH as i32 - tw) / 2,
            (SCREEN_HEIGHT as i32) - 28,
            18.0,
            palette.text_secondary,
        );
    }

    fn render_folder_picker(&self, canvas: &mut Canvas, palette: &ThemePalette) {
        // Header
        let cur_path_str = format!("选择书库目录: {}", self.browse_path.to_string_lossy());
        Widgets::draw_header(canvas, &cur_path_str, palette);

        // Center card container
        let card_w = 900;
        let card_h = 600;
        let card_x = (SCREEN_WIDTH as i32 - card_w) / 2;
        let card_y = 65;

        canvas.draw_rounded_rect_alpha(card_x + 4, card_y + 6, card_w, card_h, 16, 0x000000, 50);
        canvas.draw_rounded_rect(card_x, card_y, card_w, card_h, 16, palette.card_bg);
        canvas.draw_rounded_border(card_x, card_y, card_w, card_h, 16, palette.accent, 2);

        // Current Directory Badge
        canvas.draw_rounded_rect(card_x + 24, card_y + 16, card_w - 48, 40, 8, palette.background);
        let path_label = format!("📁 当前浏览路径: {}", self.browse_path.display());
        canvas.draw_text(&path_label, card_x + 36, card_y + 24, 20.0, palette.text_primary);

        // Folder list (Max 7 visible at once)
        let visible_count = 7;
        let start_idx = if self.folder_selected_index >= visible_count {
            self.folder_selected_index - visible_count + 1
        } else {
            0
        };
        let end_idx = (start_idx + visible_count).min(self.folder_entries.len());

        let item_h = 58;
        let item_start_y = card_y + 70;

        if self.folder_entries.is_empty() {
            let msg = "(当前目录下没有其他子文件夹)";
            let mw = canvas.measure_text_width(msg, 22.0);
            canvas.draw_text(msg, (SCREEN_WIDTH as i32 - mw) / 2, card_y + 240, 22.0, palette.text_muted);
        } else {
            for (display_idx, idx) in (start_idx..end_idx).enumerate() {
                let iy = item_start_y + (display_idx as i32) * item_h;
                let ix = card_x + 24;
                let iw = card_w - 48;
                let is_selected = idx == self.folder_selected_index;

                let entry = &self.folder_entries[idx];
                let is_parent = if let Some(parent) = self.browse_path.parent() {
                    entry == parent
                } else {
                    false
                };

                let folder_name = if is_parent {
                    "📁 [ .. 上级目录 ]".to_string()
                } else {
                    format!(
                        "📁 {}",
                        entry.file_name().and_then(|s| s.to_str()).unwrap_or("文件夹")
                    )
                };

                if is_selected {
                    canvas.draw_rounded_rect(ix, iy, iw, item_h - 8, 8, palette.card_selected_bg);
                    canvas.draw_rounded_border(ix, iy, iw, item_h - 8, 8, palette.accent, 2);
                } else {
                    canvas.draw_rounded_rect_alpha(ix, iy, iw, item_h - 8, 8, palette.background, 100);
                    canvas.draw_rounded_border(ix, iy, iw, item_h - 8, 8, palette.border, 1);
                }

                let text_col = if is_selected { palette.accent } else { palette.text_primary };
                canvas.draw_text(&folder_name, ix + 20, iy + 14, 22.0, text_col);
            }
        }

        // Bottom Action Hints
        let tip_text = "(A) 进入该文件夹    (X / Start) 选择当前目录为书库    (B) 取消返回";
        let tw = canvas.measure_text_width(tip_text, 18.0);
        canvas.draw_text(
            tip_text,
            (SCREEN_WIDTH as i32 - tw) / 2,
            (SCREEN_HEIGHT as i32) - 28,
            18.0,
            palette.text_secondary,
        );
    }
}
