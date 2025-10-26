use serde::{Serialize, Deserialize};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::path::Path;
use std::fs;
use std::env;

/// 服务器信息结构体
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerInfo {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub back_tcp_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub front_tcp_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub front_ws_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

/// 服务器组结构体
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerGroup {
    pub id: String,
    pub name: String,
    pub r#type: String,
    pub children: Vec<ServerInfo>,
}

/// 服务器列表结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerList {
    pub servers: Vec<ServerGroup>
}

/// 解析服务器配置XML文件
#[tauri::command]
pub fn parse_server_config(path: &str, config_file_name: Option<&str>) -> Result<ServerList, String> {
    // 使用传入的配置文件名，如果没有传入则使用默认值
    let config_name = config_file_name.unwrap_or("config.xml");
    
    // 检查路径是否存在
    let config_path = Path::new(path).join(config_name);
    println!("查找配置文件: {:?}", config_path);
    if !config_path.exists() {
        return Err(format!("配置文件不存在: {:?}", config_path));
    }
    println!("配置文件存在，开始解析");

    // 解析XML
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("无法读取文件: {}", e))?;
    let mut reader = Reader::from_str(&content);
    reader.config_mut().trim_text(true);
    
    let mut server_groups = Vec::new();
    let mut current_group: Option<ServerGroup> = None;
    let mut current_server: Option<ServerInfo> = None;
    let mut in_servers_section = false;
    let mut in_group_section = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.name();
                match name.as_ref() {
                    b"servers" => {
                        in_servers_section = true;
                    },
                    b"group" if in_servers_section => {
                        in_group_section = true;
                        // 创建新服务器组
                        let mut group = ServerGroup {
                            id: String::new(),
                            name: String::new(),
                            r#type: "group".to_string(),
                            children: Vec::new(),
                        };

                        // 解析组属性
                        for attr in e.attributes() {
                            let attr = attr.map_err(|e| format!("XML属性错误: {}", e))?;
                            match attr.key.as_ref() {
                                b"name" => {
                                    let name = attr.unescape_value().map_err(|e| format!("XML解码错误: {}", e))?.into_owned();
                                    group.name = name.clone();
                                    group.id = name; // 使用name作为id
                                },
                                _ => {}
                            }
                        }

                        current_group = Some(group);
                    },
                    b"server" if in_servers_section && in_group_section => {
                        // 创建新服务器信息
                    let mut server = ServerInfo {
                        id: String::new(),
                        name: None,
                        back_tcp_port: None,
                        front_tcp_port: None,
                        front_ws_port: None,
                        r#type: Some("server".to_string()),
                    };

                    // 解析服务器属性
                    for attr in e.attributes() {
                        let attr = attr.map_err(|e| format!("XML属性错误: {}", e))?;
                        match attr.key.as_ref() {
                            b"id" => server.id = attr.unescape_value().map_err(|e| format!("XML解码错误: {}", e))?.into_owned(),
                            b"name" => server.name = Some(attr.unescape_value().map_err(|e| format!("XML解码错误: {}", e))?.into_owned()),
                            b"back_tcp_port" => {
                                let port_str = attr.unescape_value().map_err(|e| format!("XML解码错误: {}", e))?;
                                server.back_tcp_port = Some(port_str.parse().map_err(|e| format!("端口解析错误: {}", e))?);
                            },
                            b"front_tcp_port" => {
                                let port_str = attr.unescape_value().map_err(|e| format!("XML解码错误: {}", e))?;
                                server.front_tcp_port = Some(port_str.parse().map_err(|e| format!("端口解析错误: {}", e))?);
                            },
                            b"front_ws_port" => {
                                let port_str = attr.unescape_value().map_err(|e| format!("XML解码错误: {}", e))?;
                                server.front_ws_port = Some(port_str.parse().map_err(|e| format!("端口解析错误: {}", e))?);
                            },
                            _ => {}
                        }
                    }

                    current_server = Some(server);
                    },
                    _ => {}
                }
            },
            Ok(Event::Empty(e)) => {
                let name = e.name();
                match name.as_ref() {
                    b"server" if in_servers_section && in_group_section => {
                        // 创建新服务器信息
                        let mut server = ServerInfo {
                            id: String::new(),
                            name: None,
                            back_tcp_port: None,
                            front_tcp_port: None,
                            front_ws_port: None,
                            r#type: Some("server".to_string()),
                        };

                        // 解析服务器属性
                        for attr in e.attributes() {
                            let attr = attr.map_err(|e| format!("XML属性错误: {}", e))?;
                            match attr.key.as_ref() {
                                b"id" => server.id = attr.unescape_value().map_err(|e| format!("XML解码错误: {}", e))?.into_owned(),
                                b"name" => server.name = Some(attr.unescape_value().map_err(|e| format!("XML解码错误: {}", e))?.into_owned()),
                                b"back_tcp_port" => {
                                    let port_str = attr.unescape_value().map_err(|e| format!("XML解码错误: {}", e))?;
                                    server.back_tcp_port = Some(port_str.parse().map_err(|e| format!("端口解析错误: {}", e))?);
                                },
                                b"front_tcp_port" => {
                                    let port_str = attr.unescape_value().map_err(|e| format!("XML解码错误: {}", e))?;
                                    server.front_tcp_port = Some(port_str.parse().map_err(|e| format!("端口解析错误: {}", e))?);
                                },
                                b"front_ws_port" => {
                                    let port_str = attr.unescape_value().map_err(|e| format!("XML解码错误: {}", e))?;
                                    server.front_ws_port = Some(port_str.parse().map_err(|e| format!("端口解析错误: {}", e))?);
                                },
                                _ => {}
                            }
                        }

                        // 直接添加到当前组
                        if let Some(ref mut group) = current_group {
                            group.children.push(server);
                        }
                    },
                    _ => {}
                }
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"server" if in_servers_section && in_group_section => {
                    if let (Some(server), Some(ref mut group)) = (current_server.take(), current_group.as_mut()) {
                        group.children.push(server);
                    }
                },
                b"group" if in_servers_section => {
                    in_group_section = false;
                    if let Some(group) = current_group.take() {
                        server_groups.push(group);
                    }
                },
                b"servers" => {
                    in_servers_section = false;
                },
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(format!("XML解析错误: {}", e));
            },
            _ => {} // 处理其他事件类型
        }
    }

    println!("成功解析 {} 个服务器组", server_groups.len());
    
    Ok(ServerList { servers: server_groups })
}

