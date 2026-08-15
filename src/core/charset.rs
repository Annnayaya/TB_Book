use chardetng::EncodingDetector;
use encoding_rs::{Encoding, GBK, UTF_8, UTF_16LE, UTF_16BE};

pub struct CharsetHelper;

impl CharsetHelper {
    /// Automatically detects encoding of a byte slice and decodes into a standard UTF-8 Rust String
    pub fn decode_bytes(bytes: &[u8]) -> (String, &'static str) {
        if bytes.is_empty() {
            return (String::new(), "UTF-8");
        }

        // 1. Check UTF-8 BOM
        if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            let (cow, _, _) = UTF_8.decode(&bytes[3..]);
            return (cow.into_owned(), "UTF-8 BOM");
        }

        // 2. Check UTF-16 BOMs
        if bytes.starts_with(&[0xFF, 0xFE]) {
            let (cow, _, _) = UTF_16LE.decode(&bytes[2..]);
            return (cow.into_owned(), "UTF-16 LE");
        }
        if bytes.starts_with(&[0xFE, 0xFF]) {
            let (cow, _, _) = UTF_16BE.decode(&bytes[2..]);
            return (cow.into_owned(), "UTF-16 BE");
        }

        // 3. Try UTF-8 strict validation
        if let Ok(utf8_str) = std::str::from_utf8(bytes) {
            return (utf8_str.to_string(), "UTF-8");
        }

        // 4. Use chardetng detector for CJK (Chinese / Japanese / Korean)
        let mut detector = EncodingDetector::new();
        detector.feed(bytes, true);
        let encoding: &'static Encoding = detector.guess(None, true);

        let (cow, _encoding_used, had_errors) = encoding.decode(bytes);
        if !had_errors {
            return (cow.into_owned(), encoding.name());
        }

        // 5. Fallback explicitly to GBK / GB18030 for Chinese web novels
        let (gbk_cow, _, _) = GBK.decode(bytes);
        (gbk_cow.into_owned(), "GBK (Fallback)")
    }
}
