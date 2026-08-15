use crate::ui::canvas::Canvas;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub title: String,
    pub char_offset: usize,
    pub page_index: usize,
}

#[derive(Debug, Clone)]
pub struct Page {
    pub lines: Vec<String>,
    pub start_offset: usize,
    pub end_offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypographySettings {
    pub font_size: f32,
    pub line_spacing: f32,
    pub margin_x: i32,
    pub margin_y: i32,
    pub indent_spaces: usize,
}

impl Default for TypographySettings {
    fn default() -> Self {
        Self {
            font_size: 28.0,
            line_spacing: 1.5,
            margin_x: 40,
            margin_y: 50,
            indent_spaces: 2,
        }
    }
}

pub struct TextBook {
    pub title: String,
    pub raw_text: String,
    pub encoding_name: String,
    pub chapters: Vec<Chapter>,
    pub pages: Vec<Page>,
    pub current_page: usize,
    pub settings: TypographySettings,
}

// CJK Punctuation Avoidance Sets (避头尾规则)
const NO_LINE_START: &[char] = &[
    '，', '。', '！', '？', '；', '：', '、', '）', '】', '》', '”', '’', '…', '—', '·',
    ',', '.', '!', '?', ';', ':', ')', ']', '}', '>',
];

const NO_LINE_END: &[char] = &[
    '（', '【', '《', '“', '‘', '(', '[', '{', '<',
];

impl TextBook {
    pub fn from_string(title: String, text: String, encoding_name: String) -> Self {
        let mut book = Self {
            title,
            raw_text: text,
            encoding_name,
            chapters: Vec::new(),
            pages: Vec::new(),
            current_page: 0,
            settings: TypographySettings::default(),
        };

        book.detect_chapters();
        book
    }

    pub fn repaginate(&mut self, canvas: &Canvas) {
        let content_width = (canvas.width as i32) - self.settings.margin_x * 2;
        let content_height = (canvas.height as i32) - self.settings.margin_y * 2 - 40; // reserve for top & bottom HUD

        if content_width <= 50 || content_height <= 50 {
            return;
        }

        let line_height = (self.settings.font_size * self.settings.line_spacing) as i32;
        let max_lines_per_page = (content_height / line_height).max(1) as usize;

        let mut pages = Vec::new();
        let mut current_page_lines = Vec::new();
        let mut page_start_offset = 0;
        let mut current_offset = 0;

        let paragraphs = self.raw_text.split('\n');

        for paragraph in paragraphs {
            let trimmed = paragraph.trim();
            if trimmed.is_empty() {
                // Empty line
                if !current_page_lines.is_empty() {
                    current_page_lines.push(String::new());
                    if current_page_lines.len() >= max_lines_per_page {
                        pages.push(Page {
                            lines: std::mem::take(&mut current_page_lines),
                            start_offset: page_start_offset,
                            end_offset: current_offset,
                        });
                        page_start_offset = current_offset;
                    }
                }
                current_offset += 1;
                continue;
            }

            // Apply 2-em indentation for standard Chinese paragraphs
            let indent = "　".repeat(self.settings.indent_spaces);
            let formatted_para = format!("{}{}", indent, trimmed);

            let wrapped_lines = wrap_chinese_text(&formatted_para, content_width, self.settings.font_size, canvas);

            for line in wrapped_lines {
                current_page_lines.push(line);
                if current_page_lines.len() >= max_lines_per_page {
                    pages.push(Page {
                        lines: std::mem::take(&mut current_page_lines),
                        start_offset: page_start_offset,
                        end_offset: current_offset,
                    });
                    page_start_offset = current_offset;
                }
            }
            current_offset += paragraph.len() + 1;
        }

        if !current_page_lines.is_empty() {
            pages.push(Page {
                lines: current_page_lines,
                start_offset: page_start_offset,
                end_offset: self.raw_text.len(),
            });
        }

        if pages.is_empty() {
            pages.push(Page {
                lines: vec!["(暂无内容)".to_string()],
                start_offset: 0,
                end_offset: 0,
            });
        }

        self.pages = pages;
        if self.current_page >= self.pages.len() {
            self.current_page = self.pages.len().saturating_sub(1);
        }

        self.update_chapter_page_indices();
    }

