use crate::core::text_engine::TextBook;
use crate::platform::battery::BatteryStatus;
use crate::ui::canvas::Canvas;
use crate::ui::theme::ThemePalette;
use crate::ui::widgets::Widgets;

pub struct TextView {
    pub show_menu: bool,
    pub menu_selection: usize,
    pub show_chapter_list: bool,
    pub chapter_selection: usize,
}

impl TextView {
    pub fn new() -> Self {
        Self {
            show_menu: false,
            menu_selection: 0,
            show_chapter_list: false,
            chapter_selection: 0,
        }
    }

    pub fn open_chapter_list(&mut self, book: &TextBook) {
        self.show_menu = false;
        self.show_chapter_list = true;
        self.chapter_selection = book.current_chapter_index().unwrap_or(0);
        if self.chapter_selection >= book.chapters().len() {
            self.chapter_selection = book.chapters().len().saturating_sub(1);
        }
    }

    pub fn render(
        &self,
        canvas: &mut Canvas,
        book: &TextBook,
        palette: &ThemePalette,
        battery: Option<BatteryStatus>,
    ) {
        canvas.clear(palette.background);

        let total_pages = book.total_pages();
        let cur_page = book.current_page();
        let percent = if total_pages > 0 {
            ((cur_page + 1) as f32) / (total_pages as f32)
        } else {
            0.0
        };

        // Persistent book title and real device status.
        let chapter_title = book.current_chapter_name();
        Widgets::draw_header(canvas, &book.title, palette, battery);

        // Dedicated chapter strip keeps the chapter name visible even when the
        // book title is long or the footer is crowded.
        let chapter_position = book
            .current_chapter_index()
            .map(|index| format!("第 {}/{} 章", index + 1, book.chapters().len()))
            .unwrap_or_else(|| "正文".to_string());
        let chapter_position_w = canvas.measure_text_width(&chapter_position, 17.0);
        let chapter_text_max = canvas.width as i32 - chapter_position_w - 120;
        let chapter_label = format!("当前章节 · {}", chapter_title);
        let chapter_label = Widgets::fit_text(canvas, &chapter_label, chapter_text_max, 18.0);
        canvas.draw_rounded_rect_alpha(
            24,
            52,
            canvas.width as i32 - 48,
            32,
            8,
            palette.card_bg,
            210,
        );
        canvas.draw_text(&chapter_label, 38, 58, 18.0, palette.text_secondary);
        canvas.draw_text(
            &chapter_position,
            canvas.width as i32 - chapter_position_w - 38,
            59,
            17.0,
            palette.accent,
        );

        // Render Text Body
        if let Some(lines) = book.current_page_lines() {
            let start_x = book.settings.margin_x;
            let mut start_y = (book.settings.margin_y + 40).max(96);
            let line_height = (book.settings.font_size * book.settings.line_spacing) as i32;

            for line in lines {
                if !line.is_empty() {
                    canvas.draw_text(
                        line,
                        start_x,
                        start_y,
                        book.settings.font_size,
                        palette.text_primary,
                    );
                }
                start_y += line_height;
            }
        }

        // Bottom Progress Footer
        let progress_text = format!(
            "第 {} / {} 页  ·  {:.0}%",
            cur_page + 1,
            total_pages,
            percent * 100.0
        );
        let encoding_info = format!(
            "{}  ·  字号 {:.0}  ·  {}",
            chapter_title, book.settings.font_size, book.encoding_name
        );
        Widgets::draw_footer_status(canvas, &encoding_info, &progress_text, percent, palette);

        // Floating Bottom Menu on Menu Key Press
        if self.show_menu {
            let menu_items = [
                ("L2/R2", "字号大小"),
                ("Y", "阅读主题"),
                ("SEL/ST", "章节目录"),
                ("B", "返回书架"),
            ];
            Widgets::draw_floating_menu(canvas, &menu_items, self.menu_selection, palette);
        }

        if self.show_chapter_list {
            self.draw_chapter_list(canvas, book, palette);
        }
    }