/// 获取配置文件路径
pub fn get_config_file_path() -> Result<std::path::PathBuf, String> {
    let exe_path = env::current_exe().map_err(|e| format!("无法获取执行文件路径: {}", e))?;
    let exe_dir = exe_path.parent().ok_or("无法获取执行文件目录")?;
    Ok(exe_dir.join("app_config.json"))
}

/// 保存服务器配置到本地文件
#[tauri::command]
pub fn save_server_path(path: &str) -> Result<(), String> {
    let config_path = get_config_file_path()?;
    
    // 先读取现有配置
    let mut config = if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("无法读取配置文件: {}", e))?;
        serde_json::from_str::<serde_json::Value>(&content)
            .unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    
    // 更新服务器路径
    config["server_path"] = serde_json::Value::String(path.to_string());
    
    fs::write(&config_path, config.to_string())
        .map_err(|e| format!("无法保存配置文件: {}", e))?;
    
    Ok(())
}

/// 保存启动配置参数（包括proto路径）
#[tauri::command]
pub fn save_startup_config(config_file_name: &str, executable_name: &str, proto_path: Option<&str>) -> Result<(), String> {
    let config_path = get_config_file_path()?;
    
    // 先读取现有配置
    let mut config = if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("无法读取配置文件: {}", e))?;
        serde_json::from_str::<serde_json::Value>(&content)
            .unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    
    // 更新启动配置
    config["config_file_name"] = serde_json::Value::String(config_file_name.to_string());
    config["executable_name"] = serde_json::Value::String(executable_name.to_string());
    
    // 保存proto路径（如果提供）
    if let Some(proto_path) = proto_path {
        config["proto_path"] = serde_json::Value::String(proto_path.to_string());
    }
    
    fs::write(&config_path, config.to_string())
        .map_err(|e| format!("无法保存配置文件: {}", e))?;
    
    Ok(())
}

