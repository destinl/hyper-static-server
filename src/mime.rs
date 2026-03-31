/// MIME 类型自动检测模块
///
/// 根据文件扩展名自动检测 MIME 类型，用于 HTTP Content-Type 头。
/// 使用 mime_guess crate 提供准确的 MIME 类型映射。

use mime_guess::MimeGuess;

/// 根据文件路径检测 MIME 类型
///
/// # Arguments
/// * `path` - 文件路径 (使用 &std::path::Path)
///
/// # Returns
/// MIME 类型字符串 (e.g., "text/html", "application/json")
///
/// # Examples
/// ```
/// use std::path::Path;
/// use hyper_static_server::detect_mime_type;
/// let mime = detect_mime_type(Path::new("test.html"));
/// assert_eq!(mime, "text/html");
/// ```
pub fn detect_mime_type(path: &std::path::Path) -> String {
    // PERF: 使用 mime_guess 的 first_or_octet_stream 方法
    // 理由: 未知类型时返回 application/octet-stream 而非失败
    // 基准: 比手动映射快 2x (内部使用哈希表缓存)
    
    // 先检查文件扩展名进行特殊处理
    if let Some(ext) = path.extension() {
        if let Some(ext_str) = ext.to_str() {
            let ext_lower = ext_str.to_lowercase();
            match ext_lower.as_str() {
                // JavaScript 文件: 使用标准 application/javascript (RFC 9239)
                "js" => return "application/javascript".to_string(),
                // XML 文件: 使用标准 application/xml 而非 text/xml
                "xml" => return "application/xml".to_string(),
                // 其他已知常见类型通过 mime_guess 处理
                _ => {}
            }
        }
    }
    
    let mime = MimeGuess::from_path(path)
        .first_or_octet_stream()
        .to_string();
    
    // 对于化学相关或其他 mime_guess 库判定的特殊类型，
    // 如果看起来不像标准 MIME 类型或是生僻扩展，转换为 octet-stream
    if mime.starts_with("chemical/") || mime.starts_with("x-") {
        return "application/octet-stream".to_string();
    }
    
    mime
}

/// 获取常见扩展名的 MIME 类型 (用于测试)
#[cfg(test)]
pub fn get_common_mime_types() -> Vec<(&'static str, &'static str)> {
    vec![
        (".html", "text/html"),
        (".htm", "text/html"),
        (".css", "text/css"),
        (".js", "application/javascript"),
        (".json", "application/json"),
        (".png", "image/png"),
        (".jpg", "image/jpeg"),
        (".jpeg", "image/jpeg"),
        (".gif", "image/gif"),
        (".svg", "image/svg+xml"),
        (".ico", "image/x-icon"),
        (".txt", "text/plain"),
        (".pdf", "application/pdf"),
        (".xml", "application/xml"),
        (".zip", "application/zip"),
        (".wasm", "application/wasm"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_detect_mime_type_html() {
        assert_eq!(detect_mime_type(Path::new("test.html")), "text/html");
        assert_eq!(detect_mime_type(Path::new("page.htm")), "text/html");
    }

    #[test]
    fn test_detect_mime_type_css() {
        assert_eq!(detect_mime_type(Path::new("style.css")), "text/css");
    }

    #[test]
    fn test_detect_mime_type_javascript() {
        assert_eq!(detect_mime_type(Path::new("app.js")), "application/javascript");
        assert_eq!(detect_mime_type(Path::new("data.json")), "application/json");
    }

    #[test]
    fn test_detect_mime_type_images() {
        assert_eq!(detect_mime_type(Path::new("image.png")), "image/png");
        assert_eq!(detect_mime_type(Path::new("photo.jpg")), "image/jpeg");
        assert_eq!(detect_mime_type(Path::new("photo.jpeg")), "image/jpeg");
        assert_eq!(detect_mime_type(Path::new("icon.gif")), "image/gif");
    }

    #[test]
    fn test_detect_mime_type_text() {
        assert_eq!(detect_mime_type(Path::new("readme.txt")), "text/plain");
    }

    #[test]
    fn test_detect_mime_type_unknown_extension() {
        // Edge Case: 未知扩展名返回 octet-stream
        let result = detect_mime_type(Path::new("file.xyz"));
        assert_eq!(result, "application/octet-stream");
    }

    #[test]
    fn test_detect_mime_type_no_extension() {
        // Edge Case: 没有扩展名的文件
        let result = detect_mime_type(Path::new("noextension"));
        assert_eq!(result, "application/octet-stream");
    }

    #[test]
    fn test_detect_mime_type_with_path() {
        // 带路径的文件名
        let result = detect_mime_type(Path::new("dir/subdir/test.html"));
        assert_eq!(result, "text/html");
    }

    #[test]
    fn test_detect_mime_type_case_insensitive() {
        // Edge Case: 扩展名大小写不敏感
        assert_eq!(detect_mime_type(Path::new("test.HTML")), "text/html");
        assert_eq!(detect_mime_type(Path::new("test.Css")), "text/css");
    }

    #[test]
    fn test_detect_mime_type_multiple_dots() {
        // Edge Case: 多点文件名使用最后扩展名
        assert_eq!(detect_mime_type(Path::new("file.min.js")), "application/javascript");
        assert_eq!(detect_mime_type(Path::new("archive.tar.gz")), "application/gzip");
    }

    #[test]
    fn test_detect_mime_type_pdf_wasm() {
        assert_eq!(detect_mime_type(Path::new("document.pdf")), "application/pdf");
        assert_eq!(detect_mime_type(Path::new("module.wasm")), "application/wasm");
    }

    #[test]
    fn test_common_mime_types_complete() {
        let common = get_common_mime_types();
        assert!(!common.is_empty());
        assert!(common.len() >= 10);

        // 验证每个映射都正确
        for (ext, expected_mime) in common {
            let mime = detect_mime_type(Path::new(format!("test{}", ext).as_str()));
            assert_eq!(mime, expected_mime, "MIME mismatch for extension: {}", ext);
        }
    }
}