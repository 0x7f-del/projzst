//! Integration tests for projzst library

use projzst::{info, pack, read_metadata, unpack, Metadata, ProjzstError};
use std::fs;
use tempfile::TempDir;

/// Helper to create test directory with sample files
fn create_test_directory(base: &std::path::Path) -> std::path::PathBuf {
    let source = base.join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("readme.txt"), "Hello, projzst!").unwrap();
    fs::write(source.join("data.bin"), vec![0u8, 1, 2, 3, 4]).unwrap();

    let subdir = source.join("subdir");
    fs::create_dir_all(&subdir).unwrap();
    fs::write(subdir.join("nested.txt"), "Nested file content").unwrap();

    source
}

/// Helper to create test metadata
fn create_test_metadata() -> Metadata {
    Metadata::new(
        "test-project",
        "Test Author",
        "test-format",
        "2024",
        "1.0.0",
        "A test project description",
    )
}

#[test]
fn test_pack_creates_valid_file() {
    let temp = TempDir::new().unwrap();
    let source = create_test_directory(temp.path());
    let output = temp.path().join("output.pjz");

    let metadata = create_test_metadata();
    pack(&source, &output, metadata, None::<&str>, 3).unwrap();

    assert!(output.exists());
    assert!(fs::metadata(&output).unwrap().len() > 4);
}

#[test]
fn test_read_metadata_from_packed_file() {
    let temp = TempDir::new().unwrap();
    let source = create_test_directory(temp.path());
    let output = temp.path().join("output.pjz");

    let original = create_test_metadata();
    pack(&source, &output, original.clone(), None::<&str>, 3).unwrap();

    let read = read_metadata(&output).unwrap();
    assert_eq!(read.name, original.name);
    assert_eq!(read.auth, original.auth);
    assert_eq!(read.fmt, original.fmt);
    assert_eq!(read.ed, original.ed);
    assert_eq!(read.ver, original.ver);
    assert_eq!(read.desc, original.desc);
}

#[test]
fn test_pack_and_unpack_full_cycle() {
    let temp = TempDir::new().unwrap();
    let source = create_test_directory(temp.path());
    let archive = temp.path().join("test.pjz");
    let extract = temp.path().join("extracted");

    let metadata = create_test_metadata();
    pack(&source, &archive, metadata, None::<&str>, 3).unwrap();
    unpack(&archive, &extract).unwrap();

    // Verify extracted files match original
    assert!(extract.join("readme.txt").exists());
    assert!(extract.join("data.bin").exists());
    assert!(extract.join("subdir/nested.txt").exists());

    let readme = fs::read_to_string(extract.join("readme.txt")).unwrap();
    assert_eq!(readme, "Hello, projzst!");

    let data = fs::read(extract.join("data.bin")).unwrap();
    assert_eq!(data, vec![0u8, 1, 2, 3, 4]);

    let nested = fs::read_to_string(extract.join("subdir/nested.txt")).unwrap();
    assert_eq!(nested, "Nested file content");
}

#[test]
fn test_unpack_creates_metadata_json() {
    let temp = TempDir::new().unwrap();
    let source = create_test_directory(temp.path());
    let archive = temp.path().join("test.pjz");
    let extract = temp.path().join("subdir/extracted");

    let metadata = create_test_metadata();
    pack(&source, &archive, metadata, None::<&str>, 3).unwrap();
    unpack(&archive, &extract).unwrap();

    // metadata.json should be in parent of extract dir
    let metadata_json = temp.path().join("subdir/metadata.json");
    assert!(metadata_json.exists());

    let content = fs::read_to_string(&metadata_json).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["name"], "test-project");
    assert_eq!(parsed["ver"], "1.0.0");
}

#[test]
fn test_info_extracts_metadata_to_json() {
    let temp = TempDir::new().unwrap();
    let source = create_test_directory(temp.path());
    let archive = temp.path().join("test.pjz");
    let json_output = temp.path().join("info/metadata.json");

    let metadata = Metadata::new("info-test", "Author", "fmt", "ed", "2.0.0", "desc");
    pack(&source, &archive, metadata, None::<&str>, 3).unwrap();

    let result = info(&archive, &json_output).unwrap();
    assert_eq!(result.name, "info-test");
    assert_eq!(result.ver, "2.0.0");

    assert!(json_output.exists());
    let content = fs::read_to_string(&json_output).unwrap();
    assert!(content.contains("info-test"));
    assert!(content.contains("2.0.0"));
}

