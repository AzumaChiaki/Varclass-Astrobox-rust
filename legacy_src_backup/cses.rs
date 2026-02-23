//! CSES YAML 课表格式适配器，与 Android CsesTimetableAdapter.kt 对应

use crate::model::Course;
use serde_yaml::Value;
use std::collections::BTreeMap;

fn is_valid_hhmm(value: &str) -> bool {
    regex::Regex::new(r"^([01]\d|2[0-3]):([0-5]\d)$")
        .unwrap()
        .is_match(value.trim())
}

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

fn to_hhmmss(hhmm: &str) -> String {
    let re = regex::Regex::new(r"^([01]\d|2[0-3]):([0-5]\d)$").unwrap();
    if let Some(caps) = re.captures(hhmm.trim()) {
        let h: i32 = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
        let m: i32 = caps.get(2).unwrap().as_str().parse().unwrap_or(0);
        return format!("{:02}:{:02}:00", h, m);
    }
    "00:00:00".to_string()
}

fn day_to_chinese(day: u8) -> &'static str {
    match day {
        1 => "星期一",
        2 => "星期二",
        3 => "星期三",
        4 => "星期四",
        5 => "星期五",
        6 => "星期六",
        7 => "星期日",
        _ => "星期?",
    }
}

fn get_i64(v: &Value) -> Option<i64> {
    v.as_i64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

pub fn import_from_cses(yaml_text: &str) -> Vec<Course> {
    let root: Value = match serde_yaml::from_str(yaml_text) {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let root = match root.as_mapping() {
        Some(m) => m,
        None => return vec![],
    };

    let version = root
        .get(&Value::String("version".into()))
        .and_then(get_i64);
    if version != Some(1) {
        return vec![];
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
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.trim().to_string())
                                    .unwrap_or_default();
                                let end_time = clazz
                                    .get(&Value::String("end_time".into()))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.trim().to_string())
                                    .unwrap_or_default();
                                if subject_key.is_empty() {
                                    continue;
                                }
                                let start = match to_hhmm(&start_time) {
                                    Some(s) => s,
                                    None => continue,
                                };
                                let end = match to_hhmm(&end_time) {
                                    Some(e) => e,
                                    None => continue,
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

    out.sort_by(|a, b| a.day.cmp(&b.day).then_with(|| a.start.cmp(&b.start)));
    out
}

pub fn export_to_cses_yaml(courses: &[Course]) -> String {
    let mut normalized: Vec<&Course> = courses
        .iter()
        .filter(|c| {
            (1..=7).contains(&c.day)
                && !c.name.is_empty()
                && is_valid_hhmm(&c.start)
                && is_valid_hhmm(&c.end)
        })
        .collect();
    normalized.sort_by(|a, b| a.day.cmp(&b.day).then_with(|| a.start.cmp(&b.start)));

    let mut subjects_by_key: BTreeMap<String, Value> = BTreeMap::new();
    for c in &normalized {
        let key = if c.room.is_empty() {
            c.name.trim().to_string()
        } else {
            format!("{}（{}）", c.name.trim(), c.room.trim())
        };
        if !subjects_by_key.contains_key(&key) {
            let mut m = serde_yaml::Mapping::new();
            m.insert(
                Value::String("name".into()),
                Value::String(key.clone()),
            );
            if !c.room.is_empty() {
                m.insert(
                    Value::String("room".into()),
                    Value::String(c.room.trim().to_string()),
                );
            }
            subjects_by_key.insert(key, Value::Mapping(m));
        }
    }

    let mut schedules = Vec::new();
    let mut day_groups: BTreeMap<u8, Vec<&Course>> = BTreeMap::new();
    for c in &normalized {
        day_groups.entry(c.day).or_default().push(c);
    }
    for (day, list) in day_groups {
        let mut classes = Vec::new();
        for c in list {
            let subject_key = if c.room.is_empty() {
                c.name.trim().to_string()
            } else {
                format!("{}（{}）", c.name.trim(), c.room.trim())
            };
            let mut class_map = serde_yaml::Mapping::new();
            class_map.insert(
                Value::String("subject".into()),
                Value::String(subject_key),
            );
            class_map.insert(
                Value::String("start_time".into()),
                Value::String(to_hhmmss(&c.start)),
            );
            class_map.insert(
                Value::String("end_time".into()),
                Value::String(to_hhmmss(&c.end)),
            );
            classes.push(Value::Mapping(class_map));
        }
        let mut schedule_map = serde_yaml::Mapping::new();
        schedule_map.insert(
            Value::String("name".into()),
            Value::String(day_to_chinese(day).to_string()),
        );
        schedule_map.insert(
            Value::String("enable_day".into()),
            Value::Number(day.into()),
        );
        schedule_map.insert(
            Value::String("weeks".into()),
            Value::String("all".to_string()),
        );
        schedule_map.insert(
            Value::String("classes".into()),
            Value::Sequence(classes),
        );
        schedules.push(Value::Mapping(schedule_map));
    }

    let mut root = serde_yaml::Mapping::new();
    root.insert(Value::String("version".into()), Value::Number(1i64.into()));
    root.insert(
        Value::String("subjects".into()),
        Value::Sequence(
            subjects_by_key
                .into_values()
                .collect(),
        ),
    );
    root.insert(
        Value::String("schedules".into()),
        Value::Sequence(schedules),
    );

    serde_yaml::to_string(&Value::Mapping(root)).unwrap_or_default()
}
