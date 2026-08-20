use crate::core::library::{BookMetadata, BookType};
use crate::platform::battery::BatteryStatus;
use crate::ui::canvas::{Canvas, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::ui::theme::ThemePalette;
use crate::ui::widgets::Widgets;
use std::path::Path;

pub struct ShelfView {
    pub selected_index: usize,
}

impl ShelfView {
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    pub fn render(
        &self,
        canvas: &mut Canvas,
        books: &[BookMetadata],
        library_path: &Path,
        palette: &ThemePalette,
        battery: Option<BatteryStatus>,
    ) {
        canvas.clear(palette.background);

        // Header with current library folder
        let header_text = format!("我的书架 [目录: {}]", library_path.display());
        Widgets::draw_header(canvas, &header_text, palette, battery);

        if books.is_empty() {
            let msg = "当前书库目录为空，按 [Menu] 键可更换书库目录";
            let mw = canvas.measure_text_width(msg, 24.0);
            canvas.draw_text(
                msg,
                (SCREEN_WIDTH as i32 - mw) / 2,
                (SCREEN_HEIGHT as i32) / 2 - 20,
                24.0,
                palette.text_secondary,
            );
            return;
        }

        // 3x2 Grid Cards
        let cols = 3;
        let rows = 2;
        let card_w = 295;
        let card_h = 280;
        let start_x = 42;
        let start_y = 65;
        let gap_x = 24;
        let gap_y = 20;

        let visible_count = cols * rows;
        let start_idx = (self.selected_index / visible_count) * visible_count;
        let end_idx = (start_idx + visible_count).min(books.len());

        for (idx, book) in books.iter().enumerate().take(end_idx).skip(start_idx) {
            let grid_pos = idx - start_idx;
            let col = grid_pos % cols;
            let row = grid_pos / cols;

            let cx = start_x + (col as i32) * (card_w + gap_x);
            let cy = start_y + (row as i32) * (card_h + gap_y);
            let is_selected = idx == self.selected_index;

            // Card background & shadow
            if is_selected {
                canvas.draw_rounded_rect_alpha(cx + 3, cy + 5, card_w, card_h, 14, 0x000000, 60);
                canvas.draw_rounded_rect(cx, cy, card_w, card_h, 14, palette.card_selected_bg);
                canvas.draw_rounded_border(cx, cy, card_w, card_h, 14, palette.accent, 3);
            } else {
                canvas.draw_rounded_rect_alpha(cx + 2, cy + 3, card_w, card_h, 14, 0x000000, 30);
                canvas.draw_rounded_rect(cx, cy, card_w, card_h, 14, palette.card_bg);
                canvas.draw_rounded_border(cx, cy, card_w, card_h, 14, palette.border, 1);
            }

            // Cover area (Book cover placeholder or visual pattern)
            let cover_w = card_w - 24;
            let cover_h = 160;
            let cover_x = cx + 12;
            let cover_y = cy + 12;

            let cover_bg = match book.book_type {
                BookType::Comic => 0x2A344A,
                BookType::Text => 0x3D332A,
                BookType::Unknown => 0x333333,
            };
            canvas.draw_rounded_rect(cover_x, cover_y, cover_w, cover_h, 8, cover_bg);

            // Format Badge (e.g. TXT / CBZ)
            let text_format = book
                .path
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or("TXT")
                .to_ascii_uppercase();
            let (badge_text, badge_color) = match book.book_type {
                BookType::Comic => ("漫画 CBZ", 0x3B82F6),
                BookType::Text => (
                    if text_format == "EPUB" {
                        "电子书 EPUB"
                    } else if text_format == "MD" {
                        "文本 MD"
                    } else {
                        "小说 TXT"
                    },
                    0xF59E0B,
                ),
                BookType::Unknown => ("其他", 0x6B7280),
            };
            canvas.draw_rounded_rect(cover_x + 8, cover_y + 8, 76, 24, 6, badge_color);
            canvas.draw_text(badge_text, cover_x + 14, cover_y + 11, 16.0, 0xFFFFFF);

            // Inner title on cover
            let cover_title: String = book.title.chars().take(8).collect();
            canvas.draw_text(&cover_title, cover_x + 16, cover_y + 65, 26.0, 0xFFFFFF);

            // Book title below cover
            let display_title: String = book.title.chars().take(12).collect();
            let title_color = if is_selected {
                palette.accent
            } else {
                palette.text_primary
            };
            canvas.draw_text(&display_title, cx + 14, cy + 182, 22.0, title_color);

            // Size / Pages Info
            let size_kb = book.file_size_bytes / 1024;
            let info_text = if size_kb > 1024 {
                format!("{:.1} MB", (size_kb as f32) / 1024.0)
            } else {
                format!("{} KB", size_kb)
            };
            canvas.draw_text(&info_text, cx + 14, cy + 215, 16.0, palette.text_secondary);

            // Progress Bar
            let bar_w = card_w - 28;
            let bar_x = cx + 14;
            let bar_y = cy + 245;
            canvas.draw_rounded_rect(bar_x, bar_y, bar_w, 8, 4, palette.background);
            let fill_w = ((bar_w as f32) * book.percent.clamp(0.0, 1.0)) as i32;
            if fill_w > 0 {
                canvas.draw_rounded_rect(bar_x, bar_y, fill_w, 8, 4, palette.accent);
            }

            let prog_pct = format!("进度: {:.0}%", book.percent * 100.0);
            let prog_w = canvas.measure_text_width(&prog_pct, 16.0);
            canvas.draw_text(
                &prog_pct,
                cx + card_w - prog_w - 14,
                cy + 215,
                16.0,
                palette.text_muted,
            );
        }

        // Bottom Navigation Tips
        let nav_tips =
            "(A) 开始阅读    (Menu) 设置中心(换目录/改字号)    (Y) 切换主题    (FN) 刷新书架";
        let tw = canvas.measure_text_width(nav_tips, 18.0);
        let tip_x = (SCREEN_WIDTH as i32 - tw) / 2;
        let tip_y = (SCREEN_HEIGHT as i32) - 28;
        canvas.draw_text(nav_tips, tip_x, tip_y, 18.0, palette.text_secondary);
    }
}
