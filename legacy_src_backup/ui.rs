use std::sync::{Mutex, OnceLock};

use crate::astrobox::psys_host::{self, ui};
use crate::sync;

pub const UI_EVENT_SYNC_PULL: &str = "sync_pull_from_watch";
pub const UI_EVENT_SYNC_PUSH: &str = "sync_push_to_watch";
pub const UI_EVENT_SYNC_REBIND: &str = "sync_rebind_channel";

struct UiState {
    root_element_id: Option<String>,
}

static UI_STATE: OnceLock<Mutex<UiState>> = OnceLock::new();

fn ui_state() -> &'static Mutex<UiState> {
    UI_STATE.get_or_init(|| {
        Mutex::new(UiState {
            root_element_id: None,
        })
    })
}

fn current_root_element_id() -> Option<String> {
    let state = ui_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.root_element_id.clone()
}

fn set_root_element_id(element_id: &str) {
    let mut state = ui_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.root_element_id = Some(element_id.to_string());
}

fn sync_button(title: &str, event_id: &str, primary: bool) -> ui::Element {
    let mut button = ui::Element::new(ui::ElementType::Button, Some(title))
        .padding(10)
        .radius(8)
        .on(ui::Event::Click, event_id);

    if primary {
        button = button.bg("#2B5BE8").text_color("#FFFFFF");
    } else {
        button = button.bg("#F2F4F8").text_color("#1A1A1A");
    }

    button
}

fn build_main_ui() -> ui::Element {
    let snapshot = sync::snapshot();

    let status_text = format!("状态: {}", snapshot.status);
    let subscribed_text = if snapshot.subscribed {
        "通道: 已订阅"
    } else {
        "通道: 未订阅（自动重试中）"
    };
    let cache_text = format!("缓存课程: {} 节", snapshot.cached_course_count);
    let device_text = format!(
        "最近设备: {}",
        snapshot.last_device_addr.as_deref().unwrap_or("未连接")
    );
    let pkg_text = format!("包名: {}", sync::interconnect_pkg_name());

    let title = ui::Element::new(ui::ElementType::P, Some("VarClass 手表同步")).size(22);
    let status = ui::Element::new(ui::ElementType::P, Some(status_text.as_str())).size(15);
    let subscribed = ui::Element::new(ui::ElementType::P, Some(subscribed_text)).size(15);
    let cache = ui::Element::new(ui::ElementType::P, Some(cache_text.as_str())).size(15);
    let device = ui::Element::new(ui::ElementType::P, Some(device_text.as_str())).size(15);
    let pkg = ui::Element::new(ui::ElementType::P, Some(pkg_text.as_str())).size(15);

    let actions = ui::Element::new(ui::ElementType::Div, None)
        .child(
            ui::Element::new(ui::ElementType::Div, None)
                .margin_bottom(8)
                .child(sync_button("从手表拉取课表", UI_EVENT_SYNC_PULL, true)),
        )
        .child(
            ui::Element::new(ui::ElementType::Div, None)
                .margin_bottom(8)
                .child(sync_button("推送缓存到手表", UI_EVENT_SYNC_PUSH, false)),
        )
        .child(ui::Element::new(ui::ElementType::Div, None).child(sync_button(
            "重新订阅同步通道",
            UI_EVENT_SYNC_REBIND,
            false,
        )));

    ui::Element::new(ui::ElementType::Div, None)
        .padding(16)
        .child(title)
        .child(status)
        .child(subscribed)
        .child(cache)
        .child(device)
        .child(pkg)
        .child(actions)
}

pub fn render_main_ui(element_id: &str) {
    set_root_element_id(element_id);
    psys_host::ui::render(element_id, build_main_ui());
}

pub fn refresh_main_ui() {
    if let Some(root_element_id) = current_root_element_id() {
        psys_host::ui::render(&root_element_id, build_main_ui());
    }
}

pub fn render_card(card_id: &str) {
    let snapshot = sync::snapshot();
    let text = format!("{} | 课程 {} 节", snapshot.status, snapshot.cached_course_count);
    psys_host::ui::render_to_text_card(card_id, &text);
}

pub async fn ui_event_processor(evtype: ui::Event, event_id: &str, _event_payload: &str) {
    if evtype != ui::Event::Click {
        return;
    }

    let result = match event_id {
        UI_EVENT_SYNC_PULL => sync::request_timetable_from_device().await,
        UI_EVENT_SYNC_PUSH => sync::sync_cached_to_device().await,
        UI_EVENT_SYNC_REBIND => sync::bootstrap_sync().await,
        _ => return,
    };

    if let Err(err) = result {
        sync::set_status(format!("同步操作失败: {}", err));
    }

    refresh_main_ui();
}
