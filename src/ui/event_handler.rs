use crate::astrobox::psys_host::ui;
use crate::model::Course;
use crate::sync;
use crate::ui::build::{fill_edit_form_by_index, refresh_main_ui};
use crate::ui::state::{ui_state, CourseForm, TabType};

pub const EVENT_TAB_ADD: &str = "tab_add";
pub const EVENT_TAB_MANAGE: &str = "tab_manage";
pub const EVENT_TAB_IMPORT: &str = "tab_import";

pub const EVENT_ADD_COURSE: &str = "event_add_course";
pub const EVENT_PULL_FROM_WATCH: &str = "event_pull_from_watch";
pub const EVENT_PUSH_TO_WATCH: &str = "event_push_to_watch";
pub const EVENT_SELECT_COURSE: &str = "event_select_course";
pub const EVENT_SAVE_EDIT: &str = "event_save_edit";
pub const EVENT_DELETE_COURSE: &str = "event_delete_course";
pub const EVENT_IMPORT_PASTE: &str = "event_import_paste";

pub const INPUT_ADD_DAY: &str = "input_add_day";
pub const INPUT_ADD_NAME: &str = "input_add_name";
pub const INPUT_ADD_ROOM: &str = "input_add_room";
pub const INPUT_ADD_START: &str = "input_add_start";
pub const INPUT_ADD_END: &str = "input_add_end";
pub const INPUT_ADD_WEEK_TYPE: &str = "input_add_week_type";

pub const INPUT_EDIT_DAY: &str = "input_edit_day";
pub const INPUT_EDIT_NAME: &str = "input_edit_name";
pub const INPUT_EDIT_ROOM: &str = "input_edit_room";
pub const INPUT_EDIT_START: &str = "input_edit_start";
pub const INPUT_EDIT_END: &str = "input_edit_end";
pub const INPUT_EDIT_WEEK_TYPE: &str = "input_edit_week_type";

pub const INPUT_IMPORT_TEXT: &str = "input_import_text";

fn parse_input_value(payload: &str) -> String {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(payload) {
        json.get("value")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    } else {
        payload.to_string()
    }
}

fn resolve_event_id(event_id: &str, payload: &str) -> String {
    if !event_id.trim().is_empty() {
        return event_id.to_string();
    }
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(payload) {
        for key in ["id", "eventId", "event_id"] {
            if let Some(value) = json.get(key).and_then(|v| v.as_str()) {
                if !value.trim().is_empty() {
                    return value.to_string();
                }
            }
        }
    }
    String::new()
}

fn parse_form(form: &CourseForm) -> Result<Course, String> {
    let day: u8 = form
        .day
        .trim()
        .parse()
        .map_err(|_| "星期必须是 1-7".to_string())?;
    if !(1..=7).contains(&day) {
        return Err("星期必须是 1-7".to_string());
    }
    let name = form.name.trim().to_string();
    if name.is_empty() {
        return Err("课程名不能为空".to_string());
    }
    let start = form.start.trim().to_string();
    let end = form.end.trim().to_string();
    if start.is_empty() || end.is_empty() {
        return Err("开始/结束节次不能为空".to_string());
    }

    Ok(Course {
        day,
        name,
        room: form.room.trim().to_string(),
        start,
        end,
        week_type: Course::normalize_week_type(Some(form.week_type.as_str())),
    })
}

fn set_debug(evtype: ui::Event, event_id: &str, payload: &str) {
    let mut payload_text = payload.replace('\n', " ");
    if payload_text.len() > 80 {
        payload_text.truncate(80);
        payload_text.push_str("...");
    }
    let mut state = ui_state()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.last_event = format!("事件: {:?} | id: {} | payload: {}", evtype, event_id, payload_text);
}

pub fn set_status_message(message: String, is_error: bool) {
    let mut state = ui_state()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.message = Some((message, is_error));
}

