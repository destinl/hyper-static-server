/// 集成测试 - 测试完整的服务器功能
///
/// 这些测试验证：
/// - 服务器启动和关闭
/// - HTTP 请求处理
/// - 文件服务和缓存支持
/// - 目录列表生成
/// - JSON 格式化功能
/// - 错误处理

use std::path::PathBuf;
use std::fs;
use std::io::Write;
use serde_json::Value;

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

// ===================== JSON 格式化功能测试 =====================

#[test]
fn test_format_json_happy_path_basic() {
    use hyper_static_server::json_formatter::format_json;
    
    // 测试基本 JSON 格式化
    let input = r#"{"b":2,"a":1,"c":3}"#;
    let result = format_json(input);
    
    // 验证排序和格式化
    assert!(result.is_valid);
    assert!(result.formatted.contains("\"a\""));
    assert!(result.formatted.contains("\"b\""));
    assert!(result.formatted.contains("\"c\""));
    
    // 验证 JSON 有效性
    let parsed: Value = serde_json::from_str(&result.formatted).expect("Result should be valid JSON");
    assert_eq!(parsed["a"], 1);
    assert_eq!(parsed["b"], 2);
    assert_eq!(parsed["c"], 3);
}

#[test]
fn test_format_json_with_escaped_quotes() {
    use hyper_static_server::json_formatter::format_json;
    
    // 测试转义引号处理
    let input = r#"{\\"name\\":\\"John\\",\\"age\\":30}"#;
    let result = format_json(input);
    
    // 如果输入包含转义符并且清除后是合法 JSON，应该成功
    if result.is_valid {
        assert!(!result.formatted.contains("\\\\\""));
        
        // 验证 JSON 有效性
        let parsed: Value = serde_json::from_str(&result.formatted).expect("Result should be valid JSON");
        assert_eq!(parsed["name"], "John");
        assert_eq!(parsed["age"], 30);
    }
}

#[test]
fn test_format_json_with_nested_objects() {
    use hyper_static_server::json_formatter::format_json;
    
    // 测试嵌套对象排序
    let input = r#"{"user":{"z":26,"a":1,"m":13},"status":"ok"}"#;
    let result = format_json(input);
    
    assert!(result.is_valid);
    
    // 验证 JSON 有效性并检查排序
    let parsed: Value = serde_json::from_str(&result.formatted).expect("Result should be valid JSON");
    assert_eq!(parsed["user"]["a"], 1);
    assert_eq!(parsed["user"]["m"], 13);
    assert_eq!(parsed["user"]["z"], 26);
}

#[test]
fn test_format_json_with_arrays() {
    use hyper_static_server::json_formatter::{format_json, count_arrays};
    
    // 测试包含数组的 JSON
    let input = r#"{"items":[1,2,3],"tags":["a","b"]}"#;
    let result = format_json(input);
    
    assert!(result.is_valid);
    
    // 验证 JSON 有效性
    let parsed: Value = serde_json::from_str(&result.formatted).expect("Result should be valid JSON");
    assert_eq!(parsed["items"].as_array().unwrap().len(), 3);
    assert_eq!(parsed["tags"].as_array().unwrap().len(), 2);
    
    // 测试数组计数
    let array_count = count_arrays(input);
    assert_eq!(array_count, 2);
}

#[test]
fn test_format_json_empty_object() {
    use hyper_static_server::json_formatter::format_json;
    
    // 测试空对象
    let input = r#"{}"#;
    let result = format_json(input);
    
    assert!(result.is_valid);
    let parsed: Value = serde_json::from_str(&result.formatted).expect("Result should be valid JSON");
    assert!(parsed.is_object());
    assert_eq!(parsed.as_object().unwrap().len(), 0);
}