    fn detect_chapters(&mut self) {
        self.chapters.clear();
        let mut offset = 0;

        for line in self.raw_text.lines() {
            let trimmed = line.trim();
            // Regex-like pattern for Chinese Chapter headings
            if is_chapter_title(trimmed) {
                self.chapters.push(Chapter {
                    title: trimmed.chars().take(30).collect(),
                    char_offset: offset,
                    page_index: 0,
                });
            }
            offset += line.len() + 1;
        }
    }

    fn update_chapter_page_indices(&mut self) {
        for chapter in &mut self.chapters {
            for (idx, page) in self.pages.iter().enumerate() {
                if chapter.char_offset >= page.start_offset && chapter.char_offset <= page.end_offset {
                    chapter.page_index = idx;
                    break;
                }
            }
        }
    }

    pub fn next_page(&mut self) -> bool {
        if self.current_page + 1 < self.pages.len() {
            self.current_page += 1;
            true
        } else {
            false
        }
    }

    pub fn prev_page(&mut self) -> bool {
        if self.current_page > 0 {
            self.current_page -= 1;
            true
        } else {
            false
        }
    }

    pub fn jump_to_page(&mut self, page: usize) {
        if !self.pages.is_empty() {
            self.current_page = page.min(self.pages.len() - 1);
        }
    }

    pub fn jump_percent(&mut self, percent: f32) {
        if !self.pages.is_empty() {
            let target = ((self.pages.len() as f32) * percent).round() as usize;
            self.jump_to_page(target);
        }
    }

    pub fn current_chapter_name(&self) -> &str {
        let mut last_title = "正文";
        for ch in &self.chapters {
            if ch.page_index <= self.current_page {
                last_title = &ch.title;
            } else {
                break;
            }
        }
        last_title
    }
}

/// Chinese & Western text wrapping algorithm enforcing CJK Punctuation Avoidance (避头尾规则)
fn wrap_chinese_text(text: &str, max_width: i32, font_size: f32, canvas: &Canvas) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        let mut test_line = current_line.clone();
        test_line.push(ch);

        let w = canvas.measure_text_width(&test_line, font_size);

        if w <= max_width {
            current_line.push(ch);
            i += 1;
        } else {
            // Needs wrap, apply CJK 避头尾规则
            if !current_line.is_empty() {
                // If next character is prohibited from starting a line (e.g. comma, period, closing bracket)
                if NO_LINE_START.contains(&ch) {
                    // Pull back one character from current_line to next line so punctuation stays attached
                    if let Some(prev_ch) = current_line.pop() {
                        lines.push(current_line);
                        current_line = String::new();
                        current_line.push(prev_ch);
                        current_line.push(ch);
                        i += 1;
                        continue;
                    }
                }

                // If current_line ends with prohibited end-char (e.g. opening bracket), move it
                if let Some(&last_ch) = current_line.chars().last().as_ref() {
                    if NO_LINE_END.contains(&last_ch) {
                        current_line.pop();
                        lines.push(current_line);
                        current_line = String::new();
                        current_line.push(last_ch);
                        current_line.push(ch);
                        i += 1;
                        continue;
                    }
                }

                lines.push(current_line);
                current_line = String::new();
                current_line.push(ch);
                i += 1;
            } else {
                // Single character exceeds max width
                current_line.push(ch);
                lines.push(current_line);
                current_line = String::new();
                i += 1;
            }
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

fn is_chapter_title(s: &str) -> bool {
    if s.len() > 60 {
        return false;
    }
    (s.starts_with('第') && (s.contains('章') || s.contains('节') || s.contains('回') || s.contains('卷') || s.contains('幕')))
        || s.starts_with("Chapter ")
        || s.starts_with("CHAPTER ")
        || s.starts_with("Prologue")
        || s.starts_with("Epilogue")
        || s.starts_with("楔子")
        || s.starts_with("后记")
}
