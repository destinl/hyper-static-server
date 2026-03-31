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
use tokio::io::{AsyncSeekExt, AsyncReadExt, SeekFrom};
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
    /// use std::time::SystemTime;
    /// use hyper_static_server::response::FileMetadata;
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

/// 文件信息结构体，用于目录列表
struct FileInfo {
    name: String,
    is_dir: bool,
    size: u64,
    modified: Option<std::time::SystemTime>,
}

/// 格式化文件大小为人类可读格式
fn format_file_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.1} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

/// 格式化时间为人类可读格式
fn format_time(time: Option<std::time::SystemTime>) -> String {
    match time {
        Some(t) => fmt_http_date(t),
        None => "-".to_string(),
    }
}

/// 构建目录列表 HTML
///
/// # Arguments
/// * `path` - 目录路径
/// * `base_url` - 请求的 URL 路径
/// * `allow_upload` - 是否允许上传文件
/// * `allow_delete` - 是否允许删除文件
///
/// # Returns
/// HTML 格式的目录列表
pub fn build_directory_listing(
    path: &Path,
    base_url: &str,
    allow_upload: bool,
    allow_delete: bool,
) -> ServerResult<String> {
    let mut html = String::from(r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>目录浏览</title>
    <style>
        :root {
            --bg-color: #0a0a0a;
            --card-bg: #141414;
            --border-color: #262626;
            --text-primary: #fafafa;
            --text-secondary: #a1a1aa;
            --accent-color: #3b82f6;
            --accent-hover: #2563eb;
            --success-color: #22c55e;
            --warning-color: #f59e0b;
        }
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            background: var(--bg-color);
            color: var(--text-primary);
            line-height: 1.6;
            min-height: 100vh;
        }
        .container {
            max-width: 1200px;
            margin: 0 auto;
            padding: 2rem;
        }
        header {
            margin-bottom: 2rem;
            padding-bottom: 1.5rem;
            border-bottom: 1px solid var(--border-color);
        }
        h1 {
            font-size: 1.5rem;
            font-weight: 600;
            margin-bottom: 0.5rem;
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }
        .breadcrumb {
            color: var(--text-secondary);
            font-size: 0.875rem;
        }
        .breadcrumb a {
            color: var(--accent-color);
            text-decoration: none;
        }
        .breadcrumb a:hover {
            text-decoration: underline;
        }
        .toolbar {
            display: flex;
            gap: 1rem;
            margin-bottom: 1.5rem;
            flex-wrap: wrap;
            align-items: center;
        }
        .upload-area {
            flex: 1;
            min-width: 300px;
            border: 2px dashed var(--border-color);
            border-radius: 0.5rem;
            padding: 1.5rem;
            text-align: center;
            transition: all 0.2s;
            cursor: pointer;
            background: var(--card-bg);
        }
        .upload-area:hover, .upload-area.dragover {
            border-color: var(--accent-color);
            background: rgba(59, 130, 246, 0.1);
        }
        .upload-area input[type="file"] {
            display: none;
        }
        .upload-icon {
            font-size: 2rem;
            margin-bottom: 0.5rem;
        }
        .upload-text {
            color: var(--text-secondary);
            font-size: 0.875rem;
        }
        .upload-text span {
            color: var(--accent-color);
            font-weight: 500;
        }
        .stats {
            display: flex;
            gap: 1.5rem;
            padding: 1rem;
            background: var(--card-bg);
            border-radius: 0.5rem;
            border: 1px solid var(--border-color);
        }
        .stat-item {
            text-align: center;
        }
        .stat-value {
            font-size: 1.25rem;
            font-weight: 600;
            color: var(--accent-color);
        }
        .stat-label {
            font-size: 0.75rem;
            color: var(--text-secondary);
            text-transform: uppercase;
        }
        .file-list {
            background: var(--card-bg);
            border: 1px solid var(--border-color);
            border-radius: 0.5rem;
            overflow: hidden;
        }
        .file-header {
            display: grid;
            grid-template-columns: 1fr 120px 200px;
            padding: 0.75rem 1rem;
            background: rgba(255, 255, 255, 0.02);
            border-bottom: 1px solid var(--border-color);
            font-size: 0.75rem;
            font-weight: 600;
            color: var(--text-secondary);
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }
        .file-item {
            display: grid;
            grid-template-columns: 1fr 120px 200px;
            padding: 0.75rem 1rem;
            border-bottom: 1px solid var(--border-color);
            transition: background 0.15s;
            align-items: center;
        }
        .file-item:last-child {
            border-bottom: none;
        }
        .file-item:hover {
            background: rgba(255, 255, 255, 0.02);
        }
        .file-name {
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }
        .file-icon {
            width: 2rem;
            height: 2rem;
            display: flex;
            align-items: center;
            justify-content: center;
            background: var(--border-color);
            border-radius: 0.375rem;
            font-size: 0.875rem;
        }
        .file-icon.folder {
            background: rgba(59, 130, 246, 0.2);
            color: var(--accent-color);
        }
        .file-name a {
            color: var(--text-primary);
            text-decoration: none;
            font-weight: 500;
        }
        .file-name a:hover {
            color: var(--accent-color);
        }
        .file-size, .file-date {
            color: var(--text-secondary);
            font-size: 0.875rem;
        }
        .empty-state {
            text-align: center;
            padding: 3rem;
            color: var(--text-secondary);
        }
        .upload-progress {
            display: none;
            margin-top: 1rem;
        }
        .upload-progress.active {
            display: block;
        }
        .progress-bar {
            height: 4px;
            background: var(--border-color);
            border-radius: 2px;
            overflow: hidden;
        }
        .progress-fill {
            height: 100%;
            background: var(--accent-color);
            width: 0%;
            transition: width 0.3s;
        }
        .upload-status {
            margin-top: 0.5rem;
            font-size: 0.875rem;
            color: var(--text-secondary);
        }
        .toast {
            position: fixed;
            bottom: 2rem;
            right: 2rem;
            padding: 1rem 1.5rem;
            background: var(--card-bg);
            border: 1px solid var(--border-color);
            border-radius: 0.5rem;
            box-shadow: 0 10px 40px rgba(0, 0, 0, 0.5);
            transform: translateY(100px);
            opacity: 0;
            transition: all 0.3s;
            z-index: 1000;
        }
        .toast.show {
            transform: translateY(0);
            opacity: 1;
        }
        .toast.success {
            border-color: var(--success-color);
        }
        .toast.error {
            border-color: #ef4444;
        }
        @media (max-width: 768px) {
            .container {
                padding: 1rem;
            }
            .file-header, .file-item {
                grid-template-columns: 1fr;
                gap: 0.25rem;
            }
            .file-size, .file-date {
                font-size: 0.75rem;
            }
            .file-header .file-size,
            .file-header .file-date {
                display: none;
            }
            .stats {
                flex-direction: column;
                gap: 0.75rem;
            }
        }
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
                </svg>
                目录浏览
            </h1>
            <div class="breadcrumb" id="breadcrumb"></div>
        </header>

        <div class="toolbar">
            <div class="upload-area" id="uploadArea">
                <input type="file" id="fileInput" multiple>
                <div class="upload-icon">
                    <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                        <polyline points="17,8 12,3 7,8"></polyline>
                        <line x1="12" y1="3" x2="12" y2="15"></line>
                    </svg>
                </div>
                <div class="upload-text">
                    拖拽文件到此处或 <span>点击上传</span>
                </div>
                <div class="upload-progress" id="uploadProgress">
                    <div class="progress-bar">
                        <div class="progress-fill" id="progressFill"></div>
                    </div>
                    <div class="upload-status" id="uploadStatus">上传中...</div>
                </div>
            </div>
            <div class="stats" id="stats">
                <div class="stat-item">
                    <div class="stat-value" id="folderCount">0</div>
                    <div class="stat-label">文件夹</div>
                </div>
                <div class="stat-item">
                    <div class="stat-value" id="fileCount">0</div>
                    <div class="stat-label">文件</div>
                </div>
                <div class="stat-item">
                    <div class="stat-value" id="totalSize">0 B</div>
                    <div class="stat-label">总大小</div>
                </div>
            </div>
        </div>

        <div class="file-list">
            <div class="file-header">
                <span>名称</span>
                <span class="file-size">大小</span>
                <span class="file-date">修改时间</span>
            </div>
            <div id="fileListContent">
"#);

    // 添加父目录链接 (如果不是根目录)
    if base_url != "/" && !base_url.is_empty() {
        html.push_str(r#"                <div class="file-item">
                    <div class="file-name">
                        <div class="file-icon folder">
                            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <polyline points="15,18 9,12 15,6"></polyline>
                            </svg>
                        </div>
                        <a href="../">返回上级目录</a>
                    </div>
                    <span class="file-size">-</span>
                    <span class="file-date">-</span>
                </div>
"#);
    }

    // 读取目录内容
    let entries = std::fs::read_dir(path)
        .map_err(|_| ServerError::PermissionDenied)?;

    let mut items: Vec<FileInfo> = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|_| ServerError::PermissionDenied)?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // 跳过隐藏文件 (以.开头的非. 和.. 文件)
        if name_str.starts_with('.') && name_str != "." && name_str != ".." {
            continue;
        }

        let metadata = entry.metadata().ok();
        let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
        let modified = metadata.and_then(|m| m.modified().ok());

        items.push(FileInfo {
            name: name_str.to_string(),
            is_dir,
            size,
            modified,
        });
    }

    // 排序：目录在前，然后按名称排序
    items.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    // 统计信息
    let folder_count = items.iter().filter(|i| i.is_dir).count();
    let file_count = items.iter().filter(|i| !i.is_dir).count();
    let total_size: u64 = items.iter().filter(|i| !i.is_dir).map(|i| i.size).sum();

    // 生成文件列表
    for item in &items {
        let link = if item.is_dir {
            format!("{}/", encode_path_segment(&item.name))
        } else {
            encode_path_segment(&item.name)
        };
        let label = html_escape(&item.name);
        let size_str = if item.is_dir { "-".to_string() } else { format_file_size(item.size) };
        let date_str = format_time(item.modified);
        let icon_class = if item.is_dir { "folder" } else { "" };
        let icon_svg = if item.is_dir {
            r#"<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg>"#
        } else {
            r#"<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"></path><polyline points="13,2 13,9 20,9"></polyline></svg>"#
        };

        html.push_str(&format!(
            r#"                <div class="file-item">
                    <div class="file-name">
                        <div class="file-icon {}">{}</div>
                        <a href="{}">{}</a>
                    </div>
                    <span class="file-size">{}</span>
                    <span class="file-date">{}</span>
                </div>
"#,
            icon_class, icon_svg, link, label, size_str, date_str
        ));
    }

    if items.is_empty() {
        html.push_str(r#"                <div class="empty-state">
                    <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" style="margin-bottom: 1rem; opacity: 0.5;">
                        <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
                    </svg>
                    <p>此目录为空</p>
                </div>
"#);
    }

    // 添加 JavaScript 和关闭标签
    html.push_str(&format!(r#"            </div>
        </div>
    </div>

    <div class="toast" id="toast"></div>

    <script>
        // 初始化
        const currentPath = "{}";
        const folderCount = {};
        const fileCount = {};
        const totalSize = "{}";

        // 更新统计信息
        document.getElementById('folderCount').textContent = folderCount;
        document.getElementById('fileCount').textContent = fileCount;
        document.getElementById('totalSize').textContent = totalSize;

        // 生成面包屑导航
        function generateBreadcrumb(path) {{
            const breadcrumb = document.getElementById('breadcrumb');
            const parts = path.split('/').filter(p => p);
            let html = '<a href="/">根目录</a>';
            let currentPath = '';
            
            parts.forEach((part, index) => {{
                currentPath += '/' + part;
                if (index < parts.length - 1) {{
                    html += ' / <a href="' + currentPath + '/">' + part + '</a>';
                }} else {{
                    html += ' / ' + part;
                }}
            }});
            
            breadcrumb.innerHTML = html;
        }}

        generateBreadcrumb(currentPath);

        // 显示提示消息
        function showToast(message, type = 'info') {{
            const toast = document.getElementById('toast');
            toast.textContent = message;
            toast.className = 'toast ' + type + ' show';
            setTimeout(() => {{
                toast.classList.remove('show');
            }}, 3000);
        }}

        // 文件上传处理
        const uploadArea = document.getElementById('uploadArea');
        const fileInput = document.getElementById('fileInput');
        const uploadProgress = document.getElementById('uploadProgress');
        const progressFill = document.getElementById('progressFill');
        const uploadStatus = document.getElementById('uploadStatus');

        uploadArea.addEventListener('click', () => fileInput.click());

        uploadArea.addEventListener('dragover', (e) => {{
            e.preventDefault();
            uploadArea.classList.add('dragover');
        }});

        uploadArea.addEventListener('dragleave', () => {{
            uploadArea.classList.remove('dragover');
        }});

        uploadArea.addEventListener('drop', (e) => {{
            e.preventDefault();
            uploadArea.classList.remove('dragover');
            handleFiles(e.dataTransfer.files);
        }});

        fileInput.addEventListener('change', (e) => {{
            handleFiles(e.target.files);
        }});

        const allowUpload = {4};
        const allowDelete = {5};

        async function handleFiles(files) {{
            if (files.length === 0) return;

            if (!allowUpload) {{
                showToast('此服务器当前为只读模式，暂不支持上传文件', 'error');
                return;
            }}

            uploadProgress.classList.add('active');
            let successCount = 0;
            
            for (let i = 0; i < files.length; i++) {{
                const file = files[i];
                uploadStatus.textContent = `上传中: ${{file.name}} (${{i + 1}}/${{files.length}})`;
                
                try {{
                    const formData = new FormData();
                    formData.append('file', file);
                    
                    const uploadPath = currentPath.endsWith('/') ? currentPath : currentPath + '/';
                    const response = await fetch(uploadPath, {{
                        method: 'POST',
                        body: formData
                    }});
                    
                    const result = await response.json();
                    if (!result.success) {{
                        showToast(`上传失败: ${{file.name}} - ${{result.message}}`, 'error');
                    }} else {{
                        successCount++;
                    }}
                    
                    progressFill.style.width = ((i + 1) / files.length * 100) + '%';
                }} catch (err) {{
                    showToast(`上传失败: ${{file.name}}`, 'error');
                }}
            }}
            
            uploadProgress.classList.remove('active');
            progressFill.style.width = '0%';
            
            if (successCount > 0) {{
                showToast(`成功上传 ${{successCount}} 个文件`, 'success');
                setTimeout(() => location.reload(), 1500);
            }}
        }}

        // 更新上传区域样式
        if (!allowUpload) {{
            uploadArea.style.opacity = '0.5';
            uploadArea.style.cursor = 'not-allowed';
            uploadArea.querySelector('.upload-text').innerHTML = '上传已禁用 (启动时添加 --allow-upload 参数)';
        }}
    </script>
</body>
</html>"#, html_escape(base_url), folder_count, file_count, format_file_size(total_size), allow_upload, allow_delete));

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
        let html = build_directory_listing(&temp_dir, "/test/", false, false).unwrap();

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<html"));
        assert!(html.contains("</html>"));
        assert!(html.contains("file-list"));
        assert!(html.contains("/test/"));
    }

    #[test]
    fn test_build_directory_listing_parent_link() {
        // Happy Path: 非根目录有父目录链接
        let temp_dir = std::env::temp_dir();
        let html = build_directory_listing(&temp_dir, "/sub/dir/", false, false).unwrap();
        assert!(html.contains("../"));
    }

    #[test]
    fn test_directory_listing_no_parent_at_root() {
        // Edge Case: 根目录没有父目录链接
        let temp_dir = std::env::temp_dir();
        let html = build_directory_listing(&temp_dir, "/", false, false).unwrap();
        assert!(!html.contains("返回上级目录"));
    }

    #[test]
    fn test_directory_listing_upload_enabled() {
        // 测试上传启用时的提示
        let temp_dir = std::env::temp_dir();
        let html = build_directory_listing(&temp_dir, "/", true, false).unwrap();
        assert!(html.contains("allowUpload = true"));
    }

    #[test]
    fn test_directory_listing_upload_disabled() {
        // 测试上传禁用时的提示
        let temp_dir = std::env::temp_dir();
        let html = build_directory_listing(&temp_dir, "/", false, false).unwrap();
        assert!(html.contains("allowUpload = false"));
    }
}
