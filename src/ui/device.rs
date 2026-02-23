use crate::sync;

pub async fn check_connected_device() -> Result<String, String> {
    sync::first_connected_device_addr()
        .await
        .ok_or_else(|| "当前无已连接设备，请先在 AstroBox 连接手表".to_string())
}
