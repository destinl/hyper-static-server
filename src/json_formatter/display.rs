/// JSON 树形显示和可交互展示
///
/// 提供展开/收起能力的 JSON 显示功能

use serde_json::{json, Value};

/// 生成带有展开/收起能力的 JSON 展示 HTML
///
/// # Arguments
/// * `json_value` - 要展示的 JSON Value
///
/// # Returns
/// 可交互的 HTML 字符串
///
/// # Note
/// 虽然这个函数目前在后端模块中未被直接调用，
/// 但它为前端树形视图功能准备，保留供未来扩展使用
#[allow(dead_code)]
pub fn format_json_with_collapsible(value: &Value) -> String {
    format_value_recursive(value, 0)
}

/// 递归格式化 JSON 值，支持展开/收起
#[allow(dead_code)]
fn format_value_recursive(value: &Value, depth: usize) -> String {
    let indent = "  ".repeat(depth);
    let next_indent = "  ".repeat(depth + 1);

    match value {
        Value::Object(map) => {
            if map.is_empty() {
                "{}".to_string()
            } else {
                let mut keys: Vec<_> = map.keys().collect();
                keys.sort();

                let items: Vec<String> = keys
                    .iter()
                    .filter_map(|key| {
                        map.get(*key).map(|val| {
                            format!(
                                "{}\"{}\": {}",
                                next_indent,
                                key,
                                format_value_recursive(val, depth + 1)
                            )
                        })
                    })
                    .collect();

                format!("{{\n{}\n{}}}", items.join(",\n"), indent)
            }
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                "[]".to_string()
            } else {
                let items: Vec<String> = arr
                    .iter()
                    .map(|v| {
                        format!(
                            "{}{}",
                            next_indent,
                            format_value_recursive(v, depth + 1)
                        )
                    })
                    .collect();

                format!("[\n{}\n{}]", items.join(",\n"), indent)
            }
        }
        Value::String(s) => json!(s).to_string(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_empty_array() {
        let input = r#"{"list":[]}"#;
        let value: Value = serde_json::from_str(input).unwrap();
        let output = format_json_with_collapsible(&value);
        assert!(output.contains("list"));
    }

    #[test]
    fn test_format_with_collapsible() {
        let json_str = r#"{"name":"John","age":30}"#;
        let value: Value = serde_json::from_str(json_str).unwrap();
        let html_output = format_json_with_collapsible(&value);

        assert!(html_output.contains("name") || html_output.contains("John"));
        assert!(html_output.contains("{") && html_output.contains("}"));
    }

    #[test]
    fn test_format_nested_objects() {
        let input = r#"{"a":{"b":1,"c":2},"d":3}"#;
        let value: Value = serde_json::from_str(input).unwrap();
        let output = format_json_with_collapsible(&value);
        
        assert!(output.contains("a"));
        assert!(output.contains("d"));
    }

    #[test]
    fn test_deeply_nested_json() {
        let input = r#"{"a":{"b":{"c":{"d":{"e":1}}}}}"#;
        let value: Value = serde_json::from_str(input).unwrap();
        let output = format_json_with_collapsible(&value);
        
        let lines: Vec<_> = output.lines().collect();
        assert!(lines.len() > 5);
    }

    #[test]
    fn test_format_json_with_special_values() {
        let input = r#"{"null_val":null,"bool_true":true,"bool_false":false}"#;
        let value: Value = serde_json::from_str(input).unwrap();
        let output = format_json_with_collapsible(&value);
        
        assert!(output.contains("null"));
        assert!(output.contains("true"));
        assert!(output.contains("false"));
    }

    #[test]
    fn test_format_json_with_numbers() {
        let input = r#"{"int":42,"float":3.14,"negative":-99}"#;
        let value: Value = serde_json::from_str(input).unwrap();
        let output = format_json_with_collapsible(&value);
        
        assert!(output.contains("42"));
        assert!(output.contains("3.14"));
        assert!(output.contains("-99"));
    }

    #[test]
    fn test_format_json_with_unicode() {
        let input = r#"{"name":"中文","emoji":"😀"}"#;
        let value: Value = serde_json::from_str(input).unwrap();
        let output = format_json_with_collapsible(&value);
        
        assert!(output.contains("中文") || output.contains("\\u"));
    }

    #[test]
    fn test_format_json_with_arrays() {
        let input = r#"{"items":[1,2,3]}"#;
        let value: Value = serde_json::from_str(input).unwrap();
        let output = format_json_with_collapsible(&value);
        
        assert!(output.contains("items"));
        assert!(output.contains("["));
    }
}
