//! WakeUp 课表格式适配器
//!
//! 与 Android WakeupTimetableAdapter.kt 对应，解析 5 段 JSON 结构。

use crate::model::Course;
use serde_json::Value;
use std::collections::BTreeMap;

fn is_valid_hhmm(value: &str) -> bool {
    let parts: Vec<&str> = value.split(':').collect();
    if parts.len() != 2 {
        return false;
    }
    let h = parts[0].parse::<i32>().unwrap_or(-1);
    let m = parts[1].parse::<i32>().unwrap_or(-1);
    (0..=23).contains(&h) && (0..=59).contains(&m)
}


/// 解析 WakeUp 节次表 JSON（node -> startTime/endTime）
fn parse_node_table(json_text: &str) -> BTreeMap<i32, (String, String)> {
    let arr: Vec<Value> = serde_json::from_str(json_text).unwrap_or_default();
    let mut out = BTreeMap::new();
    for obj in arr {
        if let Some(obj) = obj.as_object() {
            if let (Some(node), Some(start), Some(end)) = (
                obj.get("node").and_then(|v| v.as_i64()),
                obj.get("startTime").and_then(|v| v.as_str()),
                obj.get("endTime").and_then(|v| v.as_str()),
            ) {
                let start = start.trim().to_string();
                let end = end.trim().to_string();
                if node > 0 && is_valid_hhmm(&start) && is_valid_hhmm(&end) {
                    out.insert(node as i32, (start, end));
                }
            }
        }
    }
    out
}


/// 解析课程元数据 JSON（id -> courseName）
fn parse_course_meta(json_text: &str) -> BTreeMap<i32, String> {
    let arr: Vec<Value> = serde_json::from_str(json_text).unwrap_or_default();
    let mut out = BTreeMap::new();
    for obj in arr {
        if let Some(obj) = obj.as_object() {
            if let (Some(id), Some(name)) = (
                obj.get("id").and_then(|v| v.as_i64()),
                obj.get("courseName").and_then(|v| v.as_str()),
            ) {
                let name = name.trim().to_string();
                if id >= 0 && !name.is_empty() {
                    out.insert(id as i32, name);
                }
            }
        }
    }
    out
}


/// 恢复被合并的换行（粘贴时 5 段 JSON 可能用空格分隔或连成一行）
fn normalize_wakeup_newlines(text: &str) -> String {
    let mut s = text.trim().to_string();
    s = s.replace("} [", "}\n[");
    s = s.replace("] [", "]\n[");
    s = s.replace("] {", "]\n{");
    
    s = s.replace("][", "]\n[");
    s = s.replace("]{", "]\n{");
    s = s.replace("}[", "}\n[");
    s
}


/// 按换行分割为多段，每段为一行 JSON（header、节次表、config、课程元数据、课程安排）
fn split_wakeup_blocks(text: &str) -> Vec<String> {
    let normalized = normalize_wakeup_newlines(text);
    normalized
        .split('\n')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

pub fn import_from_wakeup(text: &str) -> Result<Vec<Course>, String> {
    let blocks = split_wakeup_blocks(text);

    if blocks.len() < 5 {
        return Err(format!(
            "WakeUp 格式需 5 段 JSON（当前 {} 段），每段一行",
            blocks.len()
        ));
    }

    let node_table = parse_node_table(blocks.get(1).map(|s| s.as_str()).unwrap_or("[]"));
    if node_table.is_empty() {
        return Err("未解析到节次时间表".to_string());
    }

    let course_meta = parse_course_meta(blocks.get(3).map(|s| s.as_str()).unwrap_or("[]"));

    let details: Vec<Value> = serde_json::from_str(blocks.get(4).map(|s| s.as_str()).unwrap_or("[]"))
        .map_err(|e| format!("课程安排段解析失败: {}", e))?;

    let mut out = Vec::new();
    for obj in details {
        let obj = match obj.as_object() {
            Some(o) => o,
            None => continue,
        };
        let day = match obj.get("day").and_then(|v| v.as_i64()) {
            Some(d) => d as u8,
            None => continue,
        };
        if day < 1 || day > 7 {
            continue;
        }
        let start_node = obj.get("startNode").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let step = obj.get("step").and_then(|v| v.as_i64()).unwrap_or(1).max(1) as i32;
        let end_node = start_node + step - 1;

        let (start, end) = (
            node_table.get(&start_node).map(|(s, _)| s.clone()).unwrap_or_default(),
            node_table.get(&end_node).map(|(_, e)| e.clone()).unwrap_or_default(),
        );
        if start.is_empty() || end.is_empty() {
            continue;
        }

        let course_id = obj.get("id").and_then(|v| v.as_i64()).unwrap_or(-1) as i32;
        let name = course_meta.get(&course_id).cloned().unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let room = obj.get("room").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let raw_type = obj.get("type").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let week_type = match raw_type {
            1 => Course::WEEK_A,
            2 => Course::WEEK_B,
            _ => Course::WEEK_ALL,
        };

        out.push(Course {
            day,
            name,
            start,
            end,
            room,
            week_type: week_type.to_string(),
        });
    }

    if out.is_empty() {
        return Err("未解析到有效课程".to_string());
    }

    Ok(out)
}
