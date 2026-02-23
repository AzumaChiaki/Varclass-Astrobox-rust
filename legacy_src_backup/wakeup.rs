//! WakeUp 课表格式适配器，与 Android WakeupTimetableAdapter.kt 对应

use crate::model::Course;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct ImportResult {
    pub courses: Vec<Course>,
    pub message: String,
}

fn is_valid_hhmm(value: &str) -> bool {
    let parts: Vec<&str> = value.split(':').collect();
    if parts.len() != 2 {
        return false;
    }
    let h = parts[0].parse::<i32>().unwrap_or(-1);
    let m = parts[1].parse::<i32>().unwrap_or(-1);
    (0..=23).contains(&h) && (0..=59).contains(&m)
}

/// 解析 WakeUp 节次表 JSON
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

/// 解析课程元数据 JSON
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

pub fn import_from_wakeup_text(text: &str) -> ImportResult {
    let blocks: Vec<&str> = text
        .split('\n')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if blocks.len() < 5 {
        return ImportResult {
            courses: vec![],
            message: "WakeUp 文件格式不完整（至少需要 5 段 JSON）".to_string(),
        };
    }

    let node_table = parse_node_table(blocks.get(1).copied().unwrap_or("[]"));
    if node_table.is_empty() {
        return ImportResult {
            courses: vec![],
            message: "未解析到节次时间表".to_string(),
        };
    }

    let course_meta = parse_course_meta(blocks.get(3).copied().unwrap_or("[]"));

    let details: Vec<Value> = match serde_json::from_str(blocks.get(4).copied().unwrap_or("[]")) {
        Ok(d) => d,
        Err(_) => {
            return ImportResult {
                courses: vec![],
                message: "课程安排段解析失败".to_string(),
            };
        }
    };

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
    ImportResult {
        message: format!("解析完成，共 {} 节", out.len()),
        courses: out,
    }
}

pub fn export_to_wakeup_text(courses: &[Course]) -> String {
    let normalized: Vec<&Course> = courses
        .iter()
        .filter(|c| (1..=7).contains(&c.day) && !c.name.is_empty() && is_valid_hhmm(&c.start) && is_valid_hhmm(&c.end))
        .collect();

    let unique_slots: std::collections::BTreeSet<(String, String)> = normalized
        .iter()
        .map(|c| (c.start.clone(), c.end.clone()))
        .collect();
    let mut unique_slots: Vec<(String, String)> = unique_slots.into_iter().collect();
    unique_slots.sort_by_key(|(s, _)| {
        let parts: Vec<&str> = s.split(':').collect();
        let h = parts.get(0).and_then(|x| x.parse::<i32>().ok()).unwrap_or(0);
        let m = parts.get(1).and_then(|x| x.parse::<i32>().ok()).unwrap_or(0);
        h * 60 + m
    });

    let mut node_map: BTreeMap<(String, String), i32> = BTreeMap::new();
    let mut section_arr = Vec::new();
    for (idx, slot) in unique_slots.iter().enumerate() {
        let node = (idx + 1) as i32;
        node_map.insert(slot.clone(), node);
        section_arr.push(serde_json::json!({
            "node": node,
            "startTime": slot.0,
            "endTime": slot.1,
            "timeTable": 1
        }));
    }

    let header = serde_json::json!({
        "courseLen": 45,
        "id": 1,
        "name": "默认",
        "sameBreakLen": false,
        "sameLen": true,
        "theBreakLen": 10
    });

    let config = serde_json::json!({
        "id": 1,
        "tableName": "Var课程表",
        "timeTable": 1,
        "nodes": unique_slots.len().max(1),
        "maxWeek": 25
    });

    let mut course_id_map: BTreeMap<String, i32> = BTreeMap::new();
    let mut courses_arr = Vec::new();
    let mut next_id = 0;
    for c in &normalized {
        let key = c.name.trim().to_string();
        if !course_id_map.contains_key(&key) {
            course_id_map.insert(key.clone(), next_id);
            courses_arr.push(serde_json::json!({
                "id": next_id,
                "tableId": 1,
                "courseName": key,
                "note": "无",
                "credit": 0.0,
                "color": "#ff2196f3"
            }));
            next_id += 1;
        }
    }

    let mut detail_arr = Vec::new();
    for c in &normalized {
        let node = *node_map.get(&(c.start.clone(), c.end.clone())).unwrap_or(&1);
        let type_val = match Course::normalize_week_type(Some(&c.week_type)).as_str() {
            Course::WEEK_A => 1,
            Course::WEEK_B => 2,
            _ => 0,
        };
        let course_id = *course_id_map.get(c.name.trim()).unwrap_or(&0);
        detail_arr.push(serde_json::json!({
            "id": course_id,
            "tableId": 1,
            "day": c.day,
            "startNode": node,
            "step": 1,
            "startWeek": 1,
            "endWeek": 25,
            "room": c.room,
            "teacher": "",
            "type": type_val,
            "ownTime": false,
            "startTime": "",
            "endTime": "",
            "level": 0
        }));
    }

    vec![
        serde_json::to_string(&header).unwrap(),
        serde_json::to_string(&section_arr).unwrap(),
        serde_json::to_string(&config).unwrap(),
        serde_json::to_string(&courses_arr).unwrap(),
        serde_json::to_string(&detail_arr).unwrap(),
    ]
    .join("\n")
}