    fn draw_chapter_list(&self, canvas: &mut Canvas, book: &TextBook, palette: &ThemePalette) {
        let card_x = 82;
        let card_y = 66;
        let card_w = canvas.width as i32 - card_x * 2;
        let card_h = 626;

        canvas.draw_rect_alpha(
            0,
            44,
            canvas.width as i32,
            canvas.height as i32 - 44,
            0x000000,
            105,
        );
        canvas.draw_rounded_rect_alpha(card_x + 4, card_y + 6, card_w, card_h, 16, 0x000000, 80);
        canvas.draw_rounded_rect(card_x, card_y, card_w, card_h, 16, palette.card_bg);
        canvas.draw_rounded_border(card_x, card_y, card_w, card_h, 16, palette.accent, 2);

        let chapters = book.chapters();
        let heading = format!("章节目录  ·  共 {} 章", chapters.len());
        canvas.draw_text(
            &heading,
            card_x + 28,
            card_y + 20,
            25.0,
            palette.text_primary,
        );
        canvas.draw_text(
            "↑↓ 选择   A 跳转   B / SELECT / START 关闭",
            card_x + 28,
            card_y + 55,
            17.0,
            palette.text_secondary,
        );
        canvas.draw_horizontal_line(
            card_x + 24,
            card_x + card_w - 24,
            card_y + 88,
            palette.border,
        );

        if chapters.is_empty() {
            let message = "未识别到章节标题，可继续使用 L1 / R1 翻页。";
            let message_w = canvas.measure_text_width(message, 22.0);
            canvas.draw_text(
                message,
                card_x + (card_w - message_w) / 2,
                card_y + card_h / 2,
                22.0,
                palette.text_secondary,
            );
            return;
        }

        let visible_count = 9;
        let selected = self.chapter_selection.min(chapters.len() - 1);
        let start_index = if selected >= visible_count {
            selected - visible_count + 1
        } else {
            0
        };
        let end_index = (start_index + visible_count).min(chapters.len());
        let current_index = book.current_chapter_index();
        let item_x = card_x + 24;
        let item_w = card_w - 48;
        let item_h = 54;
        let item_start_y = card_y + 104;

        for (row, chapter_index) in (start_index..end_index).enumerate() {
            let chapter = &chapters[chapter_index];
            let item_y = item_start_y + row as i32 * item_h;
            let is_selected = chapter_index == selected;
            let is_current = current_index == Some(chapter_index);

            if is_selected {
                canvas.draw_rounded_rect(
                    item_x,
                    item_y,
                    item_w,
                    item_h - 6,
                    8,
                    palette.card_selected_bg,
                );
                canvas.draw_rounded_border(
                    item_x,
                    item_y,
                    item_w,
                    item_h - 6,
                    8,
                    palette.accent,
                    2,
                );
            } else if is_current {
                canvas.draw_rounded_rect_alpha(
                    item_x,
                    item_y,
                    item_w,
                    item_h - 6,
                    8,
                    palette.background,
                    150,
                );
            }

            let prefix = if is_current {
                format!("当前  {:02}", chapter_index + 1)
            } else {
                format!("      {:02}", chapter_index + 1)
            };
            let prefix_color = if is_selected || is_current {
                palette.accent
            } else {
                palette.text_muted
            };
            canvas.draw_text(&prefix, item_x + 16, item_y + 12, 18.0, prefix_color);

            let page_text = format!("第 {} 页", chapter.start_page + 1);
            let page_w = canvas.measure_text_width(&page_text, 17.0);
            let title_x = item_x + 120;
            let title_max_width = item_w - 120 - page_w - 34;
            let title = Widgets::fit_text(canvas, &chapter.title, title_max_width, 20.0);
            let title_color = if is_selected {
                palette.accent
            } else {
                palette.text_primary
            };
            canvas.draw_text(&title, title_x, item_y + 10, 20.0, title_color);
            canvas.draw_text(
                &page_text,
                item_x + item_w - page_w - 16,
                item_y + 12,
                17.0,
                palette.text_secondary,
            );
        }
    }
}
