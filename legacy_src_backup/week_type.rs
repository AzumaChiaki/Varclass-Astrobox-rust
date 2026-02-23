//! AB 周逻辑，与 Android WeekTypeHelper.kt 对应


const PREF_KEY_REF_DATE: &str = "ab_ref_date";
const PREF_KEY_REF_TYPE: &str = "ab_ref_type";
const PREF_KEY_REF_DAY: &str = "ab_ref_day";
const TAG_FILE: &str = "ab_tag.txt";

#[derive(Debug, Clone)]
pub struct AbWeekTag {
    pub ref_date: String,
    pub ref_type: String,
    pub ref_day: u8,
}

fn week_day_from_calendar(year: i32, month: u32, day: u32) -> u8 {
    let mut m = month as i32;
    let mut y = year;
    if m <= 2 {
        m += 12;
        y -= 1;
    }
    let c = y / 100;
    let yy = y % 100;
    let w = (c / 4 - 2 * c + yy + yy / 4 + 26 * (m + 1) / 10 + day as i32 - 1) % 7;
    match (w + 7) % 7 {
        0 => 7,
        n => n as u8,
    }
}

fn parse_ymd(s: &str) -> Option<(i32, u32, u32)> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y = parts[0].parse::<i32>().ok()?;
    let m = parts[1].parse::<u32>().ok()?;
    let d = parts[2].parse::<u32>().ok()?;
    if m > 12 || d > 31 {
        return None;
    }
    Some((y, m, d))
}

fn date_to_ordinal(year: i32, month: u32, day: u32) -> i64 {
    let mut m = month as i64;
    let mut y = year as i64;
    if m <= 2 {
        m += 12;
        y -= 1;
    }
    let c = y / 100;
    let yy = y % 100;
    (146097 * c) / 4 + (1461 * yy) / 4 + (153 * m + 2) / 5 + day as i64 - 719162
}

fn compute_week_from_ref(
    ref_date_str: &str,
    ref_type: &str,
    ref_day: u8,
    now_ymd: (i32, u32, u32),
) -> Option<String> {
    let (ref_y, ref_m, ref_d) = parse_ymd(ref_date_str)?;
    let ref_day = if (1..=7).contains(&ref_day) { ref_day } else { 1 };

    let ref_monday_offset = ref_day as i64 - 1;
    let now_week_day = week_day_from_calendar(now_ymd.0, now_ymd.1, now_ymd.2);
    let now_monday_offset = now_week_day as i64 - 1;

    let ref_ord = date_to_ordinal(ref_y, ref_m, ref_d);
    let now_ord = date_to_ordinal(now_ymd.0, now_ymd.1, now_ymd.2);

    let ref_monday = ref_ord - ref_monday_offset;
    let now_monday = now_ord - now_monday_offset;

    let diff_days = now_monday - ref_monday;
    let diff_weeks = diff_days / 7;
    let parity = ((diff_weeks % 2) + 2) % 2;

    Some(if parity == 0 {
        ref_type.to_string()
    } else if ref_type == "a" {
        "b".to_string()
    } else {
        "a".to_string()
    })
}

fn normalize_tag(tag: AbWeekTag) -> Option<AbWeekTag> {
    if tag.ref_type != "a" && tag.ref_type != "b" {
        return None;
    }
    if !regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$")
        .unwrap()
        .is_match(&tag.ref_date)
    {
        return None;
    }
    let (y, m, d) = parse_ymd(&tag.ref_date)?;
    let date_week_day = week_day_from_calendar(y, m, d);
    Some(AbWeekTag {
        ref_date: tag.ref_date,
        ref_type: tag.ref_type,
        ref_day: date_week_day,
    })
}

pub fn get_current_week_type(storage: &dyn Storage) -> Option<String> {
    let tag = read_tag(storage)?;
    let now = chrono_now_ymd();
    compute_week_from_ref(&tag.ref_date, &tag.ref_type, tag.ref_day, now)
}

