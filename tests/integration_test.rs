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

    // 创建测试文件
    let test_file = fixtures_dir.join("test.txt");
    std::fs::write(&test_file, "Hello, World!").unwrap();

    // 创建测试 HTML 文件
    let html_file = fixtures_dir.join("index.html");
    std::fs::write(&html_file, "<html><body>Test</body></html>").unwrap();

    // 创建测试子目录
    let subdir = fixtures_dir.join("subdir");
    let _ = std::fs::create_dir_all(&subdir);
    let subdir_file = subdir.join("nested.txt");
    std::fs::write(&subdir_file, "Nested content").unwrap();

    fixtures_dir
}

/// 清理测试文件
fn cleanup_test_files() {
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let _ = std::fs::remove_dir_all(&fixtures_dir);
}

#[tokio::test]
async fn test_200_ok_file() {
    // Happy Path: 成功获取文件
    let server = setup_test_server().await;
    setup_test_files();

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/test.txt", server.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert!(response.headers().contains_key("content-type"));
    assert!(response.headers().contains_key("etag"));
    assert!(response.headers().contains_key("last-modified"));

    let body = response.text().await.unwrap();
    assert_eq!(body, "Hello, World!");

    cleanup_test_files();
}

#[tokio::test]
async fn test_200_ok_html_file() {
    // Happy Path: HTML 文件返回正确的 Content-Type
    let server = setup_test_server().await;
    setup_test_files();

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/index.html", server.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let content_type = response.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(content_type.contains("text/html"));

    cleanup_test_files();
}

#[tokio::test]
async fn test_404_not_found() {
    // Error Case: 文件不存在
    let server = setup_test_server().await;
    setup_test_files();

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/nonexistent.txt", server.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 404);

    cleanup_test_files();
}

#[tokio::test]
async fn test_403_path_traversal_blocked() {
    // Security: 目录遍历尝试被阻止
    let server = setup_test_server().await;
    setup_test_files();

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/../../../etc/passwd", server.base_url))
        .send()
        .await
        .unwrap();

    // 应该返回 403 或 404 (取决于实现)
    assert!(response.status() == 403 || response.status() == 404);

    cleanup_test_files();
}

#[tokio::test]
async fn test_304_not_modified() {
    // Happy Path: 缓存验证 - 304 Not Modified
    let server = setup_test_server().await;
    setup_test_files();

    let client = reqwest::Client::new();

    // 第一次请求获取 ETag
    let response = client
        .get(format!("{}/test.txt", server.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let etag = response.headers().get("etag").unwrap().clone();

    // 第二次请求带 If-None-Match
    let response = client
        .get(format!("{}/test.txt", server.base_url))
        .header("If-None-Match", etag)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 304);

    cleanup_test_files();
}

#[tokio::test]
async fn test_206_partial_content() {
    // Happy Path: Range 请求 - 206 Partial Content
    let server = setup_test_server().await;
    setup_test_files();

    let client = reqwest::Client::new();

    // 创建一个大一点的测试文件
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let large_file = fixtures_dir.join("large.bin");
    std::fs::write(&large_file, vec![0u8; 1000]).unwrap();

    // 请求前 100 字节
    let response = client
        .get(format!("{}/large.bin", server.base_url))
        .header("Range", "bytes=0-99")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 206);
    assert!(response.headers().contains_key("content-range"));
    assert_eq!(response.headers().get("content-length").unwrap(), "100");

    cleanup_test_files();
}

#[tokio::test]
async fn test_directory_listing() {
    // Happy Path: 目录列表生成
    let server = setup_test_server().await;
    setup_test_files();

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/subdir/", server.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let content_type = response.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(content_type.contains("text/html"));

    let body = response.text().await.unwrap();
    assert!(body.contains("Index of"));
    assert!(body.contains("nested.txt"));

    cleanup_test_files();
}

#[tokio::test]
async fn test_cors_headers() {
    // Happy Path: CORS 头存在
    let server = setup_test_server().await;
    setup_test_files();

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/test.txt", server.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert!(response.headers().contains_key("access-control-allow-origin"));

    cleanup_test_files();
}

#[tokio::test]
async fn test_mime_type_detection() {
    // Happy Path: MIME 类型自动检测
    let server = setup_test_server().await;
    setup_test_files();

    let client = reqwest::Client::new();

    // 创建不同扩展名的测试文件
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");

    let css_file = fixtures_dir.join("style.css");
    std::fs::write(&css_file, "body {}").unwrap();

    let js_file = fixtures_dir.join("app.js");
    std::fs::write(&js_file, "console.log('test')").unwrap();

    let json_file = fixtures_dir.join("data.json");
    std::fs::write(&json_file, r#"{"test": true}"#).unwrap();

    // 验证 CSS
    let response = client
        .get(format!("{}/style.css", server.base_url))
        .send()
        .await
        .unwrap();
    assert!(response.headers().get("content-type").unwrap().to_str().unwrap().contains("text/css"));

    // 验证 JS
    let response = client
        .get(format!("{}/app.js", server.base_url))
        .send()
        .await
        .unwrap();
    assert!(response.headers().get("content-type").unwrap().to_str().unwrap().contains("javascript"));

    // 验证 JSON
    let response = client
        .get(format!("{}/data.json", server.base_url))
        .send()
        .await
        .unwrap();
    assert!(response.headers().get("content-type").unwrap().to_str().unwrap().contains("json"));

    cleanup_test_files();
}

#[tokio::test]
async fn test_index_html_served_at_root() {
    // Happy Path: 根路径自动返回 index.html
    let server = setup_test_server().await;
    setup_test_files();

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/", server.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let content_type = response.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(content_type.contains("text/html"));

    let body = response.text().await.unwrap();
    assert!(body.contains("<html>"));

    cleanup_test_files();
}
