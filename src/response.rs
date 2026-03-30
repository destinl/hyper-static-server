/// HTTP 响应构建模块
///
/// 负责构建各种 HTTP 响应，包括：
/// - 完整文件响应 (200 OK)
/// - 部分内容响应 (206 Partial Content)
/// - 未修改响应 (304 Not Modified)
/// - 目录列表 HTML
///
/// 支持 ETag 缓存验证和 Range 请求处理。

use axum::{
    body::Body,
    http::{HeaderValue, Response, StatusCode, header},
};
use tokio::io::AsyncReadExt;
use std::path::Path;
use std::time::SystemTime;
use tokio::fs::File;
use tokio::io::{AsyncSeekExt, SeekFrom};
use tokio_util::io::ReaderStream;
use urlencoding::encode;

use crate::error::{ServerError, ServerResult};
use crate::mime::detect_mime_type;
use httpdate::{fmt_http_date, parse_http_date};

/// 文件元数据，用于 ETag 和 Last-Modified 生成
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// 文件大小 (字节)
    pub size: u64,
    /// 最后修改时间
    pub modified: SystemTime,
}

impl FileMetadata {
    /// 从文件元数据创建实例
    pub fn from_metadata(metadata: std::fs::Metadata) -> ServerResult<Self> {
        Ok(FileMetadata {
            size: metadata.len(),
            modified: metadata.modified().map_err(ServerError::from)?,
        })
    }

    /// 生成 ETag 值
    ///
    /// 格式："{mtime_hex}-{size_hex}"
    /// 例如："5f3a2b1c-1000"
    ///
    /// # Examples
    /// ```
    /// let meta = FileMetadata { size: 4096, modified: SystemTime::UNIX_EPOCH };
    /// let etag = meta.generate_etag();
    /// assert_eq!(etag, "0-1000");
    /// ```
    pub fn generate_etag(&self) -> String {
        let mtime = self
            .modified
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        format!("{:x}-{:x}", mtime, self.size)
    }

    /// 生成 Last-Modified HTTP 头值
    pub fn generate_last_modified(&self) -> ServerResult<HeaderValue> {
        let http_date = fmt_http_date(self.modified);
        Ok(HeaderValue::from_str(&http_date)
            .map_err(|_| ServerError::IoError("Invalid header value".into()))?)
    }
}

/// Range 请求解析结果
#[derive(Debug, Clone)]
pub struct RangeValue {
    /// 起始位置 (字节)
    pub start: u64,
    /// 结束位置 (字节，包含)
    pub end: u64,
}

impl RangeValue {
    /// 解析 Range 头
    ///
    /// 支持格式:
    /// - `bytes=1024-2047` (指定范围)
    /// - `bytes=1024-` (从 1024 到文件末尾)
    /// - `bytes=-500` (最后 500 字节)
    ///
    /// # Arguments
    /// * `range_header` - Range 头的值
    /// * `file_size` - 文件总大小
    ///
    /// # Returns
    /// 解析后的 Range 值
    ///
    /// # Errors
    /// 返回 ServerError::InvalidRange 如果格式错误
    pub fn parse(range_header: &str, file_size: u64) -> ServerResult<Self> {
        // 期望格式："bytes=start-end" 或 "bytes=start-" 或 "bytes=-suffix"
        if !range_header.starts_with("bytes=") {
            return Err(ServerError::InvalidRange);
        }

        let range_spec = &range_header[6..]; // 跳过 "bytes="
        let parts: Vec<&str> = range_spec.split('-').collect();

        if parts.len() != 2 {
            return Err(ServerError::InvalidRange);
        }

        let (start, end) = match (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
            // 情况 1: bytes=start-end
            (Ok(start), Ok(end)) => {
                if start > end || start >= file_size {
                    return Err(ServerError::InvalidRange);
                }
                (start, end.min(file_size - 1))
            }
            // 情况 2: bytes=start- (从 start 到末尾)
            (Ok(start), Err(_)) => {
                if start >= file_size {
                    return Err(ServerError::InvalidRange);
                }
                (start, file_size - 1)
            }
            // 情况 3: bytes=-suffix (最后 suffix 字节)
            (Err(_), Ok(suffix)) => {
                if suffix == 0 || suffix > file_size {
                    return Err(ServerError::InvalidRange);
                }
                (file_size - suffix, file_size - 1)
            }
            // 无效格式
            (Err(_), Err(_)) => return Err(ServerError::InvalidRange),
        };

        Ok(RangeValue { start, end })
    }

    /// 计算内容长度
    pub fn content_length(&self) -> u64 {
        self.end - self.start + 1
    }

    /// 生成 Content-Range 头值
    /// 格式：bytes start-end/total
    pub fn to_content_range(&self, total: u64) -> String {
        format!("bytes {}-{}/{}", self.start, self.end, total)
    }
}

