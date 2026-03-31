/// JSON 格式化核心逻辑
///
/// 负责解析、排序和美化 JSON 字符串

use serde_json::{Value, Map};
use std::collections::BTreeMap;

/// JSON 格式化结果
///
/// 包含格式化后的 JSON 字符串、有效性信息、错误信息、
/// 数组统计等详细信息。
#[derive(Debug, Clone)]
pub struct JsonFormattingResult {
    /// 格式化后的 JSON 字符串
    pub formatted: String,
    /// 是否成功（是否为合法的 JSON）
    pub is_valid: bool,
    /// 错误信息（如果不合法）
    pub error: Option<String>,
    /// 数组计数：每个数组的字段名及其元素数量
    pub array_stats: BTreeMap<String, usize>,
    /// 对象展开/收起状态（传入无法预先知道，返回为空）
    #[allow(dead_code)]
    pub objects_collapsible: Vec<String>,
}

/// 尝试移除转义字符并验证 JSON 有效性
///
/// # Arguments
/// * `input` - 输入的 JSON 字符串（可能包含转义符）
///
/// # Returns
/// 返回清理后的 JSON 字符串（如果有效）
fn remove_escape_characters(input: &str) -> String {
    // 移除额外的转义反斜杠（保留真正需要的转义）
    // 这里的逻辑是：如果移除后仍然是合法 JSON，就执行移除
    input
        .replace("\\\"", "\"")
        .replace("\\\\", "\\")
}

/// 验证字符串是否为合法 JSON（最多一次）
#[allow(dead_code)]
pub fn is_valid_json(s: &str) -> bool {
    serde_json::from_str::<Value>(s).is_ok()
}

/// 尝试解析 JSON，可选择是否移除转义符
///
/// # Arguments
/// * `input` - 输入字符串
///
/// # Returns
/// 解析后的 Value 和是否应用了转义符移除
fn try_parse_json(input: &str) -> Result<(Value, bool), String> {
    // 首先尝试直接解析
    if let Ok(value) = serde_json::from_str::<Value>(input) {
        return Ok((value, false));
    }

    // 尝试移除转义符后解析
    let cleaned = remove_escape_characters(input);
    if let Ok(value) = serde_json::from_str::<Value>(&cleaned) {
        // 验证移除转义符后仍然合法
        return Ok((value, true));
    }

    // 原始字符串作为 JSON 字符串处理
    Err("Invalid JSON format".to_string())
}

/// 递归地统计 JSON 中所有的数组
fn collect_array_stats(
    value: &Value,
    path: &str,
    stats: &mut BTreeMap<String, usize>,
) {
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                let new_path: String = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", path, key)
                };
                collect_array_stats(val, &new_path, stats);
            }
        }
        Value::Array(arr) => {
            stats.insert(path.to_string(), arr.len());
            for (idx, val) in arr.iter().enumerate() {
                let item_path = format!("{}[{}]", path, idx);
                collect_array_stats(val, &item_path, stats);
            }
        }
        _ => {}
    }
}

/// 递归排序 JSON 对象的所有字段
fn sort_json_object(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted = Map::new();
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();

            for key in keys {
                if let Some(val) = map.get(key) {
                    sorted.insert(key.to_string(), sort_json_object(val));
                }
            }
            Value::Object(sorted)
        }
        Value::Array(arr) => {
            let sorted_items: Vec<_> = arr.iter().map(sort_json_object).collect();
            Value::Array(sorted_items)
        }
        other => other.clone(),
    }
}

/// 格式化 JSON 字符串
///
/// # Arguments
/// * `input` - 输入的 JSON 字符串
///
/// # Returns
/// 包含格式化结果的 JsonFormattingResult 结构体
///
/// # 功能特性
/// - 自动排序所有 JSON 对象的字段按英文字母顺序
/// - 支持移除转义符（仅当清除后仍为合法 JSON 时）
/// - 统计所有嵌套数组及其元素个数
/// - 美化 JSON 格式，添加正确的缩进
///
/// # Examples
/// ```
/// use hyper_static_server::format_json;
/// let input = r#"{"name":"John","age":30}"#;
/// let result = format_json(input);
/// assert!(result.is_valid);
/// assert!(result.formatted.contains("name"));
/// ```
pub fn format_json(input: &str) -> JsonFormattingResult {
    let input = input.trim();

    // 尝试解析 JSON
    match try_parse_json(input) {
        Ok((value, _escape_removed)) => {
            // 排序 JSON
            let sorted = sort_json_object(&value);

            // 收集数组统计
            let mut array_stats = BTreeMap::new();
            collect_array_stats(&sorted, "", &mut array_stats);

            // 格式化输出
            let formatted = serde_json::to_string_pretty(&sorted)
                .unwrap_or_else(|_| sorted.to_string());

            JsonFormattingResult {
                formatted,
                is_valid: true,
                error: None,
                array_stats,
                objects_collapsible: Vec::new(),
            }
        }
        Err(e) => JsonFormattingResult {
            formatted: input.to_string(),
            is_valid: false,
            error: Some(e),
            array_stats: BTreeMap::new(),
            objects_collapsible: Vec::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_simple_json() {
        let input = r#"{"name":"John","age":30}"#;
        let result = format_json(input);

        assert!(result.is_valid);
        assert!(result.error.is_none());
        assert!(result.formatted.contains("\"name\""));
        assert!(result.formatted.contains("\"age\""));
    }

    #[test]
    fn test_format_nested_json() {
        let input = r#"{"person":{"name":"John","age":30},"city":"NYC"}"#;
        let result = format_json(input);

        assert!(result.is_valid);
        let lines: Vec<_> = result.formatted.lines().collect();
        assert!(lines.len() > 3);
    }

    #[test]
    fn test_sorted_fields_alphabetically() {
        let input = r#"{"z":1,"a":2,"m":3}"#;
        let result = format_json(input);

        assert!(result.is_valid);
        // 检查是否按字母顺序
        let formatted = result.formatted.to_lowercase();
        let a_pos = formatted.find("\"a\"").unwrap_or(usize::MAX);
        let m_pos = formatted.find("\"m\"").unwrap_or(usize::MAX);
        let z_pos = formatted.find("\"z\"").unwrap_or(usize::MAX);

        assert!(a_pos < m_pos);
        assert!(m_pos < z_pos);
    }

    #[test]
    fn test_format_empty_object() {
        let input = "{}";
        let result = format_json(input);

        assert!(result.is_valid);
        assert_eq!(result.formatted.trim(), "{}");
    }

    #[test]
    fn test_invalid_json_missing_quotes() {
        let input = r#"{"name":John,"age":30}"#;
        let result = format_json(input);

        assert!(!result.is_valid);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_is_valid_json_basic() {
        assert!(is_valid_json("{}"));
        assert!(is_valid_json("[]"));
        assert!(is_valid_json(r#"{"key":"value"}"#));
        assert!(!is_valid_json("not json"));
    }

    #[test]
    fn test_remove_escape_characters_simple() {
        let input = r#"\"test\""#;
        let cleaned = remove_escape_characters(input);
        assert_eq!(cleaned, "\"test\"");
    }
}
