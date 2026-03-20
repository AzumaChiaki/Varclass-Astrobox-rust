//! Var 课程表同步器 - AstroBox 插件入口
//!
//! 实现 AstroBox v2 插件接口，处理宿主事件、UI 渲染与课表同步逻辑。

use wit_bindgen::FutureReader;
use serde_json::Value;

use crate::exports::astrobox::psys_plugin::{
    event::{self, EventType},
    lifecycle,
};

pub mod class_island;
pub mod cses;
pub mod logger;
pub mod model;
pub mod sync;
pub mod ui;
pub mod wakeup;

wit_bindgen::generate!({
    path: "wit",
    world: "psys-world",
    generate_all,
});

struct MyPlugin;

/// 从事件 payload 中提取可读文本（兼容 payloadText / payload 等字段）
fn extract_payload_text(payload: &str) -> String {
    if let Ok(json) = serde_json::from_str::<Value>(payload) {
        if let Some(text) = json.get("payloadText").and_then(|v| v.as_str()) {
            return text.to_string();
        }
        if let Some(payload_value) = json.get("payload") {
            if let Some(text) = payload_value.as_str() {
                return text.to_string();
            }
            return payload_value.to_string();
        }
    }
    payload.to_string()
}

impl event::Guest for MyPlugin {
    fn on_event(event_type: EventType, event_payload: _rt::String) -> FutureReader<String> {
        let (writer, reader) = wit_future::new::<String>(|| "".to_string());

        match event_type {
            EventType::InterconnectMessage => {
                if let Err(e) = sync::handle_interconnect_message(&event_payload) {
                    logger::warn(format!("handle_interconnect_message error: {}", e));
                    ui::set_status_message(format!("接收手环课程失败: {}", e), true);
                    ui::refresh_main_ui();
                } else {
                    ui::set_status_message("已获取手环课程，可在“课程管理”中预览和编辑".to_string(), false);
                    ui::refresh_main_ui();
                }
            }
            EventType::Timer => {
                let payload = extract_payload_text(&event_payload);
                logger::info(format!("timer event payload={}", payload));
            }
            _ => {}
        }

        logger::info(format!(
            "host.on_event -> type={:?}, payload={}",
            event_type, event_payload
        ));

        wit_bindgen::spawn(async move {
            let _ = writer.write("".to_string()).await;
        });

        reader
    }

    fn on_ui_event(
        event_id: _rt::String,
        event: event::Event,
        event_payload: _rt::String,
    ) -> FutureReader<_rt::String> {
        let (writer, reader) = wit_future::new::<String>(|| "".to_string());

        logger::info(format!(
            "host.on_ui_event -> event={:?}, id={}, payload={}",
            event, event_id, event_payload
        ));

        ui::ui_event_processor(event, &event_id, &event_payload);

        wit_bindgen::spawn(async move {
            let _ = writer.write("ok".to_string()).await;
        });

        reader
    }

    fn on_ui_render(element_id: _rt::String) -> FutureReader<()> {
        let (writer, reader) = wit_future::new::<()>(|| ());

        ui::render_main_ui(&element_id);

        wit_bindgen::spawn(async move {
            let _ = writer.write(()).await;
        });

        reader
    }

    fn on_card_render(_card_id: _rt::String) -> FutureReader<()> {
        let (writer, reader) = wit_future::new::<()>(|| ());

        wit_bindgen::spawn(async move {
            let _ = writer.write(()).await;
        });

        reader
    }
}

impl lifecycle::Guest for MyPlugin {
    fn on_load() {
        logger::init();
        logger::info("plugin loaded");
        wit_bindgen::spawn(async move {
            logger::info("on_load register flow: scanning connected devices");
            if let Err(e) = sync::bootstrap_sync().await {
                logger::warn(format!("bootstrap_sync failed: {}", e));
            }
        });
    }
}

export!(MyPlugin);
