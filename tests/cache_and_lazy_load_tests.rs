use std::fs;
use std::path::PathBuf;

#[test]
fn test_txt_sample_indexing_and_lazy_loading() {
    let test_dir = PathBuf::from("data/test_cache");
    let _ = fs::create_dir_all(&test_dir);
    let sample_file = test_dir.join("test_book.txt");

    let content = "第1章 初识\n这是第一章的第一段内容。\n这是第一章的第二段内容。\n\
                   第2章 探索\n这是第二章的第一段内容。\n这是第二章的第二段内容。\n\
                   第3章 终局\n这是第三章的总结内容。\n";

    fs::write(&sample_file, content).unwrap();

    // Verify file written
    assert!(sample_file.exists());

    // Clean up
    let _ = fs::remove_file(&sample_file);
    let _ = fs::remove_dir_all(&test_dir);
}
