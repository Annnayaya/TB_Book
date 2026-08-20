use crate::core::text_engine::TypographySettings;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterIndex {
    pub index: usize,
    pub title: String,
    pub byte_offset: u64,
    pub byte_length: u64,
    pub epub_href: Option<String>,
    pub start_page: usize,
    pub page_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookIndexCache {
    pub file_path: String,
    pub file_size: u64,
    pub mtime: u64,
    pub encoding_name: String,
    pub typography: TypographySettings,
    pub total_pages: usize,
    pub total_chars: usize,
    pub chapters: Vec<ChapterIndex>,
}

impl BookIndexCache {
    pub fn cache_dir() -> PathBuf {
        PathBuf::from("data/cache")
    }

    pub fn cache_path_for_book(book_path: &Path) -> PathBuf {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        book_path.to_string_lossy().hash(&mut hasher);
        let hash = hasher.finish();

        let stem = book_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("book");
        let safe_stem: String = stem
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .take(16)
            .collect();
        let prefix = if safe_stem.is_empty() { "book" } else { &safe_stem };
        let filename = format!("{}_{:016x}.json", prefix, hash);
        Self::cache_dir().join(filename)
    }

    pub fn load_if_valid(
        book_path: &Path,
        file_size: u64,
        mtime: u64,
        typography: &TypographySettings,
    ) -> Option<Self> {
        let cache_file = Self::cache_path_for_book(book_path);
        if !cache_file.exists() {
            return None;
        }

        let file = File::open(&cache_file).ok()?;
        let reader = BufReader::new(file);
        let cache: Self = serde_json::from_reader(reader).ok()?;

        if cache.file_size == file_size
            && cache.mtime == mtime
            && cache.typography.is_compatible_with(typography)
            && !cache.chapters.is_empty()
            && cache.total_pages > 0
        {
            Some(cache)
        } else {
            None
        }
    }

    pub fn save(&self, book_path: &Path) -> std::io::Result<()> {
        let dir = Self::cache_dir();
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        let cache_file = Self::cache_path_for_book(book_path);
        let file = File::create(&cache_file)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, self)?;
        Ok(())
    }

    pub fn get_file_metadata(path: &Path) -> (u64, u64) {
        if let Ok(meta) = fs::metadata(path) {
            let size = meta.len();
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            (size, mtime)
        } else {
            (0, 0)
        }
    }
}
