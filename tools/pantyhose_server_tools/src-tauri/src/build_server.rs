use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::process::{Command, Stdio};
use std::io::{BufReader, BufRead};
use std::path::Path;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
fn decode_gbk_to_utf8(bytes: &[u8]) -> String {
    // 尝试用 GBK 解码，如果失败则用 UTF-8
    match encoding_rs::GBK.decode(bytes) {
        (cow, _, false) => cow.into_owned(),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

#[cfg(not(windows))]
fn decode_gbk_to_utf8(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

// 构建进程信息
pub struct BuildProcess {
    pub is_running: Arc<AtomicBool>,
    pub logs: Arc<Mutex<Vec<String>>>,
    pub last_read_index: Arc<Mutex<usize>>,
}

// 存储构建进程和日志 - 使用静态 RwLock
static BUILD_PROCESS: RwLock<Option<Arc<BuildProcess>>> = RwLock::new(None);

/// 开始编译服务器（后台执行）
#[tauri::command]
pub fn start_build_server(mode: &str, clean: bool, executable_name: &str) -> Result<bool, String> {
    // 检查是否已有编译进程在运行
    {
        let build_guard = BUILD_PROCESS.read().map_err(|e| format!("读取构建进程状态失败: {}", e))?;
        if let Some(ref process) = *build_guard {
            if process.is_running.load(Ordering::SeqCst) {
                return Err("已有编译进程在运行".to_string());
            }
        }
    }
    
    // 从配置文件获取 server_path
    let config_path = crate::server_config::get_config_file_path()
        .map_err(|e| format!("获取配置文件路径失败: {}", e))?;
    
    let project_root = if config_path.exists() {
        // 读取配置文件
        let config_content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("读取配置文件失败: {}", e))?;
        
        let config: serde_json::Value = serde_json::from_str(&config_content)
            .map_err(|e| format!("解析配置文件失败: {}", e))?;
        
        if let Some(server_path) = config.get("server_path").and_then(|v| v.as_str()) {
            // server_path 的上一级目录就是项目根目录
            let path = Path::new(server_path);
            if let Some(parent) = path.parent() {
                parent.to_path_buf()
            } else {
                return Err("无法从 server_path 获取项目根目录".to_string());
            }
        } else {
            return Err("配置文件中没有 server_path".to_string());
        }
    } else {
        return Err("配置文件不存在，请先在设置中配置服务器路径".to_string());
    };
    
    // 创建新的构建进程记录
    let build_process = Arc::new(BuildProcess {
        is_running: Arc::new(AtomicBool::new(true)),
        logs: Arc::new(Mutex::new(Vec::new())),
        last_read_index: Arc::new(Mutex::new(0)),
    });
    
    let logs = build_process.logs.clone();
    let is_running = build_process.is_running.clone();
    let mode = mode.to_string();
    let executable_name = executable_name.to_string();
    
    // 在后台线程中执行编译
    std::thread::spawn(move || {
        // 如果需要清理
        if clean {
            if let Ok(mut logs_guard) = logs.lock() {
                logs_guard.push("[BUILD] 执行 cargo clean...".to_string());
            }
            
            let mut clean_cmd = Command::new("cargo");
            clean_cmd.arg("clean")
                .current_dir(&project_root);
            
            #[cfg(windows)]
            {
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                clean_cmd.creation_flags(CREATE_NO_WINDOW);
            }
            
            match clean_cmd.output() {
                Ok(output) => {
                    if output.status.success() {
                        if let Ok(mut logs_guard) = logs.lock() {
                            logs_guard.push("[BUILD] 清理完成".to_string());
                        }
                    } else {
                        let err_msg = format!("[ERROR] cargo clean 失败: {}", 
                            decode_gbk_to_utf8(&output.stderr));
                        if let Ok(mut logs_guard) = logs.lock() {
                            logs_guard.push(err_msg);
                        }
                        is_running.store(false, Ordering::SeqCst);
                        return;
                    }
                }
                Err(e) => {
                    let err_msg = format!("[ERROR] 执行 cargo clean 失败: {}", e);
                    if let Ok(mut logs_guard) = logs.lock() {
                        logs_guard.push(err_msg);
                    }
                    is_running.store(false, Ordering::SeqCst);
                    return;
                }
            }
        }
        
        // 开始编译
        let start_msg = format!("[BUILD] 开始编译 ({} 模式)...", mode);
        if let Ok(mut logs_guard) = logs.lock() {
            logs_guard.push(start_msg);
        }
        
        let mut build_cmd = Command::new("cargo");
        build_cmd.arg("build");
        
        if mode == "release" {
            build_cmd.arg("--release");
        }
        
        build_cmd.current_dir(&project_root);
        build_cmd.stdout(Stdio::piped());
        build_cmd.stderr(Stdio::piped());
        
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            build_cmd.creation_flags(CREATE_NO_WINDOW);
        }
        
        match build_cmd.spawn() {
            Ok(mut child) => {
                // 读取stdout
                if let Some(stdout) = child.stdout.take() {
                    let logs_clone = logs.clone();
                    std::thread::spawn(move || {
                        let reader = BufReader::new(stdout);
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                if let Ok(mut logs_guard) = logs_clone.lock() {
                                    logs_guard.push(line);
                                }
                            }
                        }
                    });
                }
                
                // 读取stderr
                if let Some(stderr) = child.stderr.take() {
                    let logs_clone = logs.clone();
                    std::thread::spawn(move || {
                        let reader = BufReader::new(stderr);
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                if let Ok(mut logs_guard) = logs_clone.lock() {
                                    logs_guard.push(line);
                                }
                            }
                        }
                    });
                }
                
                // 等待进程结束
                match child.wait() {
                    Ok(status) => {
                        if status.success() {
                            let target_dir = if mode == "release" {
                                "bin/target/release"
                            } else {
                                "bin/target/debug"
                            };
                            
                            // 复制编译结果到 server_path
                            let exe_source = project_root.join(target_dir).join(&executable_name);
                            
                            // 重新读取配置获取 server_path
                            if let Ok(config_path) = crate::server_config::get_config_file_path() {
                                if let Ok(config_content) = std::fs::read_to_string(&config_path) {
                                    if let Ok(config) = serde_json::from_str::<serde_json::Value>(&config_content) {
                                        if let Some(server_path) = config.get("server_path").and_then(|v| v.as_str()) {
                                            let exe_dest = Path::new(server_path).join(&executable_name);
                                            
                                            // 尝试复制文件
                                            let mut copy_result = std::fs::copy(&exe_source, &exe_dest);
                                            
                                            // 如果复制失败，可能是文件被占用
                                            if copy_result.is_err() {
                                                if let Ok(mut logs_guard) = logs.lock() {
                                                    logs_guard.push("[BUILD] 检测到目标文件可能被占用，尝试终止进程...".to_string());
                                                }
                                                
                                                // 尝试终止占用进程
                                                #[cfg(windows)]
                                                {
                                                    // 使用 taskkill 命令终止进程
                                                    let mut kill_cmd = Command::new("taskkill");
                                                    kill_cmd.args(&["/F", "/IM", &executable_name]);
                                                    
                                                    const CREATE_NO_WINDOW: u32 = 0x08000000;
                                                    kill_cmd.creation_flags(CREATE_NO_WINDOW);
                                                    
                                                    let _ = kill_cmd.output();
                                                    
                                                    // 等待一下让进程完全终止
                                                    std::thread::sleep(std::time::Duration::from_millis(500));
                                                }
                                                
                                                // 再次尝试复制
                                                copy_result = std::fs::copy(&exe_source, &exe_dest);
                                            }
                                            
                                            // 处理复制结果
                                            match copy_result {
                                                Ok(_) => {
                                                    let success_msg = format!(
                                                        "[BUILD] 编译成功完成！\n[BUILD] 可执行文件已复制到: {}",
                                                        exe_dest.display()
                                                    );
                                                    if let Ok(mut logs_guard) = logs.lock() {
                                                        logs_guard.push(success_msg);
                                                    }
                                                }
                                                Err(e) => {
                                                    let warn_msg = format!(
                                                        "[BUILD] 编译成功，但复制到服务器目录失败: {}\n[BUILD] 源文件位置: {}",
                                                        e, exe_source.display()
                                                    );
                                                    if let Ok(mut logs_guard) = logs.lock() {
                                                        logs_guard.push(warn_msg);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            let err_msg = format!("[ERROR] 编译失败，退出码: {:?}", status.code());
                            if let Ok(mut logs_guard) = logs.lock() {
                                logs_guard.push(err_msg);
                            }
                        }
                    }
                    Err(e) => {
                        let err_msg = format!("[ERROR] 等待编译进程失败: {}", e);
                        if let Ok(mut logs_guard) = logs.lock() {
                            logs_guard.push(err_msg);
                        }
                    }
                }
            }
            Err(e) => {
                let err_msg = format!("[ERROR] 启动编译进程失败: {}", e);
                if let Ok(mut logs_guard) = logs.lock() {
                    logs_guard.push(err_msg);
                }
            }
        }
        
        is_running.store(false, Ordering::SeqCst);
    });
    
    // 保存构建进程引用
    let mut build_guard = BUILD_PROCESS.write().map_err(|e| format!("写入构建进程状态失败: {}", e))?;
    *build_guard = Some(build_process);
    
    Ok(true)
}

/// 获取编译日志（增量）
#[tauri::command]
pub fn get_build_logs() -> Result<Vec<String>, String> {
    let build_guard = BUILD_PROCESS.read().map_err(|e| format!("读取构建进程状态失败: {}", e))?;
    
    if let Some(ref process) = *build_guard {
        if let Ok(logs) = process.logs.lock() {
            if let Ok(mut last_index) = process.last_read_index.lock() {
                let new_logs = if *last_index < logs.len() {
                    logs[*last_index..].to_vec()
                } else {
                    Vec::new()
                };
                
                *last_index = logs.len();
                return Ok(new_logs);
            }
        }
    }
    
    Ok(Vec::new())
}

/// 检查编译是否正在运行
#[tauri::command]
pub fn is_build_running() -> Result<bool, String> {
    let build_guard = BUILD_PROCESS.read().map_err(|e| format!("读取构建进程状态失败: {}", e))?;
    
    if let Some(ref process) = *build_guard {
        Ok(process.is_running.load(Ordering::SeqCst))
    } else {
        Ok(false)
    }
}

/// 停止编译进程
#[tauri::command]
pub fn stop_build() -> Result<bool, String> {
    let build_guard = BUILD_PROCESS.read().map_err(|e| format!("读取构建进程状态失败: {}", e))?;
    
    if let Some(ref process) = *build_guard {
        process.is_running.store(false, Ordering::SeqCst);
        
        // 添加终止日志
        if let Ok(mut logs) = process.logs.lock() {
            logs.push("[BUILD] 编译已被用户终止".to_string());
        }
    }
    
    Ok(true)
}

/// 执行编译命令（用于特定目录的编译）
#[tauri::command]
pub fn execute_build(build_path: &str, server_path: &str) -> Result<String, String> {
    let build_file = Path::new(build_path);
    
    // 检查build文件是否存在
    if !build_file.exists() {
        return Err(format!("编译文件不存在: {}", build_path));
    }
    
    // 执行编译命令
    let mut cmd = Command::new("cmd");
    cmd.args(&["/C", build_path])
        .current_dir(server_path);
    
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    
    let output = cmd.output()
        .map_err(|e| format!("执行编译命令失败: {}", e))?;
    
    let stdout = decode_gbk_to_utf8(&output.stdout);
    let stderr = decode_gbk_to_utf8(&output.stderr);
    
    if !output.status.success() {
        return Err(format!("编译失败:\n{}\n{}", stdout, stderr));
    }
    
    Ok(format!("{}\n{}", stdout, stderr))
}

/// 检查编译脚本是否存在
#[tauri::command]
pub fn check_build_script(server_path: &str) -> Result<bool, String> {
    let build_path = Path::new(server_path).join("build.cmd");
    Ok(build_path.exists())
}