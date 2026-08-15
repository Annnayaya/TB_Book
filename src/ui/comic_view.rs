use crate::core::comic_engine::{ComicArchive, ReadingDirection};
use crate::ui::canvas::{Canvas, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::ui::theme::ThemePalette;
use crate::ui::widgets::Widgets;

pub struct ComicView {
    pub show_menu: bool,
    pub menu_selection: usize,
}

impl ComicView {
    pub fn new() -> Self {
        Self {
            show_menu: false,
            menu_selection: 0,
        }
    }

    pub fn render(&self, canvas: &mut Canvas, comic: &ComicArchive, palette: &ThemePalette) {
        canvas.clear(palette.background);

        let sw = SCREEN_WIDTH as f32;
        let sh = SCREEN_HEIGHT as f32;

        if let Some(img) = &comic.current_image {
            let iw = img.width() as f32;
            let ih = img.height() as f32;

            // Calculate base scale to fit 4:3 screen
            let base_scale = (sw / iw).min(sh / ih);
            let cur_scale = base_scale * comic.zoom_level;

            let view_w = (sw / cur_scale).min(iw);
            let view_h = (sh / cur_scale).min(ih);

            let dst_w = (view_w * cur_scale) as i32;
            let dst_h = (view_h * cur_scale) as i32;

            let dst_x = ((sw - dst_w as f32) / 2.0) as i32;
            let dst_y = ((sh - dst_h as f32) / 2.0) as i32;

            canvas.draw_image(
                img,
                dst_x,
                dst_y,
                dst_w,
                dst_h,
                comic.pan_x,
                comic.pan_y,
                view_w,
                view_h,
            );

            // Minimap HUD when zoomed in
            if comic.zoom_level > 1.05 && comic.minimap_timer > 0.0 {
                Widgets::draw_minimap(
                    canvas,
                    iw,
                    ih,
                    comic.pan_x,
                    comic.pan_y,
                    view_w,
                    view_h,
                    palette,
                );
            }
        } else {
            let msg = "正在加载漫画图片...";
            let mw = canvas.measure_text_width(msg, 24.0);
            canvas.draw_text(
                msg,
                (SCREEN_WIDTH as i32 - mw) / 2,
                (SCREEN_HEIGHT as i32) / 2,
                24.0,
                palette.text_primary,
            );
        }

        // Subdued Top Header
        let total_pages = comic.page_entries.len();
        let cur_page = comic.current_page;
        let dir_str = match comic.reading_direction {
            ReadingDirection::RightToLeft => "日漫 [RTL]",
            ReadingDirection::LeftToRight => "国漫 [LTR]",
        };
        let crop_str = if comic.auto_crop { " | 切白边开启" } else { "" };
        let header_title = format!("{} (第 {}/{} 页)  [{}{}]", comic.title, cur_page + 1, total_pages, dir_str, crop_str);
        Widgets::draw_header(canvas, &header_title, palette);

        // Subdued Bottom Status Bar
        let percent = if total_pages > 0 {
            ((cur_page + 1) as f32) / (total_pages as f32)
        } else {
            0.0
        };
        let zoom_info = if comic.zoom_level > 1.0 {
            format!("🔍 缩放: {:.1}x | [十字键] 漫游移动 | [X] 还原全屏", comic.zoom_level)
        } else {
            format!("🔍 1.0x 全屏 | [R2/L2] 缩放 | [L1/R1] 翻页")
        };
        let prog_text = format!("{}/{} ({:.0}%)", cur_page + 1, total_pages, percent * 100.0);
        Widgets::draw_footer_status(canvas, &zoom_info, &prog_text, percent, palette);

        // Floating Bottom Menu on Menu Key Press
        if self.show_menu {
            let menu_items = [
                ("L2/R2", "平滑缩放"),
                ("X", "还原全屏"),
                ("Y", "阅读方向"),
                ("FN", "智能切边"),
                ("B", "返回书架"),
            ];
            Widgets::draw_floating_menu(canvas, &menu_items, self.menu_selection, palette);
        }
    }
}
