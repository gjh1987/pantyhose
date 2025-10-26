// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn exit_app() {
    std::process::exit(0);
}

mod server_config;
mod server_manager;
mod build_server;
mod misc;
mod client;
mod proto_reader;

use server_config::{parse_server_config, save_server_path, load_server_path, save_startup_config, load_startup_config, get_app_config, save_message_templates, load_message_templates};
use server_manager::{init_process_manager, start_server, stop_server, get_server_status, get_all_server_status, refresh_all_server_status, get_server_logs, clear_server_memory_logs, clear_server_logs, get_server_info, restart_server, get_all_servers_info};
use build_server::{execute_build, check_build_script, start_build_server, get_build_logs, is_build_running, stop_build};
use misc::{kill_all_servers};
use proto_reader::{read_proto_files, read_proto_file};
use client::{
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 初始化事件发送器的 app handle
            client::event_emitter::init_app_handle(app.handle().clone());
            // 初始化进程管理器
            init_process_manager(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet, 
            parse_server_config, 
            save_server_path, 
            load_server_path, 
            save_startup_config,
            load_startup_config,
            get_app_config,
            save_message_templates,
            load_message_templates,
            init_process_manager,
            start_server, 
            stop_server, 
            get_server_status, 
            get_all_server_status,
            refresh_all_server_status,
            get_server_logs, 
            clear_server_memory_logs,
            clear_server_logs, 
            get_server_info, 
            restart_server, 
            get_all_servers_info,
            execute_build,
            check_build_script,
            start_build_server,
            get_build_logs,
            is_build_running,
            stop_build,
            kill_all_servers,
            exit_app,
            // 客户端管理命令
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
            is_client_connected,
            read_proto_files,
            read_proto_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
