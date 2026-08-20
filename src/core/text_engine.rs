use crate::core::cache::{BookIndexCache, ChapterIndex};
use crate::core::charset::CharsetHelper;
use crate::core::epub_engine::EpubDocument;
use crate::ui::canvas::Canvas;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Page {
    pub lines: Vec<String>,
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

impl TypographySettings {
    pub fn is_compatible_with(&self, other: &TypographySettings) -> bool {
        (self.font_size - other.font_size).abs() < 0.01
            && (self.line_spacing - other.line_spacing).abs() < 0.01
            && self.margin_x == other.margin_x
            && self.margin_y == other.margin_y
            && self.indent_spaces == other.indent_spaces
    }
}

pub struct TextBook {
    pub title: String,
    pub path: PathBuf,
    pub is_epub: bool,
    pub encoding_name: String,
    pub cache: BookIndexCache,
    pub current_chapter_index: usize,
    pub current_chapter_pages: Vec<Page>,
    pub current_page_in_chapter: usize,
    pub current_global_page: usize,
    pub settings: TypographySettings,
    in_memory_text: Option<String>,
}

// CJK Punctuation Avoidance Sets (避头尾规则)
const NO_LINE_START: &[char] = &[
    '，', '。', '！', '？', '；', '：', '、', '）', '】', '》', '”', '’', '…', '—', '·', ',', '.',
    '!', '?', ';', ':', ')', ']', '}', '>',
];

const NO_LINE_END: &[char] = &['（', '【', '《', '“', '‘', '(', '[', '{', '<'];

impl TextBook {
    /// Open a book (TXT/MD/EPUB) with instant cache hit or build index on first load.
    pub fn open<P: AsRef<Path>>(
        path: P,
        title: String,
        settings: TypographySettings,
        canvas: &Canvas,
    ) -> Result<Self, String> {
        let path_buf = path.as_ref().to_path_buf();
        let (file_size, mtime) = BookIndexCache::get_file_metadata(&path_buf);
        if file_size == 0 {
            return Err("文件为空或不存在".to_string());
        }

        let is_epub = path_buf
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("epub"));

        // 1. Try loading from cache
        if let Some(cache) = BookIndexCache::load_if_valid(&path_buf, file_size, mtime, &settings) {
            println!(
                "✓ [Cache HIT] 命中排版索引缓存: {} (共 {} 章, {} 页)",
                title,
                cache.chapters.len(),
                cache.total_pages
            );
            let mut book = Self {
                title,
                path: path_buf,
                is_epub,
                encoding_name: cache.encoding_name.clone(),
                cache,
                current_chapter_index: 0,
                current_chapter_pages: Vec::new(),
                current_page_in_chapter: 0,
                current_global_page: 0,
                settings,
                in_memory_text: None,
            };
            book.load_chapter(0, canvas);
            return Ok(book);
        }

        // 2. Cache MISS: Build index
        println!(
            "==> [Cache MISS] 首次加载/排版变更，正在建立章节与全局分页索引: {} ...",
            title
        );

        let cache = if is_epub {
            Self::build_epub_index(&path_buf, file_size, mtime, &settings, canvas)?
        } else {
            Self::build_txt_index(&path_buf, file_size, mtime, &settings, canvas)?
        };

        let _ = cache.save(&path_buf);

        let mut book = Self {
            title,
            path: path_buf,
            is_epub,
            encoding_name: cache.encoding_name.clone(),
            cache,
            current_chapter_index: 0,
            current_chapter_pages: Vec::new(),
            current_page_in_chapter: 0,
            current_global_page: 0,
            settings,
            in_memory_text: None,
        };
        book.load_chapter(0, canvas);
        Ok(book)
    }

    /// For unit tests or in-memory books
    pub fn from_string(title: String, text: String, encoding_name: String) -> Self {
        let settings = TypographySettings::default();
        let canvas = Canvas::new();

        let mut chapters = Vec::new();
        let mut chapter_slices = Vec::new();
        let mut current_title = "正文".to_string();
        let mut current_text = String::new();

        for line in text.lines() {
            let trimmed = line.trim();
            if is_chapter_title(trimmed) {
                if !current_text.is_empty() {
                    chapter_slices.push((current_title.clone(), std::mem::take(&mut current_text)));
                }
                current_title = trimmed.chars().take(40).collect();
            }
            current_text.push_str(line);
            current_text.push('\n');
        }
        if !current_text.is_empty() {
            chapter_slices.push((current_title, current_text));
        }
        if chapter_slices.is_empty() {
            chapter_slices.push(("正文".to_string(), text.clone()));
        }

        let mut total_pages = 0;
        for (i, (ch_title, ch_text)) in chapter_slices.iter().enumerate() {
            let pages = paginate_text(ch_text, &settings, &canvas);
            let page_count = pages.len().max(1);
            chapters.push(ChapterIndex {
                index: i,
                title: ch_title.clone(),
                byte_offset: 0,
                byte_length: ch_text.len() as u64,
                epub_href: None,
                start_page: total_pages,
                page_count,
            });
            total_pages += page_count;
        }

        let cache = BookIndexCache {
            file_path: "memory".to_string(),
            file_size: text.len() as u64,
            mtime: 0,
            encoding_name: encoding_name.clone(),
            typography: settings.clone(),
            total_pages,
            total_chars: text.chars().count(),
            chapters,
        };

        let initial_pages = if let Some((_, first_text)) = chapter_slices.first() {
            paginate_text(first_text, &settings, &canvas)
        } else {
            vec![Page { lines: vec!["(暂无内容)".to_string()] }]
        };

        Self {
            title,
            path: PathBuf::from("memory"),
            is_epub: false,
            encoding_name,
            cache,
            current_chapter_index: 0,
            current_chapter_pages: initial_pages,
            current_page_in_chapter: 0,
            current_global_page: 0,
            settings,
            in_memory_text: Some(text),
        }
    }

    fn build_txt_index(
        path: &Path,
        file_size: u64,
        mtime: u64,
        settings: &TypographySettings,
        canvas: &Canvas,
    ) -> Result<BookIndexCache, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("无法读取文件: {e}"))?;
        let (full_text, encoding) = CharsetHelper::decode_bytes(&bytes);

        let mut chapter_headers: Vec<(String, usize)> = Vec::new();
        let mut cursor = 0;

        while cursor < full_text.len() {
            let next_nl = full_text[cursor..].find('\n').map(|i| cursor + i).unwrap_or(full_text.len());
            let line = &full_text[cursor..next_nl];
            let trimmed = line.trim();
            if is_chapter_title(trimmed) {
                chapter_headers.push((trimmed.chars().take(40).collect(), cursor));
            }
            if next_nl >= full_text.len() {
                break;
            }
            cursor = next_nl + 1;
        }

        // If no chapters detected, chunk into sections of ~30KB so memory is always bounded
        let mut chapter_ranges: Vec<(String, usize, usize)> = Vec::new();
        if chapter_headers.is_empty() {
            let chunk_size = 30 * 1024;
            let mut start = 0;
            let mut part_num = 1;
            while start < full_text.len() {
                let mut end = (start + chunk_size).min(full_text.len());
                if end < full_text.len() {
                    // Find next newline
                    if let Some(nl) = full_text[end..].find('\n') {
                        end += nl + 1;
                    }
                }
                chapter_ranges.push((format!("第 {} 部分", part_num), start, end));
                start = end;
                part_num += 1;
            }
        } else {
            // First section before chapter 1 (if any prologue)
            if chapter_headers[0].1 > 0 {
                let intro_text = full_text[0..chapter_headers[0].1].trim();
                if !intro_text.is_empty() {
                    chapter_ranges.push(("前言 / 序".to_string(), 0, chapter_headers[0].1));
                }
            }
            for i in 0..chapter_headers.len() {
                let start = chapter_headers[i].1;
                let end = if i + 1 < chapter_headers.len() {
                    chapter_headers[i + 1].1
                } else {
                    full_text.len()
                };
                chapter_ranges.push((chapter_headers[i].0.clone(), start, end));
            }
        }

        let is_utf8 = encoding == "UTF-8";
        let mut chapters = Vec::new();
        let mut total_pages = 0;

        for (idx, (ch_title, start_byte_in_utf8, end_byte_in_utf8)) in chapter_ranges.iter().enumerate() {
            let ch_slice = &full_text[*start_byte_in_utf8..*end_byte_in_utf8];
            let pages = paginate_text(ch_slice, settings, canvas);
            let page_count = pages.len().max(1);

            let (byte_offset, byte_length) = if is_utf8 {
                (*start_byte_in_utf8 as u64, (*end_byte_in_utf8 - *start_byte_in_utf8) as u64)
            } else {
                // For non-UTF8 files, calculate position by char count
                let start_chars = full_text[..*start_byte_in_utf8].chars().count();
                let slice_chars = ch_slice.chars().count();
                let start_orig = find_char_byte_offset(&bytes, start_chars, encoding);
                let end_orig = find_char_byte_offset(&bytes, start_chars + slice_chars, encoding);
                (start_orig as u64, (end_orig - start_orig) as u64)
            };

            chapters.push(ChapterIndex {
                index: idx,
                title: ch_title.clone(),
                byte_offset,
                byte_length,
                epub_href: None,
                start_page: total_pages,
                page_count,
            });
            total_pages += page_count;
        }

        Ok(BookIndexCache {
            file_path: path.to_string_lossy().to_string(),
            file_size,
            mtime,
            encoding_name: encoding.to_string(),
            typography: settings.clone(),
            total_pages,
            total_chars: full_text.chars().count(),
            chapters,
        })
    }

    fn build_epub_index(
        path: &Path,
        file_size: u64,
        mtime: u64,
        settings: &TypographySettings,
        canvas: &Canvas,
    ) -> Result<BookIndexCache, String> {
        let entries = EpubDocument::get_spine_entries(path)?;
        if entries.is_empty() {
            return Err("EPUB 未包含有效内容章节".to_string());
        }

        let mut chapters = Vec::new();
        let mut total_pages = 0;
        let mut total_chars = 0;

        for (idx, entry_path) in entries.iter().enumerate() {
            let ch_text = EpubDocument::read_chapter(path, entry_path).unwrap_or_default();
            let trimmed = ch_text.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Extract chapter title from first line or default
            let first_line = trimmed.lines().next().unwrap_or("").trim();
            let title = if is_chapter_title(first_line) {
                first_line.chars().take(40).collect()
            } else if !first_line.is_empty() && first_line.chars().count() <= 30 {
                first_line.to_string()
            } else {
                format!("第 {} 章", idx + 1)
            };

            let pages = paginate_text(trimmed, settings, canvas);
            let page_count = pages.len().max(1);
            total_chars += trimmed.chars().count();

            chapters.push(ChapterIndex {
                index: chapters.len(),
                title,
                byte_offset: 0,
                byte_length: 0,
                epub_href: Some(entry_path.clone()),
                start_page: total_pages,
                page_count,
            });
            total_pages += page_count;
        }

        if chapters.is_empty() {
            return Err("EPUB 内无可渲染正文".to_string());
        }

        Ok(BookIndexCache {
            file_path: path.to_string_lossy().to_string(),
            file_size,
            mtime,
            encoding_name: "EPUB (UTF-8)".to_string(),
            typography: settings.clone(),
            total_pages,
            total_chars,
            chapters,
        })
    }

    /// Load only the active chapter from disk into memory.
    pub fn load_chapter(&mut self, chapter_index: usize, canvas: &Canvas) {
        if self.cache.chapters.is_empty() {
            self.current_chapter_pages = vec![Page { lines: vec!["(暂无内容)".to_string()] }];
            return;
        }

        let idx = chapter_index.min(self.cache.chapters.len() - 1);
        let chapter = &self.cache.chapters[idx];

        let text = if let Some(mem) = &self.in_memory_text {
            mem.clone()
        } else if self.is_epub {
            if let Some(href) = &chapter.epub_href {
                EpubDocument::read_chapter(&self.path, href).unwrap_or_default()
            } else {
                String::new()
            }
        } else {
            // Read slice from TXT file
            if let Ok(mut file) = File::open(&self.path) {
                if file.seek(SeekFrom::Start(chapter.byte_offset)).is_ok() {
                    let mut buf = vec![0u8; chapter.byte_length as usize];
                    if file.read_exact(&mut buf).is_ok() {
                        let (decoded, _) = CharsetHelper::decode_bytes(&buf);
                        decoded
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        };

        self.current_chapter_pages = paginate_text(&text, &self.settings, canvas);
        if self.current_chapter_pages.is_empty() {
            self.current_chapter_pages.push(Page { lines: vec!["(本章无文字)".to_string()] });
        }
        self.current_chapter_index = idx;
    }

    /// Rebuild entire index cache when user adjusts font size or line spacing.
    pub fn rebuild_cache_and_repaginate(&mut self, canvas: &Canvas) {
        let (file_size, mtime) = BookIndexCache::get_file_metadata(&self.path);
        if self.in_memory_text.is_some() {
            return;
        }

        let new_cache = if self.is_epub {
            Self::build_epub_index(&self.path, file_size, mtime, &self.settings, canvas)
        } else {
            Self::build_txt_index(&self.path, file_size, mtime, &self.settings, canvas)
        };

        if let Ok(cache) = new_cache {
            let _ = cache.save(&self.path);
            self.cache = cache;
            self.load_chapter(self.current_chapter_index, canvas);
            self.current_global_page = self.cache.chapters[self.current_chapter_index].start_page
                + self.current_page_in_chapter.min(self.current_chapter_pages.len().saturating_sub(1));
        }
    }

    pub fn total_pages(&self) -> usize {
        self.cache.total_pages
    }

    pub fn current_page(&self) -> usize {
        self.current_global_page
    }

    pub fn chapters(&self) -> &[ChapterIndex] {
        &self.cache.chapters
    }

    pub fn current_chapter_index(&self) -> Option<usize> {
        Some(self.current_chapter_index)
    }

    pub fn current_chapter_name(&self) -> &str {
        self.cache
            .chapters
            .get(self.current_chapter_index)
            .map(|c| c.title.as_str())
            .unwrap_or("正文")
    }

    pub fn current_page_lines(&self) -> Option<&[String]> {
        self.current_chapter_pages
            .get(self.current_page_in_chapter)
            .map(|p| p.lines.as_slice())
    }

    pub fn next_page(&mut self, canvas: &Canvas) -> bool {
        if self.current_page_in_chapter + 1 < self.current_chapter_pages.len() {
            self.current_page_in_chapter += 1;
            self.current_global_page += 1;
            true
        } else if self.current_chapter_index + 1 < self.cache.chapters.len() {
            self.load_chapter(self.current_chapter_index + 1, canvas);
            self.current_page_in_chapter = 0;
            self.current_global_page = self.cache.chapters[self.current_chapter_index].start_page;
            true
        } else {
            false
        }
    }

    pub fn prev_page(&mut self, canvas: &Canvas) -> bool {
        if self.current_page_in_chapter > 0 {
            self.current_page_in_chapter -= 1;
            self.current_global_page = self.current_global_page.saturating_sub(1);
            true
        } else if self.current_chapter_index > 0 {
            self.load_chapter(self.current_chapter_index - 1, canvas);
            self.current_page_in_chapter = self.current_chapter_pages.len().saturating_sub(1);
            self.current_global_page =
                self.cache.chapters[self.current_chapter_index].start_page + self.current_page_in_chapter;
            true
        } else {
            false
        }
    }

    pub fn jump_to_global_page(&mut self, page: usize, canvas: &Canvas) {
        let target = page.min(self.cache.total_pages.saturating_sub(1));
        let mut target_ch = 0;

        for (i, ch) in self.cache.chapters.iter().enumerate() {
            if target >= ch.start_page && target < ch.start_page + ch.page_count {
                target_ch = i;
                break;
            }
            if target >= ch.start_page {
                target_ch = i;
            }
        }

        if self.current_chapter_index != target_ch || self.current_chapter_pages.is_empty() {
            self.load_chapter(target_ch, canvas);
        }

        let ch = &self.cache.chapters[target_ch];
        self.current_page_in_chapter = target.saturating_sub(ch.start_page).min(self.current_chapter_pages.len().saturating_sub(1));
        self.current_global_page = ch.start_page + self.current_page_in_chapter;
    }

    pub fn jump_percent(&mut self, percent: f32, canvas: &Canvas) {
        if self.cache.total_pages > 0 {
            let last_page = self.cache.total_pages.saturating_sub(1);
            let target = ((last_page as f32) * percent.clamp(0.0, 1.0)).round() as usize;
            self.jump_to_global_page(target, canvas);
        }
    }

    pub fn jump_to_chapter(&mut self, chapter_index: usize, canvas: &Canvas) -> bool {
        if chapter_index < self.cache.chapters.len() {
            self.load_chapter(chapter_index, canvas);
            self.current_page_in_chapter = 0;
            self.current_global_page = self.cache.chapters[chapter_index].start_page;
            true
        } else {
            false
        }
    }
}

/// Paginate a chapter or slice of text into Page objects based on typography settings & canvas.
pub fn paginate_text(text: &str, settings: &TypographySettings, canvas: &Canvas) -> Vec<Page> {
    let content_width = (canvas.width as i32) - settings.margin_x * 2;
    let content_height = (canvas.height as i32) - settings.margin_y * 2 - 40;

    if content_width <= 50 || content_height <= 50 {
        return vec![Page { lines: vec!["(视窗过小)".to_string()] }];
    }

    let line_height = (settings.font_size * settings.line_spacing) as i32;
    let max_lines_per_page = (content_height / line_height).max(1) as usize;

    let mut pages = Vec::new();
    let mut current_page_lines = Vec::new();

    for paragraph in text.split('\n') {
        let trimmed = paragraph.trim();
        if trimmed.is_empty() {
            if !current_page_lines.is_empty() {
                current_page_lines.push(String::new());
                if current_page_lines.len() >= max_lines_per_page {
                    pages.push(Page {
                        lines: std::mem::take(&mut current_page_lines),
                    });
                }
            }
            continue;
        }

        let indent = "　".repeat(settings.indent_spaces);
        let formatted_para = format!("{}{}", indent, trimmed);
        let wrapped = wrap_chinese_text(&formatted_para, content_width, settings.font_size, canvas);

        for line in wrapped {
            current_page_lines.push(line);
            if current_page_lines.len() >= max_lines_per_page {
                pages.push(Page {
                    lines: std::mem::take(&mut current_page_lines),
                });
            }
        }
    }

    if !current_page_lines.is_empty() {
        pages.push(Page {
            lines: current_page_lines,
        });
    }

    if pages.is_empty() {
        pages.push(Page {
            lines: vec!["(暂无内容)".to_string()],
        });
    }

    pages
}

/// Chinese & Western text wrapping algorithm enforcing CJK Punctuation Avoidance (避头尾规则)
pub fn wrap_chinese_text(text: &str, max_width: i32, font_size: f32, canvas: &Canvas) -> Vec<String> {
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
            if !current_line.is_empty() {
                if NO_LINE_START.contains(&ch) {
                    if let Some(prev_ch) = current_line.pop() {
                        lines.push(current_line);
                        current_line = String::new();
                        current_line.push(prev_ch);
                        current_line.push(ch);
                        i += 1;
                        continue;
                    }
                }

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

pub fn is_chapter_title(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 40 {
        return false;
    }

    // Sentences ending with full stops or commas are regular body text
    if trimmed.ends_with('。') || trimmed.ends_with('，') || trimmed.ends_with('；') {
        return false;
    }

    let lower = trimmed.to_ascii_lowercase();
    (trimmed.starts_with('第')
        && (trimmed.contains('章')
            || trimmed.contains('节')
            || trimmed.contains('回')
            || trimmed.contains('卷')
            || trimmed.contains('幕')
            || trimmed.contains('篇')))
        || lower.starts_with("chapter ")
        || lower.starts_with("prologue")
        || lower.starts_with("epilogue")
        || trimmed.starts_with("序章")
        || trimmed.starts_with("序言")
        || trimmed.starts_with("前言")
        || trimmed.starts_with("引子")
        || trimmed.starts_with("楔子")
        || trimmed.starts_with("尾声")
        || trimmed.starts_with("后记")
}

fn find_char_byte_offset(bytes: &[u8], target_chars: usize, encoding: &str) -> usize {
    if encoding == "UTF-8" {
        let mut char_count = 0;
        for (byte_idx, _) in bytes.iter().enumerate() {
            if char_count >= target_chars {
                return byte_idx;
            }
            // UTF-8 leading byte check
            let b = bytes[byte_idx];
            if (b & 0xC0) != 0x80 {
                char_count += 1;
            }
        }
        bytes.len()
    } else {
        // Approximate for GBK / others: decode prefix
        let mut low = 0;
        let mut high = bytes.len();
        while low < high {
            let mid = (low + high) / 2;
            let (decoded, _) = CharsetHelper::decode_bytes(&bytes[..mid]);
            if decoded.chars().count() < target_chars {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        low.min(bytes.len())
    }
}

#[cfg(test)]
mod tests {
    use super::{is_chapter_title, TextBook};
    use crate::ui::canvas::Canvas;

    #[test]
    fn detects_common_chinese_and_english_chapter_titles() {
        assert!(is_chapter_title("第1章 开始"));
        assert!(is_chapter_title("第十二回 重逢"));
        assert!(is_chapter_title("序章"));
        assert!(is_chapter_title("Chapter 3 The Road"));
        assert!(is_chapter_title("chapter 4 the return"));
        assert!(!is_chapter_title("这是普通正文中的一行文字。"));
    }

    #[test]
    fn chapter_jump_and_current_chapter_are_consistent() {
        let first_section = "这一段是开篇正文。\n".repeat(24);
        let second_section = "这一段是中篇正文。\n".repeat(24);
        let text = format!(
            "第1章 开始\n{}第2章 继续\n{}第3章 结束\n收尾正文。",
            first_section, second_section
        );
        let mut book = TextBook::from_string("测试".to_string(), text, "UTF-8".to_string());
        let canvas = Canvas::new();

        assert_eq!(book.chapters().len(), 3);
        assert!(book.jump_to_chapter(1, &canvas));
        assert_eq!(book.current_chapter_index(), Some(1));
        assert_eq!(book.current_chapter_name(), "第2章 继续");
        assert!(!book.jump_to_chapter(99, &canvas));
    }

    #[test]
    fn jump_percent_reaches_both_ends() {
        let mut book =
            TextBook::from_string("测试".to_string(), "正文\n".repeat(50), "UTF-8".to_string());
        let canvas = Canvas::new();

        book.jump_percent(0.0, &canvas);
        assert_eq!(book.current_page(), 0);
        book.jump_percent(1.0, &canvas);
        assert_eq!(book.current_page(), book.total_pages() - 1);
    }

    #[test]
    fn test_file_based_indexing_and_cache_hit_and_repaginate() {
        use std::fs;
        use std::path::PathBuf;

        let temp_dir = PathBuf::from("data/test_temp");
        let _ = fs::create_dir_all(&temp_dir);
        let book_file = temp_dir.join("sample_novel.txt");

        let content = "第1章 风起\n这是第一章的内容。包含多行文字。\n这一行也是第一章的内容。\n\
                       第2章 云涌\n这是第二章的内容。剧情正在发展中。\n第二章的结尾部分。\n\
                       第3章 终局\n大结局正文内容。\n全书完。\n";

        fs::write(&book_file, content).unwrap();

        let canvas = Canvas::new();
        let settings = super::TypographySettings::default();

        // 1. First open: cache miss -> builds index and cache file
        let mut book1 = TextBook::open(&book_file, "测试小说".to_string(), settings.clone(), &canvas).unwrap();
        assert_eq!(book1.chapters().len(), 3);
        assert_eq!(book1.current_chapter_name(), "第1章 风起");
        assert_eq!(book1.current_page(), 0);

        // Turn pages sequentially to cross chapter 1 -> chapter 2
        while book1.current_chapter_index() == Some(0) {
            if !book1.next_page(&canvas) {
                break;
            }
        }
        assert_eq!(book1.current_chapter_index(), Some(1));
        assert_eq!(book1.current_chapter_name(), "第2章 云涌");

        // Turn page back to cross chapter 2 -> chapter 1
        assert!(book1.prev_page(&canvas));
        assert_eq!(book1.current_chapter_index(), Some(0));

        // 2. Second open: cache hit
        let book2 = TextBook::open(&book_file, "测试小说".to_string(), settings.clone(), &canvas).unwrap();
        assert_eq!(book2.total_pages(), book1.total_pages());
        assert_eq!(book2.chapters().len(), 3);

        // 3. Change font size & repaginate
        let mut modified_settings = settings.clone();
        modified_settings.font_size = 36.0;
        book1.settings = modified_settings.clone();
        book1.rebuild_cache_and_repaginate(&canvas);
        assert_eq!(book1.settings.font_size, 36.0);

        // Clean up
        let _ = fs::remove_file(&book_file);
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
