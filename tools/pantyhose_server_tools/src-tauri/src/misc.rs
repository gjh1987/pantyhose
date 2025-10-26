use std::process::Command;

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

/// 关闭所有服务器进程
#[tauri::command]
pub fn kill_all_servers(executable_name: &str) -> Result<String, String> {
    // 在 Windows 系统上使用 taskkill 命令关闭进程
    #[cfg(windows)]
    {
        let mut kill_cmd = Command::new("taskkill");
        kill_cmd.args(&["/F", "/IM", executable_name]);
        
        // 隐藏命令行窗口
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        kill_cmd.creation_flags(CREATE_NO_WINDOW);
        
        match kill_cmd.output() {
            Ok(output) => {
                let stdout = decode_gbk_to_utf8(&output.stdout);
                let stderr = decode_gbk_to_utf8(&output.stderr);
                
                if output.status.success() {
                    Ok(format!("成功强制关闭所有 {} 进程\n{}", executable_name, stdout))
                } else {
                    // 如果进程不存在，taskkill 会返回错误，但这不算真正的错误
                    if stderr.contains("not found") || stderr.contains("未找到") {
                        Ok(format!("没有找到运行中的 {} 进程", executable_name))
                    } else {
                        Err(format!("强制关闭进程失败: {}", stderr))
                    }
                }
            }
            Err(e) => Err(format!("执行 taskkill 命令失败: {}", e))
        }
    }
    
    // 在非 Windows 系统上使用 pkill 命令（如果需要支持）
    #[cfg(not(windows))]
    {
        // 去除 .exe 后缀（在 Linux/macOS 上通常没有 .exe 后缀）
        let process_name = executable_name.trim_end_matches(".exe");
        
        let mut kill_cmd = Command::new("pkill");
        kill_cmd.args(&["-f", process_name]);
        
        match kill_cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    Ok(format!("成功强制关闭所有 {} 进程", process_name))
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if output.status.code() == Some(1) {
                        // pkill 返回 1 表示没有找到匹配的进程
                        Ok(format!("没有找到运行中的 {} 进程", process_name))
                    } else {
                        Err(format!("强制关闭进程失败: {}", stderr))
                    }
                }
            }
            Err(e) => Err(format!("执行 pkill 命令失败: {}", e))
        }
    }
}