/// 从本地文件加载服务器路径
#[tauri::command]
pub fn load_server_path() -> Result<Option<String>, String> {
    let config_path = get_config_file_path()?;
    
    if !config_path.exists() {
        // 如果配置文件不存在，返回None，让前端使用默认值
        return Ok(None);
    }
    
    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("无法读取配置文件: {}", e))?;
    
    let config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("配置文件格式错误: {}", e))?;
    
    let server_path = config["server_path"]
        .as_str()
        .map(|s| s.to_string());
    
    Ok(server_path)
}

/// 从本地文件加载启动配置参数（包括proto路径）
#[tauri::command]
pub fn load_startup_config() -> Result<(Option<String>, Option<String>, Option<String>), String> {
    let config_path = get_config_file_path()?;
    
    if !config_path.exists() {
        // 如果配置文件不存在，返回None，让前端使用默认值
        return Ok((None, None, None));
    }
    
    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("无法读取配置文件: {}", e))?;
    
    let config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("配置文件格式错误: {}", e))?;
    
    let config_file_name = config["config_file_name"]
        .as_str()
        .map(|s| s.to_string());
    
    let executable_name = config["executable_name"]
        .as_str()
        .map(|s| s.to_string());
    
    let proto_path = config["proto_path"]
        .as_str()
        .map(|s| s.to_string());
    
    Ok((config_file_name, executable_name, proto_path))
}

/// 获取应用配置
#[tauri::command]
pub fn get_app_config() -> Result<serde_json::Value, String> {
    let config_path = get_config_file_path()?;
    
    if !config_path.exists() {
        return Ok(serde_json::json!({}));
    }
    
    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("无法读取配置文件: {}", e))?;
    
    let config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("配置文件格式错误: {}", e))?;
    
    Ok(config)
}

/// 保存消息模板
#[tauri::command]
pub fn save_message_templates(server_type: &str, templates: serde_json::Value) -> Result<(), String> {
    let config_path = get_config_file_path()?;
    
    // 先读取现有配置
    let mut config = if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("无法读取配置文件: {}", e))?;
        serde_json::from_str::<serde_json::Value>(&content)
            .unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    
    // 确保 msg_template 对象存在
    if !config["msg_template"].is_object() {
        config["msg_template"] = serde_json::json!({});
    }
    
    // 更新对应服务器类型的模板
    config["msg_template"][server_type] = templates;
    
    // 格式化 JSON 以便于阅读
    let formatted = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("无法格式化 JSON: {}", e))?;
    
    fs::write(&config_path, formatted)
        .map_err(|e| format!("无法保存配置文件: {}", e))?;
    
    Ok(())
}

/// 加载消息模板
#[tauri::command]
pub fn load_message_templates(server_type: &str) -> Result<serde_json::Value, String> {
    let config_path = get_config_file_path()?;
    
    if !config_path.exists() {
        return Ok(serde_json::json!([]));
    }
    
    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("无法读取配置文件: {}", e))?;
    
    let config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("配置文件格式错误: {}", e))?;
    
    // 获取对应服务器类型的模板
    let templates = config["msg_template"][server_type].clone();
    
    if templates.is_null() {
        Ok(serde_json::json!([]))
    } else {
        Ok(templates)
    }
}


