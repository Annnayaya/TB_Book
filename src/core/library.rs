use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BookType {
    Text,
    Comic,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookMetadata {
    pub path: PathBuf,
    pub title: String,
    pub book_type: BookType,
    pub file_size_bytes: u64,
    pub last_read_timestamp: u64,
    pub current_page: usize,
    pub total_pages: usize,
    pub percent: f32,
    pub zoom_level: f32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LibraryDatabase {
    pub history: HashMap<String, BookMetadata>,
}

impl LibraryDatabase {
    pub fn load() -> Self {
        let candidates = [
            "data/history.json",
            "history.json",
            "/mnt/SDCARD/Apps/BrickReader/history.json",
        ];

        for path in candidates {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(db) = serde_json::from_str(&content) {
                    return db;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::create_dir_all("data");
            let _ = fs::write("data/history.json", json);
        }
    }

    pub fn update_progress(&mut self, path: &Path, current_page: usize, total_pages: usize, zoom: f32) {
        let key = path.to_string_lossy().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let percent = if total_pages > 0 {
            (current_page as f32) / (total_pages as f32)
        } else {
            0.0
        };

        if let Some(meta) = self.history.get_mut(&key) {
            meta.current_page = current_page;
            meta.total_pages = total_pages;
            meta.percent = percent;
            meta.zoom_level = zoom;
            meta.last_read_timestamp = now;
        } else {
            let title = path.file_stem().and_then(|s| s.to_str()).unwrap_or("未知").to_string();
            let book_type = Self::detect_type(path);
            let file_size_bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0);

            self.history.insert(
                key,
                BookMetadata {
                    path: path.to_path_buf(),
                    title,
                    book_type,
                    file_size_bytes,
                    last_read_timestamp: now,
                    current_page,
                    total_pages,
                    percent,
                    zoom_level: zoom,
                },
            );
        }
        self.save();
    }

    pub fn scan_books<P: AsRef<Path>>(dir: P) -> Vec<BookMetadata> {
        let mut results = Vec::new();
        let dir_path = dir.as_ref();
        if !dir_path.exists() {
            return results;
        }

        if let Ok(entries) = fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let book_type = Self::detect_type(&path);
                    if book_type != BookType::Unknown {
                        let title = path.file_stem().and_then(|s| s.to_str()).unwrap_or("未知").to_string();
                        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

                        results.push(BookMetadata {
                            path,
                            title,
                            book_type,
                            file_size_bytes: size,
                            last_read_timestamp: 0,
                            current_page: 0,
                            total_pages: 0,
                            percent: 0.0,
                            zoom_level: 1.0,
                        });
                    }
                }
            }
        }

        results.sort_by(|a, b| a.title.cmp(&b.title));
        results
    }

    pub fn detect_type(path: &Path) -> BookType {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        match ext.as_str() {
            "txt" | "md" | "epub" => BookType::Text,
            "cbz" | "zip" => BookType::Comic,
            _ => BookType::Unknown,
        }
    }
}
