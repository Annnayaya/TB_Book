use crate::core::text_engine::TextBook;
use crate::ui::canvas::Canvas;
use crate::ui::theme::ThemePalette;
use crate::ui::widgets::Widgets;

pub struct TextView {
    pub show_menu: bool,
    pub menu_selection: usize,
}

impl TextView {
    pub fn new() -> Self {
        Self {
            show_menu: false,
            menu_selection: 0,
        }
    }

    pub fn render(&self, canvas: &mut Canvas, book: &TextBook, palette: &ThemePalette) {
        canvas.clear(palette.background);

        let total_pages = book.pages.len();
        let cur_page = book.current_page;
        let percent = if total_pages > 0 {
            ((cur_page + 1) as f32) / (total_pages as f32)
        } else {
            0.0
        };

        // Top Status Header
        let chapter_title = book.current_chapter_name();
        let header_title = format!("{}  ·  {}", book.title, chapter_title);
        Widgets::draw_header(canvas, &header_title, palette);

        // Render Text Body
        if let Some(page) = book.pages.get(cur_page) {
            let start_x = book.settings.margin_x;
            let mut start_y = book.settings.margin_y + 40;
            let line_height = (book.settings.font_size * book.settings.line_spacing) as i32;

            for line in &page.lines {
                if !line.is_empty() {
                    canvas.draw_text(line, start_x, start_y, book.settings.font_size, palette.text_primary);
                }
                start_y += line_height;
            }
        }

        // Bottom Progress Footer
        let progress_text = format!("{}/{} 页 ({:.0}%)", cur_page + 1, total_pages, percent * 100.0);
        let encoding_info = format!("字号: {:.0}px | 编码: {}", book.settings.font_size, book.encoding_name);
        Widgets::draw_footer_status(canvas, &encoding_info, &progress_text, percent, palette);

        // Floating Bottom Menu on Menu Key Press
        if self.show_menu {
            let menu_items = [
                ("L2/R2", "字号大小"),
                ("Y", "阅读主题"),
                ("Sel", "章节目录"),
                ("B", "返回书架"),
            ];
            Widgets::draw_floating_menu(canvas, &menu_items, self.menu_selection, palette);
        }
    }
}
