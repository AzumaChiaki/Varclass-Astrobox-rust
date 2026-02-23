//! 与穿戴设备同步课表（QAIC Interconnect 协议）

use std::sync::{Mutex, OnceLock};

use crate::astrobox::psys_host::{device, interconnect, register, thirdpartyapp};
use crate::model::Course;

const DEFAULT_PKG_NAME: &str = "com.azuma.syclass";
const TARGET_APP_NAME_KEYWORD: &str = "Var课程表";
const CANDIDATE_PKG_NAMES: [&str; 4] = [
    "com.azuma.syclass",
    "com.azuma.varclass",
    "com.azumachiaki.syclass",
    "com.azumachiaki.varclass",
];
pub fn interconnect_pkg_name() -> &'static str {
    DEFAULT_PKG_NAME
}

#[derive(Debug, Clone)]
pub struct AbTagForApply {
    pub ref_date: String,
    pub ref_type: String,
    pub ref_day: u8,
}

#[derive(Debug, Clone)]
pub struct InterconnectResult {
    pub courses: Vec<Course>,
    pub ab_tag: Option<AbTagForApply>,
}

#[derive(Debug, Clone)]
pub struct SyncSnapshot {
    pub status: String,
    pub cached_course_count: usize,
    pub last_device_addr: Option<String>,
    pub subscribed: bool,
}

#[derive(Default)]
struct SyncState {
    cached_courses: Vec<Course>,
    cached_ab_tag: Option<AbTagForApply>,
    status: String,
    last_device_addr: Option<String>,
    subscribed: bool,
}

static SYNC_STATE: OnceLock<Mutex<SyncState>> = OnceLock::new();

fn sync_state() -> &'static Mutex<SyncState> {
    SYNC_STATE.get_or_init(|| Mutex::new(SyncState::default()))
}

fn with_state<R>(f: impl FnOnce(&mut SyncState) -> R) -> R {
    let mut guard = sync_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&mut guard)
}

pub fn set_status(status: impl Into<String>) {
    let status = status.into();
    with_state(|state| {
        state.status = status;
    });
}

pub fn snapshot() -> SyncSnapshot {
    with_state(|state| SyncSnapshot {
        status: if state.status.is_empty() {
            "等待同步".to_string()
        } else {
            state.status.clone()
        },
        cached_course_count: state.cached_courses.len(),
        last_device_addr: state.last_device_addr.clone(),
        subscribed: state.subscribed,
    })
}

fn set_last_device(addr: &str) {
    let addr = addr.to_string();
    with_state(|state| {
        state.last_device_addr = Some(addr);
    });
}

/// 获取首个已连接设备地址；若无则回退到设备列表首个。
pub async fn first_connected_device_addr() -> Option<String> {
    let devices = device::get_connected_device_list().await;
    devices.first().map(|d| d.addr.clone())
}

/// 获取一个可用于订阅的设备地址：优先已连接设备，其次已发现设备。
async fn first_bootstrap_device_addr() -> Option<(String, bool)> {
    let connected = device::get_connected_device_list().await;
    if let Some(d) = connected.first() {
        return Some((d.addr.clone(), true));
    }

    let all = device::get_device_list().await;
    all.first().map(|d| (d.addr.clone(), false))
}

async fn resolve_target_pkg_name(addr: &str) -> Result<String, String> {
    let apps = thirdpartyapp::get_thirdparty_app_list(addr)
        .await
        .map_err(|e| format!("读取手表应用列表失败: {:?}", e))?;

    if let Some(app) = apps.iter().find(|a| a.app_name.contains(TARGET_APP_NAME_KEYWORD)) {
        return Ok(app.package_name.clone());
    }

    if let Some(app) = apps
        .iter()
        .find(|a| CANDIDATE_PKG_NAMES.iter().any(|pkg| *pkg == a.package_name))
    {
        return Ok(app.package_name.clone());
    }

    let app_names = apps
        .iter()
        .map(|a| format!("{}({})", a.app_name, a.package_name))
        .collect::<Vec<_>>()
        .join(", ");

    Err(format!(
        "手表未找到目标快应用。需要包含“{}”的应用。当前手表应用: {}",
        TARGET_APP_NAME_KEYWORD, app_names
    ))
}

