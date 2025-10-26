use std::process::{Command, Child, Stdio};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::path::Path;
use std::io::{BufRead, BufReader};
use serde::{Serialize, Deserialize};
use std::fs;
use tauri::Manager;

/// 服务器进程信息
#[derive(Debug, Clone)]
pub struct ServerProcess {
    pub server_id: String,
    pub logs: Arc<Mutex<Vec<String>>>,
    pub last_read_index: Arc<Mutex<usize>>,
    pub start_time: std::time::SystemTime,
}

/// 日志条目
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

/// 全局进程管理器
pub struct GlobalProcessManager {
    processes: Arc<Mutex<HashMap<String, Child>>>,
    process_info: Arc<Mutex<HashMap<String, ServerProcess>>>,
    app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
    monitor_running: Arc<Mutex<bool>>,
}

impl GlobalProcessManager {
    fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            process_info: Arc::new(Mutex::new(HashMap::new())),
            app_handle: Arc::new(Mutex::new(None)),
            monitor_running: Arc::new(Mutex::new(false)),
        }
    }

    /// 初始化 app handle
    pub fn init_app_handle(&self, handle: tauri::AppHandle) {
        let mut app_handle = self.app_handle.lock().unwrap();
        *app_handle = Some(handle);
        
        // 启动全局监控线程
        if !*self.monitor_running.lock().unwrap() {
            self.start_monitor_thread();
        }
    }

    /// 启动全局监控线程
    fn start_monitor_thread(&self) {
        let processes = Arc::clone(&self.processes);
        let process_info = Arc::clone(&self.process_info);
        let app_handle = Arc::clone(&self.app_handle);
        let monitor_running = Arc::clone(&self.monitor_running);
        
        *monitor_running.lock().unwrap() = true;
        
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(500)); // 每500ms检查一次
                
                if !*monitor_running.lock().unwrap() {
                    break;
                }
                
                let mut stopped_servers = Vec::new();
                
                // 检查所有进程状态
                if let Ok(mut procs) = processes.lock() {
                    for (server_id, child) in procs.iter_mut() {
                        match child.try_wait() {
                            Ok(Some(exit_status)) => {
                                // 进程已退出
                                println!("服务器 {} 进程已退出，退出状态: {:?}", server_id, exit_status);
                                stopped_servers.push(server_id.clone());
                                
                                // 添加退出日志
                                if let Ok(info_map) = process_info.lock() {
                                    if let Some(server_process) = info_map.get(server_id) {
                                        if let Ok(mut logs) = server_process.logs.lock() {
                                            logs.push(format!("[SYSTEM] 进程已退出，退出码: {:?}", exit_status.code()));
                                        }
                                    }
                                }
                            }
                            Ok(None) => {
                                // 进程仍在运行
                            }
                            Err(e) => {
                                println!("检查服务器 {} 状态时出错: {}", server_id, e);
                                stopped_servers.push(server_id.clone());
                            }
                        }
                    }
                    
                    // 移除已停止的进程
                    for server_id in &stopped_servers {
                        procs.remove(server_id);
                        
                        // 通知前端
                        if let Ok(handle_guard) = app_handle.lock() {
                            if let Some(handle) = handle_guard.as_ref() {
                                if let Some(window) = handle.get_webview_window("main") {
                                    let eval_script = format!("window.ServerManager && window.ServerManager.onServerStop('{}');", server_id);
                                    let _ = window.eval(&eval_script);
                                }
                            }
                        }
                    }
                }
                
                // 如果有状态变化，通知前端更新所有状态
                if !stopped_servers.is_empty() {
                    if let Ok(handle_guard) = app_handle.lock() {
                        if let Some(handle) = handle_guard.as_ref() {
                            Self::notify_all_status(handle, &processes, &process_info);
                        }
                    }
                }
            }
            
            println!("全局进程监控线程已退出");
        });
    }

    /// 添加进程
    pub fn add_process(&self, server_id: String, mut child: Child, server_process: ServerProcess) {
        // 获取stdout和stderr用于日志捕获
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        
        // 保存进程
        self.processes.lock().unwrap().insert(server_id.clone(), child);
        self.process_info.lock().unwrap().insert(server_id.clone(), server_process.clone());
        
        // 启动日志捕获线程
        if let Some(stdout) = stdout {
            let server_id_stdout = server_id.clone();
            let logs_stdout = server_process.logs.clone();
            thread::spawn(move || {
                capture_output_logs(stdout, server_id_stdout, logs_stdout, "INFO");
            });
        }
        
        if let Some(stderr) = stderr {
            let server_id_stderr = server_id.clone();
            let logs_stderr = server_process.logs.clone();
            thread::spawn(move || {
                capture_output_logs(stderr, server_id_stderr, logs_stderr, "ERROR");
            });
        }
    }

    /// 停止进程
    pub fn stop_process(&self, server_id: &str) -> Result<bool, String> {
        let mut processes = self.processes.lock().map_err(|e| format!("锁定进程管理器失败: {}", e))?;
        
        if let Some(mut child) = processes.remove(server_id) {
            match child.kill() {
                Ok(_) => {
                    println!("成功停止服务器 {}", server_id);
                    let _ = child.wait(); // 等待进程完全退出
                    
                    // 添加停止日志
                    if let Ok(info_map) = self.process_info.lock() {
                        if let Some(server_process) = info_map.get(server_id) {
                            if let Ok(mut logs) = server_process.logs.lock() {
                                logs.push("[SYSTEM] 服务器已停止".to_string());
                            }
                        }
                    }
                    
                    Ok(true)
                }
                Err(e) => {
                    Err(format!("停止服务器失败: {}", e))
                }
            }
        } else {
            Err(format!("服务器 {} 未运行或不存在", server_id))
        }
    }

    /// 检查进程是否运行
    pub fn is_running(&self, server_id: &str) -> bool {
        if let Ok(mut processes) = self.processes.lock() {
            if let Some(child) = processes.get_mut(server_id) {
                match child.try_wait() {
                    Ok(None) => true, // 进程仍在运行
                    _ => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    /// 获取所有进程状态
    pub fn get_all_status(&self) -> HashMap<String, bool> {
        let mut status_map = HashMap::new();
        
        if let Ok(mut processes) = self.processes.lock() {
            for (server_id, child) in processes.iter_mut() {
                let is_running = match child.try_wait() {
                    Ok(None) => true,
                    _ => false,
                };
                status_map.insert(server_id.clone(), is_running);
            }
        }
        
        status_map
    }

    /// 获取进程信息
    pub fn get_process_info(&self, server_id: &str) -> Option<ServerProcess> {
        self.process_info.lock().ok()?.get(server_id).cloned()
    }

    /// 通知前端所有状态
    fn notify_all_status(app_handle: &tauri::AppHandle, processes: &Arc<Mutex<HashMap<String, Child>>>, _process_info: &Arc<Mutex<HashMap<String, ServerProcess>>>) {
        let mut status_map = HashMap::new();
        
        if let Ok(mut procs) = processes.lock() {
            for (server_id, child) in procs.iter_mut() {
                let is_running = match child.try_wait() {
                    Ok(None) => true,
                    _ => false,
                };
                status_map.insert(server_id.clone(), is_running);
            }
        }
        
        if let Some(window) = app_handle.get_webview_window("main") {
            let status_json = serde_json::to_string(&status_map).unwrap_or_else(|_| "{}".to_string());
            let eval_script = format!("window.ServerManager && window.ServerManager.updateAllStatus({});", status_json);
            let _ = window.eval(&eval_script);
        }
    }
}

// 全局进程管理器实例
static PROCESS_MANAGER: std::sync::OnceLock<GlobalProcessManager> = std::sync::OnceLock::new();

fn get_process_manager() -> &'static GlobalProcessManager {
    PROCESS_MANAGER.get_or_init(|| GlobalProcessManager::new())
}

/// 初始化进程管理器
#[tauri::command]
pub fn init_process_manager(app_handle: tauri::AppHandle) {
    get_process_manager().init_app_handle(app_handle);
}

/// 启动服务器
#[tauri::command]
pub fn start_server(app_handle: tauri::AppHandle, server_path: &str, server_id: &str, executable_name: &str, config_file_name: Option<&str>) -> Result<bool, String> {
    // 确保进程管理器已初始化
    get_process_manager().init_app_handle(app_handle.clone());
    
    // 检查是否已经在运行
    if get_process_manager().is_running(server_id) {
        return Ok(true);
    }
    
    let exe_name = executable_name;
    let config_name = config_file_name.unwrap_or("config.xml");
    
    let pantyhose_exe = Path::new(server_path).join(exe_name);
    let config_xml = Path::new(server_path).join(config_name);
    
    // 检查文件是否存在
    if !pantyhose_exe.exists() {
        return Err(format!("{} 不存在: {:?}", exe_name, pantyhose_exe));
    }
    
    if !config_xml.exists() {
        return Err(format!("{} 不存在: {:?}", config_name, config_xml));
    }
    
    // 启动进程
    let mut cmd = Command::new(&pantyhose_exe);
    cmd.arg(config_name)
       .arg(server_id)
       .current_dir(server_path)
       .stdout(Stdio::piped())
       .stderr(Stdio::piped());
    
    // Windows平台特定：隐藏窗口
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    
    let child = cmd.spawn().map_err(|e| format!("启动服务器失败: {}", e))?;
    
    println!("启动服务器 {} 成功, PID: {:?}", server_id, child.id());
    
    // 创建服务器进程信息
    let server_process = ServerProcess {
        server_id: server_id.to_string(),
        logs: Arc::new(Mutex::new(Vec::new())),
        last_read_index: Arc::new(Mutex::new(0)),
        start_time: std::time::SystemTime::now(),
    };
    
    // 添加到全局管理器
    get_process_manager().add_process(server_id.to_string(), child, server_process);
    
    // 通知前端服务器已启动
    if let Some(window) = app_handle.get_webview_window("main") {
        let eval_script = format!("window.ServerManager && window.ServerManager.onServerStart('{}');", server_id);
        let _ = window.eval(&eval_script);
    }
    
    Ok(true)
}

/// 停止服务器
#[tauri::command]
pub fn stop_server(app_handle: tauri::AppHandle, server_id: &str) -> Result<bool, String> {
    let result = get_process_manager().stop_process(server_id)?;
    
    // 通知前端服务器已停止
    if let Some(window) = app_handle.get_webview_window("main") {
        let eval_script = format!("window.ServerManager && window.ServerManager.onServerStop('{}');", server_id);
        let _ = window.eval(&eval_script);
    }
    
    Ok(result)
}

/// 获取服务器状态
#[tauri::command]
pub fn get_server_status(server_id: &str) -> Result<bool, String> {
    Ok(get_process_manager().is_running(server_id))
}

/// 获取所有服务器状态
#[tauri::command]
pub fn get_all_server_status() -> Result<HashMap<String, bool>, String> {
    Ok(get_process_manager().get_all_status())
}

/// 刷新并通知所有服务器状态
#[tauri::command]
pub fn refresh_all_server_status(app_handle: tauri::AppHandle) -> Result<(), String> {
    let status_map = get_process_manager().get_all_status();
    
    // 通知前端更新所有状态
    if let Some(window) = app_handle.get_webview_window("main") {
        let status_json = serde_json::to_string(&status_map).unwrap_or_else(|_| "{}".to_string());
        let eval_script = format!("window.ServerManager && window.ServerManager.updateAllStatus({});", status_json);
        let _ = window.eval(&eval_script);
    }
    
    Ok(())
}

/// 获取服务器日志（只返回新增的日志）
#[tauri::command]
pub fn get_server_logs(server_id: &str, _limit: Option<usize>) -> Result<Vec<String>, String> {
    if let Some(server_process) = get_process_manager().get_process_info(server_id) {
        if let Ok(logs) = server_process.logs.lock() {
            if let Ok(mut last_index) = server_process.last_read_index.lock() {
                // 获取从上次读取位置开始的新日志
                let new_logs = if *last_index < logs.len() {
                    logs[*last_index..].to_vec()
                } else {
                    Vec::new()
                };
                
                // 更新读取位置
                *last_index = logs.len();
                
                Ok(new_logs)
            } else {
                Err("无法获取日志读取位置".to_string())
            }
        } else {
            Err("无法获取服务器日志".to_string())
        }
    } else {
        Ok(Vec::new()) // 服务器不存在时返回空数组
    }
}

/// 清空服务器内存日志
#[tauri::command]
pub fn clear_server_memory_logs(server_id: &str) -> Result<bool, String> {
    if let Some(server_process) = get_process_manager().get_process_info(server_id) {
        if let Ok(mut logs) = server_process.logs.lock() {
            // 记录当前日志数量作为新的读取起点
            if let Ok(mut last_index) = server_process.last_read_index.lock() {
                *last_index = logs.len();
            }
            logs.clear();
            logs.push("[SYSTEM] 日志已清空".to_string());
            Ok(true)
        } else {
            Err("无法清空服务器日志".to_string())
        }
    } else {
        Err(format!("服务器 {} 不存在", server_id))
    }
}

/// 获取服务器信息
#[tauri::command]
pub fn get_server_info(server_id: &str) -> Result<serde_json::Value, String> {
    if let Some(server_process) = get_process_manager().get_process_info(server_id) {
        let is_running = get_process_manager().is_running(server_id);
        
        let uptime = if is_running {
            server_process.start_time.elapsed()
                .map(|duration| duration.as_secs())
                .unwrap_or(0)
        } else {
            0
        };
        
        let info = serde_json::json!({
            "server_id": server_process.server_id,
            "is_running": is_running,
            "start_time": server_process.start_time
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            "uptime_seconds": uptime,
            "log_count": server_process.logs.lock().map(|logs| logs.len()).unwrap_or(0)
        });
        
        Ok(info)
    } else {
        Err(format!("服务器 {} 不存在", server_id))
    }
}

/// 捕获进程输出日志
fn capture_output_logs<R: std::io::Read + Send + 'static>(
    reader: R,
    server_id: String,
    logs: Arc<Mutex<Vec<String>>>,
    log_level: &str,
) {
    let buf_reader = BufReader::new(reader);
    let _log_level = log_level.to_string();
    
    for line_result in buf_reader.lines() {
        match line_result {
            Ok(line) => {
                if !line.trim().is_empty() {
                    // 直接使用原始日志，不添加额外的时间戳和级别
                    if let Ok(mut logs_guard) = logs.lock() {
                        logs_guard.push(line.trim().to_string());
                    }
                }
            }
            Err(e) => {
                println!("读取服务器 {} 输出时出错: {}", server_id, e);
                break;
            }
        }
    }
    
    println!("服务器 {} 的输出流已关闭", server_id);
}

/// 重启服务器
#[tauri::command]
pub fn restart_server(app_handle: tauri::AppHandle, server_path: &str, server_id: &str, executable_name: &str, config_file_name: Option<&str>) -> Result<bool, String> {
    // 先停止服务器
    let _ = stop_server(app_handle.clone(), server_id);
    
    // 等待一秒确保进程完全停止
    thread::sleep(Duration::from_secs(1));
    
    // 重新启动服务器
    start_server(app_handle, server_path, server_id, executable_name, config_file_name)
}

/// 获取所有服务器的详细信息
#[tauri::command]
pub fn get_all_servers_info() -> Result<HashMap<String, serde_json::Value>, String> {
    let mut servers_info = HashMap::new();
    let status_map = get_process_manager().get_all_status();
    
    for (server_id, is_running) in status_map {
        if let Some(server_process) = get_process_manager().get_process_info(&server_id) {
            let uptime = if is_running {
                server_process.start_time.elapsed()
                    .map(|duration| duration.as_secs())
                    .unwrap_or(0)
            } else {
                0
            };
            
            let info = serde_json::json!({
                "server_id": server_process.server_id,
                "is_running": is_running,
                "start_time": server_process.start_time
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                "uptime_seconds": uptime,
                "log_count": server_process.logs.lock().map(|logs| logs.len()).unwrap_or(0)
            });
            
            servers_info.insert(server_id, info);
        }
    }
    
    Ok(servers_info)
}

/// 清理服务器日志文件
#[tauri::command]
pub fn clear_server_logs(log_path: &str) -> Result<(), String> {
    let path = Path::new(log_path);
    
    if !path.exists() {
        return Ok(()); // 如果日志目录不存在，直接返回成功
    }
    
    if !path.is_dir() {
        return Err(format!("路径不是目录: {:?}", path));
    }
    
    // 遍历目录中的所有文件
    let entries = fs::read_dir(path)
        .map_err(|e| format!("读取目录失败: {}", e))?;
    
    for entry in entries {
        let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
        let file_path = entry.path();
        
        // 只删除日志文件（.log, .txt 等）
        if file_path.is_file() {
            let extension = file_path.extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("");
            
            if extension == "log" || extension == "txt" {
                fs::remove_file(&file_path)
                    .map_err(|e| format!("删除文件失败 {:?}: {}", file_path, e))?;
            }
        }
    }
    
    Ok(())
}