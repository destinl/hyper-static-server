/// JSON 格式化模块
///
/// 提供 JSON 字符串的格式化、排序、美观展示等功能。
/// 包括去除转义符、按字段排序、统计数组等操作。

mod format;
mod stats;
mod display;

pub use format::{format_json, JsonFormattingResult};
pub use stats::{count_arrays, get_field_count};