async fn ensure_interconnect_registered(addr: &str, pkg_name: &str) -> Result<(), String> {
    match register::register_interconnect_recv(addr, pkg_name).await {
        Ok(()) => Ok(()),
        Err(err) => {
            let raw = format!("{:?}", err);
            let lower = raw.to_lowercase();
            if lower.contains("already") || lower.contains("exists") || lower.contains("duplicate")
            {
                Ok(())
            } else {
                Err(format!(
                    "注册接收失败(addr={}, pkg={}): {}",
                    addr, pkg_name, raw
                ))
            }
        }
    }
}

/// 启动时执行：订阅首个设备的 interconnect 通道。
pub async fn bootstrap_sync() -> Result<(), String> {
    let (addr, connected) = first_bootstrap_device_addr()
        .await
        .ok_or_else(|| "未检测到可用设备，请先连接或扫描手表".to_string())?;

    let pkg_name = if connected {
        resolve_target_pkg_name(&addr)
            .await
            .unwrap_or_else(|_| DEFAULT_PKG_NAME.to_string())
    } else {
        DEFAULT_PKG_NAME.to_string()
    };
    ensure_interconnect_registered(&addr, &pkg_name).await?;

    set_last_device(&addr);
    with_state(|state| {
        state.subscribed = true;
        state.status = if connected {
            format!("已订阅设备 {} 的同步通道 ({})", addr, pkg_name)
        } else {
            format!("已预注册设备 {} 的同步通道({})，待连接后生效", addr, pkg_name)
        };
    });
    Ok(())
}

/// 在设备后连场景下自动重试订阅；已订阅时直接返回。
pub async fn bootstrap_if_needed() -> Result<(), String> {
    let already_subscribed = with_state(|state| state.subscribed);
    if already_subscribed {
        return Ok(());
    }

    match bootstrap_sync().await {
        Ok(()) => Ok(()),
        Err(err) => {
            with_state(|state| {
                state.subscribed = false;
                state.status = format!("等待设备连接后自动重试: {}", err);
            });
            Err(err)
        }
    }
}

/// 推送课程到设备（pushTimetable）。
pub async fn sync_to_device(courses: &[Course], ab_tag_json: Option<&str>) -> Result<(), String> {
    let addr = first_connected_device_addr()
        .await
        .ok_or_else(|| "无已连接设备，请先连接手表".to_string())?;
    let pkg_name = resolve_target_pkg_name(&addr).await?;
    ensure_interconnect_registered(&addr, &pkg_name).await?;

    let classes: Vec<serde_json::Value> = courses
        .iter()
        .map(|c| {
            serde_json::json!({
                "day": c.day,
                "name": c.display_name(),
                "start": c.start,
                "end": c.end,
                "weekType": Course::normalize_week_type(Some(&c.week_type)),
            })
        })
        .collect();

    let ab_tag = ab_tag_json.and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok());

    let payload = serde_json::json!({
        "type": "pushTimetable",
        "classes": classes,
        "abTag": ab_tag,
    });

    interconnect::send_qaic_message(&addr, &pkg_name, &payload.to_string())
        .await
        .map_err(|e| {
            format!(
                "推送失败(addr={}, pkg={}): {:?}",
                addr, pkg_name, e
            )
        })?;

    set_last_device(&addr);
    with_state(|state| {
        state.subscribed = true;
        state.status = format!(
            "已向设备 {} 的 {} 推送 {} 节课程",
            addr, pkg_name, courses.len()
        );
    });
    Ok(())
}

/// 将当前内存缓存推送到设备。
pub async fn sync_cached_to_device() -> Result<(), String> {
    let (courses, ab_tag_json) = with_state(|state| {
        let ab_tag_json = state.cached_ab_tag.as_ref().map(|tag| {
            serde_json::json!({
                "refDate": tag.ref_date,
                "refType": tag.ref_type,
                "refDay": tag.ref_day,
            })
            .to_string()
        });
        (state.cached_courses.clone(), ab_tag_json)
    });

    if courses.is_empty() {
        return Err("当前无可推送课程，请先从手表拉取".to_string());
    }

    sync_to_device(&courses, ab_tag_json.as_deref()).await
}