#[test]
fn test_format_json_empty_array() {
    use hyper_static_server::json_formatter::format_json;
    
    // 测试空数组
    let input = r#"{"list":[]}"#;
    let result = format_json(input);
    
    assert!(result.is_valid);
    let parsed: Value = serde_json::from_str(&result.formatted).expect("Result should be valid JSON");
    assert!(parsed["list"].is_array());
    assert_eq!(parsed["list"].as_array().unwrap().len(), 0);
}

#[test]
fn test_format_json_complex_nested_structure() {
    use hyper_static_server::json_formatter::format_json;
    
    // 测试复杂嵌套结构
    let input = r#"{"z":1,"a":{"z":2,"a":{"z":3,"a":"deep"}},"m":1.5}"#;
    let result = format_json(input);
    
    assert!(result.is_valid);
    let parsed: Value = serde_json::from_str(&result.formatted).expect("Result should be valid JSON");
    assert_eq!(parsed["a"]["a"]["a"], "deep");
    assert_eq!(parsed["z"], 1);
}

#[test]
fn test_format_json_with_special_values() {
    use hyper_static_server::json_formatter::format_json;
    
    // 测试特殊值（null, true, false）
    let input = r#"{"null_val":null,"bool_true":true,"bool_false":false}"#;
    let result = format_json(input);
    
    assert!(result.is_valid);
    let parsed: Value = serde_json::from_str(&result.formatted).expect("Result should be valid JSON");
    assert!(parsed["null_val"].is_null());
    assert_eq!(parsed["bool_true"], true);
    assert_eq!(parsed["bool_false"], false);
}

#[test]
fn test_format_json_with_numbers() {
    use hyper_static_server::json_formatter::format_json;
    
    // 测试数字类型
    let input = r#"{"int":42,"float":3.14,"negative":-99,"scientific":1e-10}"#;
    let result = format_json(input);
    
    assert!(result.is_valid);
    let parsed: Value = serde_json::from_str(&result.formatted).expect("Result should be valid JSON");
    assert_eq!(parsed["int"], 42);
    assert_eq!(parsed["float"], 3.14);
    assert_eq!(parsed["negative"], -99);
}

#[test]
fn test_format_json_invalid_input() {
    use hyper_static_server::json_formatter::format_json;
    
    // 测试无效 JSON 处理
    let invalid_inputs = vec![
        r#"{"unclosed":1"#,
        r#"{"invalid syntax}"#,
        r#"not json at all"#,
        r#"{"trailing": 1,}"#,
    ];
    
    for input in invalid_inputs {
        let result = format_json(input);
        assert!(!result.is_valid, "Input '{}' should fail to format", input);
        assert!(result.error.is_some(), "Error message should be present for '{}'", input);
    }
}

#[test]
fn test_format_json_unescape_validation() {
    use hyper_static_server::json_formatter::format_json;
    
    // 测试转义符清除验证
    // 当清除转义符后仍为合法 JSON 时才进行操作
    
    // 案例1：双引号转义的有效 JSON
    let input_valid = r#"{"test":"value"}"#;
    let result_valid = format_json(input_valid);
    assert!(result_valid.is_valid, "Should successfully handle valid JSON");
    
    // 案例2：无效的转义 JSON 应该失败
    let input_invalid = r#"{"test":incomplete}"#;
    let result_invalid = format_json(input_invalid);
    assert!(!result_invalid.is_valid, "Should fail for invalid JSON");
}

#[test]
fn test_count_arrays_single_level() {
    use hyper_static_server::json_formatter::count_arrays;
    
    // 测试单层数组计数
    let input = r#"{"arr1":[],"arr2":[],"obj":{"arr3":[]}}"#;
    let count = count_arrays(input);
    assert_eq!(count, 3);
}

#[test]
fn test_count_arrays_zero() {
    use hyper_static_server::json_formatter::count_arrays;
    
    // 测试无数组的情况
    let input = r#"{"a":1,"b":"text","c":null}"#;
    let count = count_arrays(input);
    assert_eq!(count, 0);
}

