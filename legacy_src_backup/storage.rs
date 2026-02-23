//! 课表与 AB 周持久化存储

use crate::model::Course;
use crate::week_type::{self, Storage};
use std::collections::BTreeMap;
use std::io::{Read, Write};

const TIMETABLE_FILE: &str = "timetable.json";
const AB_TAG_FILE: &str = "ab_tag.txt";

/// 基于文件系统的存储实现
pub struct FileStorage {
    base_dir: String,
}

impl FileStorage {
    pub fn new(base_dir: impl Into<String>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    fn path(&self, name: &str) -> std::path::PathBuf {
        std::path::Path::new(&self.base_dir).join(name)
    }
}

impl Storage for FileStorage {
    fn get(&self, key: &str) -> Option<String> {
        let path = self.path("prefs.json");
        let mut f = std::fs::File::open(path).ok()?;
        let mut s = String::new();
        f.read_to_string(&mut s).ok()?;
        let prefs: BTreeMap<String, String> = serde_json::from_str(&s).ok()?;
        prefs.get(key).cloned()
    }

    fn set(&mut self, key: &str, value: &str) {
        let path = self.path("prefs.json");
        let mut prefs: BTreeMap<String, String> = std::fs::File::open(&path)
            .ok()
            .and_then(|mut f| {
                let mut s = String::new();
                f.read_to_string(&mut s).ok()?;
                serde_json::from_str(&s).ok()
            })
            .unwrap_or_default();
        prefs.insert(key.to_string(), value.to_string());
        if let Ok(mut f) = std::fs::File::create(&path) {
            let _ = f.write_all(serde_json::to_string(&prefs).unwrap_or_default().as_bytes());
        }
    }

    fn remove(&mut self, key: &str) {
        let path = self.path("prefs.json");
        if let Ok(mut f) = std::fs::File::open(&path) {
            let mut s = String::new();
            if f.read_to_string(&mut s).is_ok() {
                if let Ok(mut prefs) = serde_json::from_str::<BTreeMap<String, String>>(&s) {
                    prefs.remove(key);
                    if let Ok(mut out) = std::fs::File::create(&path) {
                        let _ = out.write_all(serde_json::to_string(&prefs).unwrap_or_default().as_bytes());
                    }
                }
            }
        }
    }

    fn read_file(&self, path: &str) -> Option<String> {
        let p = self.path(path);
        let mut f = std::fs::File::open(p).ok()?;
        let mut s = String::new();
        f.read_to_string(&mut s).ok()?;
        Some(s)
    }

    fn write_file(&mut self, path: &str, content: &str) {
        let p = self.path(path);
        if let Ok(mut f) = std::fs::File::create(p) {
            let _ = f.write_all(content.as_bytes());
        }
    }

    fn remove_file(&mut self, path: &str) {
        let _ = std::fs::remove_file(self.path(path));
    }
}

/// 从插件目录加载课表（WASI 下为当前工作目录）
pub fn load_courses(base_dir: &str) -> Vec<Course> {
    let path = std::path::Path::new(base_dir).join(TIMETABLE_FILE);
    let mut f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return vec![],
    };
    let mut s = String::new();
    if f.read_to_string(&mut s).is_err() || s.trim().is_empty() {
        return vec![];
    }
    let arr: Vec<serde_json::Value> = match serde_json::from_str(&s) {
        Ok(a) => a,
        Err(_) => return vec![],
    };
    let mut out = Vec::new();
    for obj in arr {
        let obj = match obj.as_object() {
            Some(o) => o,
            None => continue,
        };
        let day = obj.get("day").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
        let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let start = obj.get("start").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let end = obj.get("end").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let room = obj.get("room").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let week_type = Course::normalize_week_type(obj.get("weekType").and_then(|v| v.as_str()));

        if (1..=7).contains(&day) && !name.is_empty() && !start.is_empty() && !end.is_empty() {
            out.push(Course {
                day,
                name,
                start,
                end,
                room,
                week_type,
            });
        }
    }
    normalize_and_deduplicate(&out)
}

/// 保存课表
pub fn save_courses(base_dir: &str, courses: &[Course]) {
    let normalized = normalize_and_deduplicate(courses);
    let arr: Vec<serde_json::Value> = normalized
        .iter()
        .map(|c| {
            serde_json::json!({
                "day": c.day,
                "name": c.name,
                "start": c.start,
                "end": c.end,
                "room": c.room,
                "weekType": Course::normalize_week_type(Some(&c.week_type))
            })
        })
        .collect();
    let path = std::path::Path::new(base_dir).join(TIMETABLE_FILE);
    if let Ok(mut f) = std::fs::File::create(path) {
        let _ = f.write_all(serde_json::to_string(&arr).unwrap_or_default().as_bytes());
    }
}

fn normalize_and_deduplicate(courses: &[Course]) -> Vec<Course> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for c in courses {
        let day = c.day;
        let name = c.name.trim().to_string();
        let start = c.start.trim().to_string();
        let end = c.end.trim().to_string();
        let room = c.room.trim().to_string();
        let week_type = Course::normalize_week_type(Some(&c.week_type));
        let key = (day, name.clone(), start.clone(), end.clone(), room.clone(), week_type.clone());
        if seen.insert(key) {
            if (1..=7).contains(&day) && !name.is_empty() && !start.is_empty() && !end.is_empty() {
                out.push(Course {
                    day,
                    name,
                    start,
                    end,
                    room,
                    week_type,
                });
            }
        }
    }
    out.sort_by(|a, b| {
        a.day.cmp(&b.day)
            .then_with(|| a.start.cmp(&b.start))
            .then_with(|| a.end.cmp(&b.end))
            .then_with(|| a.name.cmp(&b.name))
    });
    out
}

pub fn get_week_label(base_dir: &str) -> String {
    let storage = FileStorage::new(base_dir);
    week_type::get_display_label(&storage)
}

pub fn set_week_a(base_dir: &str) {
    let mut storage = FileStorage::new(base_dir);
    week_type::set_current_week_ref(&mut storage, "a");
}

pub fn set_week_b(base_dir: &str) {
    let mut storage = FileStorage::new(base_dir);
    week_type::set_current_week_ref(&mut storage, "b");
}

pub fn clear_week(base_dir: &str) {
    let mut storage = FileStorage::new(base_dir);
    week_type::clear_week_ref(&mut storage);
}

pub fn get_current_week_type_opt(base_dir: &str) -> Option<String> {
    let storage = FileStorage::new(base_dir);
    week_type::get_current_week_type(&storage)
}

/// 读取 ab_tag.txt 内容（用于同步到设备时带上 AB 周信息）
pub fn read_ab_tag_json(base_dir: &str) -> Option<String> {
    let path = std::path::Path::new(base_dir).join(AB_TAG_FILE);
    let mut f = std::fs::File::open(path).ok()?;
    let mut s = String::new();
    f.read_to_string(&mut s).ok()?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}
