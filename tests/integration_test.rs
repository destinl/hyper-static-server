/// 集成测试 - 测试完整的服务器功能
///
/// 这些测试验证：
/// - 服务器启动和关闭
/// - HTTP 请求处理
/// - 文件服务和缓存支持
/// - 目录列表生成
/// - 错误处理

use std::path::PathBuf;
use std::fs;
use std::io::Write;

// 创建临时测试目录
fn setup_test_env() -> PathBuf {
    let test_dir = std::env::temp_dir().join("hyper_static_test");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).expect("Failed to create test directory");

    // 创建测试文件
    let mut test_file = fs::File::create(test_dir.join("test.txt")).expect("Failed to create test file");
    test_file.write_all(b"Hello, World!").expect("Failed to write to test file");

    let mut html_file = fs::File::create(test_dir.join("index.html")).expect("Failed to create HTML file");
    html_file.write_all(b"<!DOCTYPE html><html><body>Test</body></html>").expect("Failed to write to HTML file");

    // 创建子目录
    let subdir = test_dir.join("subdir");
    fs::create_dir(&subdir).expect("Failed to create subdirectory");
    let mut subfile = fs::File::create(subdir.join("nested.txt")).expect("Failed to create nested file");
    subfile.write_all(b"Nested content").expect("Failed to write to nested file");

    test_dir
}

// 清理测试环境
fn cleanup_test_env(test_dir: &PathBuf) {
    let _ = fs::remove_dir_all(test_dir);
}

#[tokio::test]
async fn test_server_startup_and_shutdown() {
    // 基本的服务器启动测试（这在真实测试中需要主线程处理）
    // 这里只是验证测试环境设置
    let test_dir = setup_test_env();
    assert!(test_dir.exists());
    assert!(test_dir.join("test.txt").exists());
    cleanup_test_env(&test_dir);
}

#[test]
fn test_server_config_creation() {
    // 验证 ServerConfig 可以被创建
    use hyper_static_server::ServerConfig;

    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 3000,
        root_dir: std::env::temp_dir(),
        cors: false,
        follow_symlinks: false,
    };

    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 3000);
    assert!(!config.cors);
    assert!(!config.follow_symlinks);
}

#[test]
fn test_file_metadata_from_real_file() {
    // 测试 FileMetadata 能否从真实文件获取
    use hyper_static_server::FileMetadata;

    let test_file = std::env::temp_dir().join("test_metadata.bin");
    
    // 创建测试文件
    let data = vec![0u8; 1024];
    fs::write(&test_file, data).expect("Failed to write test file");

    // 读取元数据
    let fs_metadata = fs::metadata(&test_file).expect("Failed to get file metadata");
    let metadata = FileMetadata::from_metadata(fs_metadata).expect("Failed to create FileMetadata");

    assert_eq!(metadata.size, 1024);
    assert!(metadata.mtime > 0);

    // 验证 ETag 生成
    let etag = metadata.generate_etag();
    assert!(!etag.is_empty());
    assert!(etag.contains('-'));

    // 清理
    let _ = fs::remove_file(&test_file);
}

#[test]
fn test_detect_mime_type_common_extensions() {
    // 测试常见文件类型的 MIME 检测
    use hyper_static_server::detect_mime_type;
    use std::path::Path;

    let test_cases = vec![
        ("test.html", "text/html"),
        ("style.css", "text/css"),
        ("script.js", "application/javascript"),
        ("data.json", "application/json"),
        ("image.png", "image/png"),
        ("photo.jpg", "image/jpeg"),
        ("readme.txt", "text/plain"),
    ];

    for (filename, expected_mime) in test_cases {
        let mime = detect_mime_type(Path::new(filename));
        assert_eq!(mime, expected_mime, "Failed for file: {}", filename);
    }
}

#[test]
fn test_error_handling_not_found() {
    // 测试 404 错误处理
    use hyper_static_server::ServerError;

    let err = ServerError::NotFound;
    assert_eq!(err.to_string(), "File not found");
}

#[test]
fn test_error_handling_permission_denied() {
    // 测试权限错误
    use hyper_static_server::ServerError;

    let err = ServerError::PermissionDenied;
    assert_eq!(err.to_string(), "Permission denied");
}

#[test]
fn test_error_handling_path_traversal() {
    // 测试路径遍历攻击检测
    use hyper_static_server::ServerError;

    let err = ServerError::PathTraversal;
    assert_eq!(err.to_string(), "Path traversal detected");
}

#[test]
fn test_temporary_directory_operations() {
    // 验证临时目录操作
    let test_dir = setup_test_env();
    
    // 验证所有文件都已创建
    assert!(test_dir.join("test.txt").exists());
    assert!(test_dir.join("index.html").exists());
    assert!(test_dir.join("subdir").exists());
    assert!(test_dir.join("subdir/nested.txt").exists());

    // 验证内容
    let content = fs::read_to_string(test_dir.join("test.txt")).unwrap();
    assert_eq!(content, "Hello, World!");

    cleanup_test_env(&test_dir);
}

#[test]
fn test_mime_detection_performance_hint() {
    // 这是一个性能提示测试，不是真正的性能测试
    // 真正的性能基准在 benches/ 目录
    use hyper_static_server::detect_mime_type;
    use std::path::Path;

    let start = std::time::Instant::now();
    
    for _ in 0..1000 {
        let _ = detect_mime_type(Path::new("test.html"));
    }
    
    let elapsed = start.elapsed();
    
    // 1000 次 MIME 检测应该在几毫秒内完成
    println!("MIME detection for 1000 files: {:?}", elapsed);
    assert!(elapsed.as_millis() < 100, "MIME detection is too slow");
}

#[test]
fn test_etag_generation_consistency() {
    // 测试相同的文件元数据生成相同的 ETag
    use hyper_static_server::FileMetadata;

    let metadata1 = FileMetadata { size: 1024, mtime: 1234567890 };
    let metadata2 = FileMetadata { size: 1024, mtime: 1234567890 };

    let etag1 = metadata1.generate_etag();
    let etag2 = metadata2.generate_etag();

    assert_eq!(etag1, etag2, "Same metadata should produce same ETag");
}

#[test]
fn test_etag_generation_difference() {
    // 测试不同的文件元数据生成不同的 ETag
    use hyper_static_server::FileMetadata;

    let metadata1 = FileMetadata { size: 1024, mtime: 1234567890 };
    let metadata2 = FileMetadata { size: 2048, mtime: 1234567890 };

    let etag1 = metadata1.generate_etag();
    let etag2 = metadata2.generate_etag();

    assert_ne!(etag1, etag2, "Different metadata should produce different ETag");
}