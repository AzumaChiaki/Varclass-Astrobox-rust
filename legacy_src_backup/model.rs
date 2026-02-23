//! 课程模型，与 Android Course.kt 对应

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Course {
    pub day: u8,
    pub name: String,
    pub start: String,
    pub end: String,
    #[serde(default)]
    pub room: String,
    #[serde(default = "default_week_type")]
    pub week_type: String,
}

fn default_week_type() -> String {
    crate::model::Course::WEEK_ALL.to_string()
}

impl Course {
    pub const WEEK_ALL: &'static str = "all";
    pub const WEEK_A: &'static str = "a";
    pub const WEEK_B: &'static str = "b";

    pub fn display_name(&self) -> String {
        let r = self.room.trim();
        if r.is_empty() {
            self.name.clone()
        } else {
            format!("{}（{}）", self.name, r)
        }
    }

    pub fn normalize_week_type(value: Option<&str>) -> String {
        match value.map(|s| s.trim().to_lowercase()).as_deref() {
            Some(Self::WEEK_A) => Self::WEEK_A.to_string(),
            Some(Self::WEEK_B) => Self::WEEK_B.to_string(),
            _ => Self::WEEK_ALL.to_string(),
        }
    }

    /// 从 "课程（教室）"/"课程(教室)" 中拆分 name + room
    pub fn split_name_and_room(display: &str) -> (String, String) {
        let s = display.trim();
        let re = regex::Regex::new(r"^\s*(.*?)\s*[（(]\s*(.+?)\s*[)）]\s*$").unwrap();
        if let Some(caps) = re.captures(s) {
            (
                caps.get(1).map(|m| m.as_str().trim().to_string()).unwrap_or_default(),
                caps.get(2).map(|m| m.as_str().trim().to_string()).unwrap_or_default(),
            )
        } else {
            (s.to_string(), String::new())
        }
    }
}