/// 检查客户端缓存是否仍然有效
///
/// 检查 If-None-Match 和 If-Modified-Since 头
///
/// # Arguments
/// * `if_none_match` - If-None-Match 头值 (可选)
/// * `etag` - 当前文件 ETag
/// * `if_modified_since` - If-Modified-Since 头值 (可选)
/// * `mtime` - 文件最后修改时间
///
/// # Returns
/// `true` 如果客户端缓存仍然有效 (应返回 304)
pub fn is_cache_valid(
    if_none_match: Option<&str>,
    etag: &str,
    if_modified_since: Option<&str>,
    modified: SystemTime,
) -> bool {
    // 检查 If-None-Match (ETag 匹配)
    if let Some(client_etag) = if_none_match {
        let mut matches = client_etag
            .split(',')
            .map(str::trim)
            .map(|value| value.trim_start_matches("W/").trim_matches('"'));

        if matches.any(|value| value == etag) {
            return true;
        }
    }

    // 检查 If-Modified-Since
    if let Some(since) = if_modified_since {
        if let Ok(datetime) = parse_http_date(since) {
            if let Ok(client_time) = datetime.duration_since(SystemTime::UNIX_EPOCH) {
                if let Ok(resource_time) = modified.duration_since(SystemTime::UNIX_EPOCH) {
                    if client_time >= resource_time {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// 构建 304 Not Modified 响应
pub fn build_not_modified_response(etag: &str, last_modified: &HeaderValue) -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_MODIFIED)
        .header(header::ETAG, format!("\"{}\"", etag))
        .header(header::LAST_MODIFIED, last_modified)
        .body(Body::empty())
        .expect("Failed to build 304 response")
}

/// 构建 200 OK 完整文件响应
///
/// # Arguments
/// * `path` - 文件路径
/// * `metadata` - 文件元数据
///
/// # Returns
/// 包含文件内容、ETag、Last-Modified、Content-Type 的响应
pub async fn build_full_file_response(
    path: &Path,
    metadata: FileMetadata,
) -> ServerResult<Response<Body>> {
    let mime_type = detect_mime_type(path);
    let etag = metadata.generate_etag();
    let last_modified = metadata.generate_last_modified()?;

    let file = File::open(path).await.map_err(ServerError::from)?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime_type)
        .header(header::CONTENT_LENGTH, metadata.size.to_string())
        .header(header::ETAG, format!("\"{}\"", etag))
        .header(header::LAST_MODIFIED, last_modified)
        .body(body)
        .expect("Failed to build 200 response");

    Ok(response)
}

/// 构建 206 Partial Content 响应 (Range 请求)
///
/// # Arguments
/// * `path` - 文件路径
/// * `metadata` - 文件元数据
/// * `range` - 解析后的 Range 值
///
/// # Returns
/// 包含部分文件内容的 206 响应
pub async fn build_partial_response(
    path: &Path,
    metadata: FileMetadata,
    range: RangeValue,
) -> ServerResult<Response<Body>> {
    let mime_type = detect_mime_type(path);
    let etag = metadata.generate_etag();
    let last_modified = metadata.generate_last_modified()?;

    let mut file = File::open(path).await.map_err(ServerError::from)?;
    file.seek(SeekFrom::Start(range.start))
        .await
        .map_err(ServerError::from)?;

    let stream = ReaderStream::new(file.take(range.content_length()));
    let body = Body::from_stream(stream);
    let content_range = range.to_content_range(metadata.size);

    let response = Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header(header::CONTENT_TYPE, mime_type)
        .header(header::CONTENT_LENGTH, range.content_length().to_string())
        .header(header::CONTENT_RANGE, content_range)
        .header(header::ETAG, format!("\"{}\"", etag))
        .header(header::LAST_MODIFIED, last_modified)
        .header(header::ACCEPT_RANGES, "bytes")
        .body(body)
        .expect("Failed to build 206 response");

    Ok(response)
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn encode_path_segment(input: &str) -> String {
    encode(input).to_string()
}

/// 构建目录列表 HTML
///
/// # Arguments
/// * `path` - 目录路径
/// * `base_url` - 请求的 URL 路径
///
/// # Returns
/// HTML 格式的目录列表
pub fn build_directory_listing(
    path: &Path,
    base_url: &str,
) -> ServerResult<String> {
    let mut html = String::from("<!DOCTYPE html>\n<html><head>\n");
    html.push_str("<meta charset=\"utf-8\">\n");
    html.push_str("<title>Directory listing</title>\n");
    html.push_str("</head><body>\n");
    html.push_str(&format!("<h1>Index of {}</h1>\n", html_escape(base_url)));
    html.push_str("<ul>\n");

    // 添加父目录链接 (如果不是根目录)
    if base_url != "/" && !base_url.is_empty() {
        html.push_str("  <li><a href=\"../\">../</a></li>\n");
    }

    // 读取目录内容
    let entries = std::fs::read_dir(path)
        .map_err(|_| ServerError::PermissionDenied)?;

    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|_| ServerError::PermissionDenied)?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // 跳过隐藏文件 (以.开头的非. 和.. 文件)
        if name_str.starts_with('.') && name_str != "." && name_str != ".." {
            continue;
        }

        let is_dir = entry.metadata()
            .map(|m| m.is_dir())
            .unwrap_or(false);

        if is_dir {
            dirs.push(name_str.to_string());
        } else {
            files.push(name_str.to_string());
        }
    }

    // 排序并添加目录
    dirs.sort();
    files.sort();

    for dir in dirs {
        let link = format!("{}/", encode_path_segment(&dir));
        let label = html_escape(&format!("{}/", dir));
        html.push_str(&format!("  <li><a href=\"{}\">{}</a></li>\n", link, label));
    }

    for file in files {
        let link = encode_path_segment(&file);
        let label = html_escape(&file);
        html.push_str(&format!("  <li><a href=\"{}\">{}</a></li>\n", link, label));
    }

    html.push_str("</ul>\n");
    html.push_str("</body></html>");

    Ok(html)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn test_file_metadata_etag_generation() {
        // Happy Path: ETag 生成
        let meta = FileMetadata {
            size: 4096,
            modified: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1234567890),
        };
        let etag = meta.generate_etag();
        assert!(!etag.is_empty());
        assert!(etag.contains('-'));
        assert_eq!(etag, "499602d2-1000");
    }

    #[test]
    fn test_file_metadata_etag_zero_size() {
        // Edge Case: 零字节文件
        let meta = FileMetadata {
            size: 0,
            modified: SystemTime::UNIX_EPOCH,
        };
        let etag = meta.generate_etag();
        assert_eq!(etag, "0-0");
    }

    #[test]
    fn test_range_parse_normal() {
        // Happy Path: 正常范围解析
        let range = RangeValue::parse("bytes=1024-2047", 10000).unwrap();
        assert_eq!(range.start, 1024);
        assert_eq!(range.end, 2047);
        assert_eq!(range.content_length(), 1024);
    }

    #[test]
    fn test_range_parse_open_ended() {
        // Happy Path: 开放结束范围
        let range = RangeValue::parse("bytes=9000-", 10000).unwrap();
        assert_eq!(range.start, 9000);
        assert_eq!(range.end, 9999);
    }

    #[test]
    fn test_range_parse_suffix() {
        // Happy Path: 后缀范围 (最后 N 字节)
        let range = RangeValue::parse("bytes=-500", 1000).unwrap();
        assert_eq!(range.start, 500);
        assert_eq!(range.end, 999);
        assert_eq!(range.content_length(), 500);
    }

    #[test]
    fn test_range_parse_invalid_format() {
        // Error Case: 无效格式
        assert!(RangeValue::parse("1024-2047", 10000).is_err());
        assert!(RangeValue::parse("bytes=abc-def", 10000).is_err());
        assert!(RangeValue::parse("bytes=1024-2047-extra", 10000).is_err());
    }

    #[test]
    fn test_range_parse_out_of_bounds() {
        // Error Case: 超出范围
        assert!(RangeValue::parse("bytes=9000-", 5000).is_err());
        assert!(RangeValue::parse("bytes=2000-1000", 5000).is_err());
    }

    #[test]
    fn test_range_content_range_header() {
        // Happy Path: Content-Range 头生成
        let range = RangeValue { start: 100, end: 199 };
        let content_range = range.to_content_range(1000);
        assert_eq!(content_range, "bytes 100-199/1000");
    }

    #[test]
    fn test_is_cache_valid_etag_match() {
        let now = SystemTime::now();
        assert!(is_cache_valid(Some("\"abc-123\""), "abc-123", None, now));
        assert!(is_cache_valid(Some("W/\"abc-123\""), "abc-123", None, now));
        assert!(!is_cache_valid(Some("\"xyz-789\""), "abc-123", None, now));
    }

    #[test]
    fn test_is_cache_valid_etag_mismatch() {
        // Happy Path: ETag 不匹配，缓存无效
        assert!(!is_cache_valid(Some("\"xyz-789\""), "abc-123", None, SystemTime::now()));
    }

    #[test]
    fn test_build_directory_listing_structure() {
        // Happy Path: 目录列表 HTML 结构
        let temp_dir = std::env::temp_dir();
        let html = build_directory_listing(&temp_dir, "/test/").unwrap();

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<html>"));
        assert!(html.contains("</html>"));
        assert!(html.contains("<ul>"));
        assert!(html.contains("</ul>"));
        assert!(html.contains("Index of /test/"));
    }

    #[test]
    fn test_build_directory_listing_parent_link() {
        // Happy Path: 非根目录有父目录链接
        let temp_dir = std::env::temp_dir();
        let html = build_directory_listing(&temp_dir, "/sub/dir/").unwrap();
        assert!(html.contains("../"));
    }

    #[test]
    fn test_directory_listing_no_parent_at_root() {
        // Edge Case: 根目录没有父目录链接
        let temp_dir = std::env::temp_dir();
        let html = build_directory_listing(&temp_dir, "/").unwrap();
        assert!(!html.contains("../"));
    }
}