/// 主动向设备请求课表（requestTimetable）。
pub async fn request_timetable_from_device() -> Result<(), String> {
    let addr = first_connected_device_addr()
        .await
        .ok_or_else(|| "无已连接设备，请先连接手表".to_string())?;
    let pkg_name = resolve_target_pkg_name(&addr).await?;

    ensure_interconnect_registered(&addr, &pkg_name).await?;

    let payload = serde_json::json!({ "type": "requestTimetable" });
    interconnect::send_qaic_message(&addr, &pkg_name, &payload.to_string())
        .await
        .map_err(|e| {
            format!(
                "请求失败(addr={}, pkg={}): {:?}",
                addr, pkg_name, e
            )
        })?;

    set_last_device(&addr);
    with_state(|state| {
        state.subscribed = true;
        state.status = format!("已向设备 {} 的 {} 请求最新课表", addr, pkg_name);
    });
    Ok(())
}

/// 在 `on_event` 里处理 interconnect 消息：只更新内存缓存，不写文件。
pub fn handle_interconnect_message(payload: &str) -> Result<InterconnectResult, String> {
    let (mut courses, ab_tag) =
        parse_timetable_data(payload).ok_or_else(|| {
            let mut brief = payload.replace('\n', " ");
            if brief.len() > 220 {
                brief.truncate(220);
                brief.push_str("...");
            }
            format!("消息不是可识别课表回包，payload={}", brief)
        })?;

    normalize_and_deduplicate(&mut courses);

    with_state(|state| {
        state.cached_courses = courses.clone();
        state.cached_ab_tag = ab_tag.clone();
        state.subscribed = true;
        state.status = format!("已从手表同步 {} 节课程", state.cached_courses.len());
    });

    Ok(InterconnectResult { courses, ab_tag })
}

fn parse_timetable_data(payload: &str) -> Option<(Vec<Course>, Option<AbTagForApply>)> {
    let root = find_timetable_payload(payload)?;
    let obj = root.as_object()?;

    let classes_arr = if let Some(arr) = obj.get("classes").and_then(|v| v.as_array()) {
        arr
    } else if let Some(arr) = obj.get("data").and_then(|v| v.as_array()) {
        // 兼容部分回调直接把课程数组放在 data 字段
        arr
    } else {
        return None;
    };
    let mut courses = Vec::new();

    for class_item in classes_arr {
        let Some(class_obj) = class_item.as_object() else {
            continue;
        };

        let Some(day) = class_obj.get("day").and_then(|v| v.as_u64()) else {
            continue;
        };
        let Some(display_name) = class_obj
            .get("name")
            .or_else(|| class_obj.get("courseName"))
            .and_then(|v| v.as_str())
        else {
            continue;
        };
        let Some(start) = class_obj
            .get("start")
            .or_else(|| class_obj.get("startSection"))
            .and_then(|v| v.as_str())
        else {
            continue;
        };
        let Some(end) = class_obj
            .get("end")
            .or_else(|| class_obj.get("endSection"))
            .and_then(|v| v.as_str())
        else {
            continue;
        };

        let day = day as u8;
        let display_name = display_name.to_string();
        let start = start.to_string();
        let end = end.to_string();
        let (name, room) = Course::split_name_and_room(&display_name);
        let week_type = Course::normalize_week_type(
            class_obj
                .get("weekType")
                .or_else(|| class_obj.get("week_type"))
                .and_then(|v| v.as_str()),
        );

        if (1..=7).contains(&day) && !name.is_empty() && !start.is_empty() && !end.is_empty() {
            courses.push(Course {
                day,
                name,
                start,
                end,
                room,
                week_type,
            });
        }
    }

    let ab_tag = obj.get("abTag").and_then(|tag| {
        let tag_obj = tag.as_object()?;
        let ref_date = tag_obj.get("refDate")?.as_str()?.to_string();
        let ref_type = tag_obj.get("refType")?.as_str()?.to_string();
        let ref_day = tag_obj.get("refDay")?.as_u64()? as u8;

        if (1..=7).contains(&ref_day) && (ref_type == "a" || ref_type == "b") {
            Some(AbTagForApply {
                ref_date,
                ref_type,
                ref_day,
            })
        } else {
            None
        }
    });

    Some((courses, ab_tag))
}