pub fn get_display_label(storage: &dyn Storage) -> String {
    match get_current_week_type(storage) {
        Some(t) if t == "a" => "当前: A周".to_string(),
        Some(t) if t == "b" => "当前: B周".to_string(),
        _ => "当前: 无AB区分".to_string(),
    }
}

pub fn set_current_week_ref(storage: &mut dyn Storage, week_type: &str) -> bool {
    if week_type != "a" && week_type != "b" {
        return false;
    }
    let now = chrono_now_ymd();
    let ref_day = week_day_from_calendar(now.0, now.1, now.2);
    let tag = AbWeekTag {
        ref_date: format!("{:04}-{:02}-{:02}", now.0, now.1, now.2),
        ref_type: week_type.to_string(),
        ref_day,
    };
    if let Some(tag) = normalize_tag(tag) {
        write_tag(storage, &tag);
        true
    } else {
        false
    }
}

pub fn clear_week_ref(storage: &mut dyn Storage) {
    storage.remove(PREF_KEY_REF_DATE);
    storage.remove(PREF_KEY_REF_TYPE);
    storage.remove(PREF_KEY_REF_DAY);
    storage.remove_file(TAG_FILE);
}

fn chrono_now_ymd() -> (i32, u32, u32) {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let days = secs / 86400;
    let j = days + 719162;
    let (y, m, d) = ordinal_to_date(j);
    (y, m, d)
}

fn ordinal_to_date(n: i64) -> (i32, u32, u32) {
    let a = n + 68569;
    let b = (4 * a) / 146097;
    let c = a - (146097 * b + 3) / 4;
    let e = (4000 * (c + 1)) / 1461001;
    let f = c - (1461 * e) / 4 + 31;
    let g = (80 * f) / 2447;
    let day = (f - (2447 * g) / 80) as u32;
    let h = g / 11;
    let month = (g + 2 - 12 * h) as u32;
    let year = (100 * (b - 49) + e + h) as i32;
    (year, month, day)
}

pub trait Storage {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&mut self, key: &str, value: &str);
    fn remove(&mut self, key: &str);
    fn read_file(&self, path: &str) -> Option<String>;
    fn write_file(&mut self, path: &str, content: &str);
    fn remove_file(&mut self, path: &str);
}

fn read_tag(storage: &dyn Storage) -> Option<AbWeekTag> {
    if let Some(text) = storage.read_file(TAG_FILE) {
        if text.trim().is_empty() {
            return None;
        }
        let v: serde_json::Value = serde_json::from_str(&text).ok()?;
        let ref_date = v.get("refDate")?.as_str()?.to_string();
        let ref_type = v.get("refType")?.as_str()?.to_string();
        let ref_day = v.get("refDay")?.as_u64()? as u8;
        return normalize_tag(AbWeekTag {
            ref_date,
            ref_type,
            ref_day,
        });
    }
    let date = storage.get(PREF_KEY_REF_DATE)?;
    let ref_type = storage.get(PREF_KEY_REF_TYPE)?;
    let ref_day = storage.get(PREF_KEY_REF_DAY)?.parse().ok()?;
    normalize_tag(AbWeekTag {
        ref_date: date,
        ref_type,
        ref_day,
    })
}

/// 应用外部同步来的 AB 周 tag（用于从设备拉取后写入）
pub fn apply_external_tag(storage: &mut dyn Storage, tag: &AbWeekTag) {
    if let Some(n) = normalize_tag(tag.clone()) {
        write_tag(storage, &n);
    }
}

fn write_tag(storage: &mut dyn Storage, tag: &AbWeekTag) {
    let j = serde_json::json!({
        "refDate": tag.ref_date,
        "refType": tag.ref_type,
        "refDay": tag.ref_day
    });
    storage.write_file(TAG_FILE, &j.to_string());
    storage.set(PREF_KEY_REF_DATE, &tag.ref_date);
    storage.set(PREF_KEY_REF_TYPE, &tag.ref_type);
    storage.set(PREF_KEY_REF_DAY, &tag.ref_day.to_string());
}