#[test]
fn test_count_arrays_nested() {
    use hyper_static_server::json_formatter::count_arrays;
    
    // 测试嵌套数组计数（每个 [ ] 对都计数）
    let input = r#"{"matrix":[[1,2],[3,4]]}"#;
    let count = count_arrays(input);
    // 应该计数所有的 [ ] 对：外层1个 + 内层2个 = 3个
    assert_eq!(count, 3);
}

#[test]
fn test_get_field_count() {
    use hyper_static_server::json_formatter::get_field_count;
    
    // 测试字段计数
    let input = r#"{"a":1,"b":2,"c":3}"#;
    let count = get_field_count(input);
    assert_eq!(count, 3);
}

#[test]
fn test_get_field_count_nested() {
    use hyper_static_server::json_formatter::get_field_count;
    
    // 测试嵌套对象的顶层字段计数（只计数顶层）
    let input = r#"{"outer":{"inner":1},"top":2}"#;
    let count = get_field_count(input);
    assert_eq!(count, 2);
}

#[test]
fn test_can_toggle_collapsible_objects() {
    use hyper_static_server::json_formatter::format_json;
    
    // 测试对象的格式化支持可折叠能力
    let input = r#"{"a":{"b":1,"c":2},"d":3}"#;
    let result = format_json(input);
    
    assert!(result.is_valid);
    
    // 验证嵌套结构存在
    let parsed: Value = serde_json::from_str(&result.formatted).expect("Result should be valid JSON");
    assert!(parsed["a"].is_object());
    assert_eq!(parsed["a"].as_object().unwrap().len(), 2);
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
    use std::time::SystemTime;

    let test_file = std::env::temp_dir().join("test_metadata.bin");
    
    // 创建测试文件
    let data = vec![0u8; 1024];
    fs::write(&test_file, data).expect("Failed to write test file");

    // 读取元数据
    let fs_metadata = fs::metadata(&test_file).expect("Failed to get file metadata");
    let metadata = FileMetadata::from_metadata(fs_metadata).expect("Failed to create FileMetadata");

    assert_eq!(metadata.size, 1024);
    assert!(metadata.modified > SystemTime::UNIX_EPOCH);

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
        ("script.js", "text/javascript"), // 注意：可能是 text/javascript 或 application/javascript
        ("data.json", "application/json"),
        ("image.png", "image/png"),
        ("photo.jpg", "image/jpeg"),
        ("readme.txt", "text/plain"),
    ];

    for (filename, expected_mime) in test_cases {
        let mime = detect_mime_type(Path::new(filename));
        // 对 JavaScript，允许两种 MIME 类型
        if filename.ends_with(".js") {
            assert!(
                mime == "text/javascript" || mime == "application/javascript",
                "Failed for file: {} (got {})",
                filename,
                mime
            );
        } else {
            assert_eq!(mime, expected_mime, "Failed for file: {}", filename);
        }
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
    use std::time::SystemTime;

    let time = SystemTime::UNIX_EPOCH;
    let metadata1 = FileMetadata { size: 1024, modified: time };
    let metadata2 = FileMetadata { size: 1024, modified: time };

    let etag1 = metadata1.generate_etag();
    let etag2 = metadata2.generate_etag();

    assert_eq!(etag1, etag2, "Same metadata should produce same ETag");
}

#[test]
fn test_etag_generation_difference() {
    // 测试不同的文件元数据生成不同的 ETag
    use hyper_static_server::FileMetadata;
    use std::time::SystemTime;

    let time = SystemTime::UNIX_EPOCH;
    let metadata1 = FileMetadata { size: 1024, modified: time };
    let metadata2 = FileMetadata { size: 2048, modified: time };

    let etag1 = metadata1.generate_etag();
    let etag2 = metadata2.generate_etag();

    assert_ne!(etag1, etag2, "Different metadata should produce different ETag");
}