fn find_timetable_payload(raw_payload: &str) -> Option<serde_json::Value> {
    let parsed: serde_json::Value = serde_json::from_str(raw_payload).ok()?;
    let mut stack = vec![parsed];

    while let Some(current) = stack.pop() {
        if let Some(object) = current.as_object() {
            // 宽松匹配：只要包含课程数组就认为是有效课表数据
            if object.get("classes").and_then(|v| v.as_array()).is_some()
                || object.get("data").and_then(|v| v.as_array()).is_some()
            {
                return Some(current);
            }
            if object
                .get("type")
                .and_then(|v| v.as_str())
                .is_some_and(|t| t == "timetableData")
            {
                return Some(current);
            }

            for key in [
                "data",
                "payload",
                "payloadText",
                "payload_text",
                "eventPayload",
                "event_payload",
                "message",
                "content",
                "body",
                "result",
            ] {
                if let Some(value) = object.get(key) {
                    stack.push(value.clone());
                    if let Some(text) = value.as_str() {
                        if let Ok(next) = serde_json::from_str::<serde_json::Value>(text) {
                            stack.push(next);
                        }
                    }
                }
            }
        } else if let Some(array) = current.as_array() {
            for value in array {
                stack.push(value.clone());
            }
        } else if let Some(text) = current.as_str() {
            if let Ok(next) = serde_json::from_str::<serde_json::Value>(text) {
                stack.push(next);
            }
        }
    }

    None
}

fn normalize_and_deduplicate(courses: &mut Vec<Course>) {
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    courses.retain(|course| {
        let key = (
            course.day,
            course.name.trim().to_string(),
            course.start.trim().to_string(),
            course.end.trim().to_string(),
            course.room.trim().to_string(),
            Course::normalize_week_type(Some(&course.week_type)),
        );
        seen.insert(key)
    });

    courses.sort_by(|a, b| {
        a.day
            .cmp(&b.day)
            .then_with(|| a.start.cmp(&b.start))
            .then_with(|| a.end.cmp(&b.end))
            .then_with(|| a.name.cmp(&b.name))
    });
}

pub fn get_cached_courses() -> Vec<Course> {
    with_state(|state| state.cached_courses.clone())
}

pub fn update_course(course: Course, index: Option<usize>) {
    with_state(|state| {
        if let Some(idx) = index {
            if idx < state.cached_courses.len() {
                state.cached_courses[idx] = course;
            }
        } else {
            state.cached_courses.push(course);
        }
        normalize_and_deduplicate(&mut state.cached_courses);
    });
}

pub fn delete_course(index: usize) {
    with_state(|state| {
        if index < state.cached_courses.len() {
            state.cached_courses.remove(index);
        }
    });
}

pub fn import_from_json(text: &str) -> Result<usize, String> {
    if let Some((mut courses, ab_tag)) = parse_timetable_data(text) {
        normalize_and_deduplicate(&mut courses);
        let count = courses.len();
        with_state(|state| {
            state.cached_courses = courses;
            state.cached_ab_tag = ab_tag;
            state.status = format!("已导入 {} 节课程", count);
        });
        Ok(count)
    } else {
        // Try parsing as simple array of courses if not timetableData format
        if let Ok(courses_arr) = serde_json::from_str::<Vec<Course>>(text) {
             let mut courses = courses_arr;
             normalize_and_deduplicate(&mut courses);
             let count = courses.len();
             with_state(|state| {
                state.cached_courses = courses;
                state.status = format!("已导入 {} 节课程", count);
             });
             Ok(count)
        } else {
            Err("无法解析课程数据，请确保格式正确".to_string())
        }
    }
}
