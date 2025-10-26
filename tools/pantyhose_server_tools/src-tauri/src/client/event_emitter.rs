use tauri::Manager;
use once_cell::sync::Lazy;
use std::sync::Mutex;

// 全局 App Handle 存储
static APP_HANDLE: Lazy<Mutex<Option<tauri::AppHandle>>> = Lazy::new(|| Mutex::new(None));

// 初始化 App Handle
pub fn init_app_handle(app_handle: tauri::AppHandle) {
    let mut handle = APP_HANDLE.lock().unwrap();
    *handle = Some(app_handle);
}

// 直接执行JavaScript代码
pub fn emit_to_frontend_raw(client_id: i32, js_code: String) {
    let handle = APP_HANDLE.lock().unwrap();
    
    if let Some(app) = handle.as_ref() {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.eval(&js_code);
            println!("发送事件到前端: {}", js_code);
        }
    } else {
        println!("App handle 未初始化，无法发送事件");
    }
}

// 发送事件到前端
pub fn emit_to_frontend(client_id: i32, event_type: &str, data: Option<String>) {
    let handle = APP_HANDLE.lock().unwrap();
    
    if let Some(app) = handle.as_ref() {
        // 获取所有窗口
        if let Some(window) = app.get_webview_window("main") {
            // 构建 JavaScript 代码
            let js_code = match event_type {
                "connected" => {
                    format!("if (window.NetClientManager) {{ window.NetClientManager.onConnect({}); }}", client_id)
                }
                "disconnected" => {
                    format!("if (window.NetClientManager) {{ window.NetClientManager.onDisconnect({}); }}", client_id)
                }
                "message" => {
                    if let Some(msg) = data {
                        // 转义消息中的特殊字符
                        let escaped_msg = msg
                            .replace('\\', "\\\\")
                            .replace('"', "\\\"")
                            .replace('\n', "\\n")
                            .replace('\r', "\\r")
                            .replace('\t', "\\t");
                        format!(
                            "if (window.NetClientManager) {{ window.NetClientManager.onMessage({}, \"{}\"); }}", 
                            client_id, 
                            escaped_msg
                        )
                    } else {
                        format!("if (window.NetClientManager) {{ window.NetClientManager.onMessage({}, \"\"); }}", client_id)
                    }
                }
                "error" => {
                    if let Some(err) = data {
                        // 转义错误消息中的特殊字符
                        let escaped_err = err
                            .replace('\\', "\\\\")
                            .replace('"', "\\\"")
                            .replace('\n', "\\n")
                            .replace('\r', "\\r")
                            .replace('\t', "\\t");
                        format!(
                            "if (window.NetClientManager) {{ window.NetClientManager.onError({}, \"{}\"); }}", 
                            client_id, 
                            escaped_err
                        )
                    } else {
                        format!("if (window.NetClientManager) {{ window.NetClientManager.onError({}, \"未知错误\"); }}", client_id)
                    }
                }
                _ => {
                    println!("未知的事件类型: {}", event_type);
                    return;
                }
            };
            
            // 执行 JavaScript
            let _ = window.eval(&js_code);
            println!("发送事件到前端: {} - {}", event_type, js_code);
        }
    } else {
        println!("App handle 未初始化，无法发送事件");
    }
}

// 客户端事件发送器
#[derive(Clone)]
pub struct ClientEventEmitter {
    client_id: i32,
}

impl ClientEventEmitter {
    pub fn new(client_id: i32) -> Self {
        Self { client_id }
    }
    
    pub fn on_connect(&self) {
        emit_to_frontend(self.client_id, "connected", None);
    }
    
    pub fn on_disconnect(&self) {
        emit_to_frontend(self.client_id, "disconnected", None);
    }
    
    pub fn on_message(&self, message: String) {
        emit_to_frontend(self.client_id, "message", Some(message));
    }
    
    pub fn on_binary_message(&self, msg_id: u16, byte_len: usize, bytes: Vec<u8>) {
        // 转换为JavaScript数组格式
        let bytes_str = format!("[{}]", bytes.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(","));
        
        // 构建调用参数
        let js_code = format!(
            "if (window.NetClientManager) {{ window.NetClientManager.onMessage({}, {}, {}, {}); }}", 
            self.client_id, 
            msg_id,
            byte_len,
            bytes_str
        );
        
        emit_to_frontend_raw(self.client_id, js_code);
    }
    
    pub fn on_error(&self, error: String) {
        emit_to_frontend(self.client_id, "error", Some(error));
    }
}