#[test]
fn test_pack_with_extra_json_file() {
    let temp = TempDir::new().unwrap();
    let source = create_test_directory(temp.path());
    let extra_file = temp.path().join("extra.json");
    let archive = temp.path().join("output.pjz");

    // Create extra JSON file
    let extra_content = r#"{
        "custom_field": "custom_value",
        "numbers": [1, 2, 3],
        "nested": {"a": 1, "b": 2}
    }"#;
    fs::write(&extra_file, extra_content).unwrap();

    let metadata = Metadata::default();
    pack(&source, &archive, metadata, Some(&extra_file), 3).unwrap();

    let read = read_metadata(&archive).unwrap();
    assert_eq!(read.extra["custom_field"], "custom_value");
    assert_eq!(read.extra["numbers"][0], 1);
    assert_eq!(read.extra["nested"]["a"], 1);
}

#[test]
fn test_pack_with_different_compression_levels() {
    let temp = TempDir::new().unwrap();
    let source = create_test_directory(temp.path());

    let output_low = temp.path().join("low.pjz");
    let output_high = temp.path().join("high.pjz");

    let metadata = create_test_metadata();

    pack(&source, &output_low, metadata.clone(), None::<&str>, 1).unwrap();
    pack(&source, &output_high, metadata, None::<&str>, 19).unwrap();

    // Both should be valid
    assert!(read_metadata(&output_low).is_ok());
    assert!(read_metadata(&output_high).is_ok());

    // Higher compression should produce smaller file (usually)
    let size_low = fs::metadata(&output_low).unwrap().len();
    let size_high = fs::metadata(&output_high).unwrap().len();

    // Just verify both work, size comparison not guaranteed for small files
    assert!(size_low > 0);
    assert!(size_high > 0);
}

#[test]
fn test_error_source_not_found() {
    let temp = TempDir::new().unwrap();
    let nonexistent = temp.path().join("does_not_exist");
    let output = temp.path().join("output.pjz");

    let result = pack(&nonexistent, &output, Metadata::default(), None::<&str>, 3);
    assert!(matches!(result, Err(ProjzstError::SourceNotFound(_))));
}

#[test]
fn test_error_extra_file_not_found() {
    let temp = TempDir::new().unwrap();
    let source = create_test_directory(temp.path());
    let nonexistent_extra = temp.path().join("no_such_file.json");
    let output = temp.path().join("output.pjz");

    let result = pack(
        &source,
        &output,
        Metadata::default(),
        Some(&nonexistent_extra),
        3,
    );
    assert!(matches!(result, Err(ProjzstError::ExtraFileNotFound(_))));
}

#[test]
fn test_error_invalid_pjz_file() {
    let temp = TempDir::new().unwrap();
    let invalid = temp.path().join("invalid.pjz");

    // Create invalid file (too short)
    fs::write(&invalid, &[0u8, 1, 2]).unwrap();

    let result = read_metadata(&invalid);
    assert!(result.is_err());
}

#[test]
fn test_metadata_with_unicode() {
    let temp = TempDir::new().unwrap();
    let source = create_test_directory(temp.path());
    let archive = temp.path().join("unicode.pjz");

    let metadata = Metadata::new(
        "项目名称",
        "作者名 🚀",
        "フォーマット",
        "版本2024",
        "1.0.0-β",
        "Description with émojis 🎉 and spëcial çharacters",
    );

    pack(&source, &archive, metadata.clone(), None::<&str>, 3).unwrap();

    let read = read_metadata(&archive).unwrap();
    assert_eq!(read.name, metadata.name);
    assert_eq!(read.auth, metadata.auth);
    assert_eq!(read.desc, metadata.desc);
}

#[test]
fn test_empty_directory_pack() {
    let temp = TempDir::new().unwrap();
    let empty_source = temp.path().join("empty");
    fs::create_dir_all(&empty_source).unwrap();
    let archive = temp.path().join("empty.pjz");
    let extract = temp.path().join("extracted");

    let metadata = create_test_metadata();
    pack(&empty_source, &archive, metadata, None::<&str>, 3).unwrap();
    unpack(&archive, &extract).unwrap();

    assert!(extract.exists());
}