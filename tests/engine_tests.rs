#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn test_charset_utf8_detection() {
        let text = "你好，Trimui Brick！这是一个测试。";
        let bytes = text.as_bytes();
        let (decoded, enc) = brick_reader_charset(bytes);
        assert_eq!(decoded, text);
        assert_eq!(enc, "UTF-8");
    }

    #[test]
    fn test_file_type_detection() {
        assert_eq!(detect_type(Path::new("test.txt")), "Text");
        assert_eq!(detect_type(Path::new("novel.epub")), "Text");
        assert_eq!(detect_type(Path::new("comic.cbz")), "Comic");
        assert_eq!(detect_type(Path::new("manga.zip")), "Comic");
        assert_eq!(detect_type(Path::new("image.png")), "Unknown");
    }

    fn brick_reader_charset(bytes: &[u8]) -> (String, &'static str) {
        if let Ok(utf8_str) = std::str::from_utf8(bytes) {
            (utf8_str.to_string(), "UTF-8")
        } else {
            (String::new(), "Unknown")
        }
    }

    fn detect_type(path: &Path) -> &'static str {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        match ext.as_str() {
            "txt" | "md" | "epub" => "Text",
            "cbz" | "zip" => "Comic",
            _ => "Unknown",
        }
    }
}
