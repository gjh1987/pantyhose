use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ProtoFileInfo {
    pub file_name: String,
    pub content: String,
    pub path: String,
}

/// 读取proto目录下的所有proto文件
#[tauri::command]
pub fn read_proto_files(proto_path: &str) -> Result<Vec<ProtoFileInfo>, String> {
    // 首先尝试直接路径
    let path = std::path::PathBuf::from(proto_path);
    
    // 确定最终路径
    let final_path = if path.exists() {
        path
    } else {
        // 获取当前工作目录
        let current_dir = std::env::current_dir()
            .map_err(|e| format!("获取当前目录失败: {}", e))?;
        
        // 尝试从 src-tauri 目录向上查找
        let possible_paths = vec![
            current_dir.join(proto_path), // 当前目录
            current_dir.parent().and_then(|p| p.parent()).map(|p| p.join(proto_path)).unwrap_or_else(|| std::path::PathBuf::new()), // 向上两级
            current_dir.parent().map(|p| p.join(proto_path)).unwrap_or_else(|| std::path::PathBuf::new()), // 向上一级
            std::path::PathBuf::from("../../").join(proto_path), // 相对路径向上两级
        ];
        
        possible_paths.into_iter()
            .find(|p| p.exists() && p.is_dir())
            .ok_or_else(|| format!("Proto目录不存在: {} (工作目录: {:?})", proto_path, current_dir))?
    };
    
    if !final_path.is_dir() {
        return Err(format!("路径不是目录: {}", proto_path));
    }
    
    let mut proto_files = Vec::new();
    read_proto_directory(&final_path, &mut proto_files)?;
    
    Ok(proto_files)
}

fn read_proto_directory(dir: &Path, proto_files: &mut Vec<ProtoFileInfo>) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("读取目录失败: {}", e))?;
    
    for entry in entries {
        let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
        let path = entry.path();
        
        if path.is_dir() {
            // 递归读取子目录
            read_proto_directory(&path, proto_files)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("proto") {
            // 读取proto文件
            let content = fs::read_to_string(&path)
                .map_err(|e| format!("读取文件失败 {:?}: {}", path, e))?;
            
            let file_name = path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            
            let path_str = path.to_str().unwrap_or("").to_string();
            
            proto_files.push(ProtoFileInfo {
                file_name,
                content,
                path: path_str,
            });
        }
    }
    
    Ok(())
}

/// 读取单个proto文件
#[tauri::command]
pub fn read_proto_file(file_path: &str) -> Result<ProtoFileInfo, String> {
    let path = Path::new(file_path);
    
    if !path.exists() {
        return Err(format!("文件不存在: {}", file_path));
    }
    
    if !path.is_file() {
        return Err(format!("路径不是文件: {}", file_path));
    }
    
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("读取文件失败: {}", e))?;
    
    let file_name = path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    
    Ok(ProtoFileInfo {
        file_name,
        content,
        path: file_path.to_string(),
    })
}