fn handle_click(event_id: &str) {
    match event_id {
        EVENT_TAB_ADD => {
            let mut state = ui_state()
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.current_tab = TabType::Add;
        }
        EVENT_TAB_MANAGE => {
            let mut state = ui_state()
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.current_tab = TabType::Manage;
        }
        EVENT_TAB_IMPORT => {
            let mut state = ui_state()
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.current_tab = TabType::Import;
        }
        EVENT_ADD_COURSE => {
            let form = {
                let state = ui_state()
                    .read()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.add_form.clone()
            };
            match parse_form(&form) {
                Ok(course) => {
                    sync::update_course(course, None);
                    set_status_message("已添加课程，可继续添加或推送到手环".to_string(), false);
                }
                Err(e) => set_status_message(e, true),
            }
        }
        EVENT_PULL_FROM_WATCH => {
            set_status_message("正在向手环请求课程...".to_string(), false);
            refresh_main_ui();
            let ret = wit_bindgen::block_on(async { sync::request_timetable_from_device().await });
            match ret {
                Ok(_) => set_status_message("请求已发送，请等待手环返回数据".to_string(), false),
                Err(e) => set_status_message(format!("请求失败: {}", e), true),
            }
        }
        EVENT_PUSH_TO_WATCH => {
            set_status_message("正在推送课程到手环...".to_string(), false);
            refresh_main_ui();
            let ret = wit_bindgen::block_on(async { sync::sync_cached_to_device().await });
            match ret {
                Ok(_) => set_status_message("推送成功".to_string(), false),
                Err(e) => set_status_message(format!("推送失败: {}", e), true),
            }
        }
        EVENT_SAVE_EDIT => {
            let (form, selected_index) = {
                let state = ui_state()
                    .read()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                (state.edit_form.clone(), state.selected_index)
            };
            let Some(index) = selected_index else {
                set_status_message("请先在课程管理中选择要编辑的课程".to_string(), true);
                return;
            };
            match parse_form(&form) {
                Ok(course) => {
                    sync::update_course(course, Some(index));
                    set_status_message("课程修改已保存".to_string(), false);
                }
                Err(e) => set_status_message(e, true),
            }
        }
        EVENT_DELETE_COURSE => {
            let selected_index = {
                let state = ui_state()
                    .read()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.selected_index
            };
            let Some(index) = selected_index else {
                set_status_message("请先在课程管理中选择要删除的课程".to_string(), true);
                return;
            };
            sync::delete_course(index);
            {
                let mut state = ui_state()
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.selected_index = None;
            }
            set_status_message("课程已删除".to_string(), false);
        }
        EVENT_IMPORT_PASTE => {
            let text = {
                let state = ui_state()
                    .read()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.import_text.clone()
            };
            match sync::import_from_json(&text) {
                Ok(count) => set_status_message(format!("导入成功，共 {} 节课程", count), false),
                Err(e) => set_status_message(format!("导入失败: {}", e), true),
            }
        }
        _ => {}
    }
}

fn handle_change(event_id: &str, payload: &str) {
    let value = parse_input_value(payload);
    match event_id {
        INPUT_ADD_DAY => {
            let mut state = ui_state()
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.add_form.day = value;
        }
        INPUT_ADD_NAME => {
            let mut state = ui_state()
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.add_form.name = value;
        }
        INPUT_ADD_ROOM => {
            let mut state = ui_state()
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.add_form.room = value;
        }
        INPUT_ADD_START => {
            let mut state = ui_state()
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.add_form.start = value;
        }
        INPUT_ADD_END => {
            let mut state = ui_state()
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.add_form.end = value;
        }
        INPUT_ADD_WEEK_TYPE => {
            let mut state = ui_state()
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.add_form.week_type = value;
        }
        INPUT_EDIT_DAY => {
            let mut state = ui_state()
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.edit_form.day = value;
        }
        INPUT_EDIT_NAME => {
            let mut state = ui_state()
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.edit_form.name = value;
        }
        INPUT_EDIT_ROOM => {
            let mut state = ui_state()
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.edit_form.room = value;
        }
        INPUT_EDIT_START => {
            let mut state = ui_state()
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.edit_form.start = value;
        }
        INPUT_EDIT_END => {
            let mut state = ui_state()
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.edit_form.end = value;
        }
        INPUT_EDIT_WEEK_TYPE => {
            let mut state = ui_state()
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.edit_form.week_type = value;
        }
        INPUT_IMPORT_TEXT => {
            let mut state = ui_state()
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.import_text = value;
        }
        EVENT_SELECT_COURSE => {
            let idx_opt = value
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<usize>().ok())
                .and_then(|v| v.checked_sub(1));
            if let Some(idx) = idx_opt {
                fill_edit_form_by_index(idx);
            }
        }
        _ => {}
    }
}

pub fn ui_event_processor(evtype: ui::Event, event_id: &str, event_payload: &str) {
    let resolved_event_id = resolve_event_id(event_id, event_payload);
    let event_id_ref = if resolved_event_id.is_empty() {
        event_id
    } else {
        &resolved_event_id
    };

    set_debug(evtype, event_id_ref, event_payload);
    match evtype {
        ui::Event::Click | ui::Event::PointerUp => handle_click(event_id_ref),
        ui::Event::Change | ui::Event::Input => handle_change(event_id_ref, event_payload),
        _ => {}
    }
    refresh_main_ui();
}
