// 核心模块
pub mod client;
pub mod client_manager_v2;
pub mod event_emitter;
pub mod dynamic_buffer;

// 支持模块
pub mod tcp_client;
pub mod websocket_client;

// 导出新的客户端管理命令
pub use client_manager_v2::{
    create_client,
    delete_client,
    switch_client_type,
    connect_client,
    disconnect_client,
    send_client_message,
    get_client_info,
    get_all_clients,
    get_client_logs,
    clear_client_logs,
    is_client_connected
};