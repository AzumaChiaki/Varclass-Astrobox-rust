//! CSES YAML 课表格式适配器，与 Android CsesTimetableAdapter.kt 对应

use crate::model::Course;
use serde_yaml::Value;
use std::collections::BTreeMap;

fn to_hhmm(value: &str) -> Option<String> {
    let v = value.trim();
    let re_hhmm = regex::Regex::new(r"^([01]\d|2[0-3]):([0-5]\d)$").unwrap();
    let re_hhmmss = regex::Regex::new(r"^([01]\d|2[0-3]):([0-5]\d):([0-5]\d)$").unwrap();
    if let Some(caps) = re_hhmm.captures(v) {
        let h: i32 = caps.get(1)?.as_str().parse().ok()?;
        let m: i32 = caps.get(2)?.as_str().parse().ok()?;
        return Some(format!("{:02}:{:02}", h, m));
    }
    if let Some(caps) = re_hhmmss.captures(v) {
        let h: i32 = caps.get(1)?.as_str().parse().ok()?;
        let m: i32 = caps.get(2)?.as_str().parse().ok()?;
        return Some(format!("{:02}:{:02}", h, m));
    }
    None
}

fn get_i64(v: &Value) -> Option<i64> {
    v.as_i64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

/// 从 YAML Value 提取字符串（兼容 08:00:00、'10:00:00' 等时间格式）
fn value_to_str(v: &Value) -> String {
    if let Some(s) = v.as_str() {
        return s.trim().to_string();
    }
    if let Some(n) = v.as_i64() {
        return n.to_string();
    }
    if let Some(n) = v.as_u64() {
        return n.to_string();
    }
    serde_yaml::to_string(v).unwrap_or_default().trim().to_string()
}

/// 恢复被合并的换行（粘贴时换行可能变成空格）
fn normalize_cses_newlines(text: &str) -> String {
    let mut s = text.trim().to_string();
    // 粘贴时换行可能变成空格，恢复关键分隔符前的换行
    s = s.replace(" subjects:", "\nsubjects:");
    s = s.replace(" schedules:", "\nschedules:");
    s = s.replace(" - name:", "\n- name:");
    s = s.replace("   room:", "\n  room:");
    s = s.replace(" - subject:", "\n  - subject:");
    s = s.replace("   enable_day:", "\n  enable_day:");
    s = s.replace("   weeks:", "\n  weeks:");
    s = s.replace("   classes:", "\n  classes:");
    s = s.replace("     start_time:", "\n    start_time:");
    s = s.replace("     end_time:", "\n    end_time:");
    s
}

pub fn import_from_cses(yaml_text: &str) -> Result<Vec<Course>, String> {
    let normalized = normalize_cses_newlines(yaml_text);
    let root: Value = serde_yaml::from_str(&normalized)
        .map_err(|e| format!("YAML 解析失败: {}（若从剪贴板粘贴，请确保保留换行）", e))?;

    let root = root
        .as_mapping()
        .ok_or_else(|| "根节点必须是对象".to_string())?;

    let version = root
        .get(&Value::String("version".into()))
        .and_then(get_i64);
    if version != Some(1) {
        return Err("CSES 格式需 version=1".to_string());
    }

    let mut subject_index: BTreeMap<String, (String, String)> = BTreeMap::new();
    if let Some(subjects) = root.get(&Value::String("subjects".into())) {
        if let Some(arr) = subjects.as_sequence() {
            for subj in arr {
                if let Some(m) = subj.as_mapping() {
                    let key = m
                        .get(&Value::String("name".into()))
                        .and_then(|v| v.as_str())
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();
                    if key.is_empty() {
                        continue;
                    }
                    let room_field = m
                        .get(&Value::String("room".into()))
                        .and_then(|v| v.as_str())
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();
                    let (base, room_from_name) = Course::split_name_and_room(&key);
                    let room = if room_field.is_empty() {
                        room_from_name
                    } else {
                        room_field
                    };
                    subject_index.insert(key, (base, room));
                }
            }
        }
    }

    let mut out = Vec::new();
    if let Some(schedules) = root.get(&Value::String("schedules".into())) {
        if let Some(arr) = schedules.as_sequence() {
            for schedule_any in arr {
                if let Some(schedule) = schedule_any.as_mapping() {
                    let day = schedule
                        .get(&Value::String("enable_day".into()))
                        .and_then(get_i64)
                        .unwrap_or(0) as u8;
                    if day < 1 || day > 7 {
                        continue;
                    }
                    if let Some(classes) = schedule
                        .get(&Value::String("classes".into()))
                        .and_then(|v| v.as_sequence())
                    {
                        for clazz_any in classes {
                            if let Some(clazz) = clazz_any.as_mapping() {
                                let subject_key = clazz
                                    .get(&Value::String("subject".into()))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.trim().to_string())
                                    .unwrap_or_default();
                                let start_time = clazz
                                    .get(&Value::String("start_time".into()))
                                    .map(value_to_str)
                                    .unwrap_or_default();
                                let end_time = clazz
                                    .get(&Value::String("end_time".into()))
                                    .map(value_to_str)
                                    .unwrap_or_default();
                                if subject_key.is_empty() {
                                    continue;
                                }
                                let Some(start) = to_hhmm(&start_time) else {
                                    continue;
                                };
                                let Some(end) = to_hhmm(&end_time) else {
                                    continue;
                                };
                                let (name, room) = subject_index
                                    .get(&subject_key)
                                    .cloned()
                                    .unwrap_or_else(|| Course::split_name_and_room(&subject_key));
                                if name.is_empty() {
                                    continue;
                                }
                                out.push(Course {
                                    day,
                                    name,
                                    start,
                                    end,
                                    room,
                                    week_type: Course::WEEK_ALL.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    if out.is_empty() {
        return Err("未解析到有效课程".to_string());
    }

    out.sort_by(|a, b| a.day.cmp(&b.day).then_with(|| a.start.cmp(&b.start)));
    Ok(out)
}
