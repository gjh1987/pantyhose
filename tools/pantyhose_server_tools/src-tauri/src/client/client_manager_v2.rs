use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use tokio::sync::RwLock;
use once_cell::sync::Lazy;
use serde::{Serialize, Deserialize};
use super::client::{Client, ClientInfo, ClientType};

// 全局客户端管理器
pub static CLIENT_MANAGER: Lazy<ClientManager> = Lazy::new(|| ClientManager::new());

// 消息日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageLog {
    pub client_id: i32,
    pub log_type: String, // "sent", "received", "error", "info"
    pub message: String,
    pub timestamp: u64,
}

pub struct ClientManager {
    next_id: AtomicI32,
    clients: Arc<RwLock<HashMap<i32, Arc<Client>>>>,
    message_logs: Arc<RwLock<Vec<MessageLog>>>,
}

impl ClientManager {
    pub fn new() -> Self {
        Self {
            next_id: AtomicI32::new(1),
            clients: Arc::new(RwLock::new(HashMap::new())),
            message_logs: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    // 创建客户端（Vue端创建客户端时调用）
    pub async fn create_client(
        &self,
        server_id: String,
        server_name: String,
        host: String,
        tcp_port: Option<u16>,
        ws_port: Option<u16>,
    ) -> Result<i32, String> {
        let client_id = self.next_id.fetch_add(1, Ordering::SeqCst);
        
        // 决定初始类型
        let initial_type = if tcp_port.is_some() {
            ClientType::Tcp
        } else if ws_port.is_some() {
            ClientType::WebSocket
        } else {
            return Err("至少需要提供一个端口".to_string());
        };
        
        // 创建客户端
        let client = Arc::new(Client::new(
            client_id,
            server_id,
            server_name,
            host,
            tcp_port,
            ws_port,
            initial_type,
        ));
        
        // 设置消息回调
        let client_id_clone = client_id;
        let logs_clone = self.message_logs.clone();
        client.set_message_callback(move |data: Vec<u8>| {
            // 解析消息格式：msgId (2 bytes) + length (2 bytes) + protobuf data
            if data.len() >= 4 {
                // 读取消息ID (big-endian)
                let msg_id = ((data[0] as u16) << 8) | (data[1] as u16);
                
                // 读取消息长度 (big-endian)
                let byte_len = ((data[2] as u16) << 8) | (data[3] as u16);
                let byte_len_usize = byte_len as usize;
                
                // 提取protobuf数据
                if data.len() >= 4 + byte_len_usize {
                    let proto_data = data[4..4 + byte_len_usize].to_vec();
                    
                    println!("收到消息: msgId={}, byteLen={}", msg_id, byte_len);
                    
                    // 发送二进制消息事件到前端
                    super::event_emitter::ClientEventEmitter::new(client_id_clone)
                        .on_binary_message(msg_id, byte_len_usize, proto_data.clone());
                    
                    // 添加日志
                    let log = MessageLog {
                        client_id: client_id_clone,
                        log_type: "received".to_string(),
                        message: format!("Binary message: msgId={}, size={}", msg_id, byte_len),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    };
                    
                    // 异步添加日志
                    let logs = logs_clone.clone();
                    tokio::spawn(async move {
                        let mut logs = logs.write().await;
                        logs.push(log);
                        
                        // 限制日志数量
                        if logs.len() > 1000 {
                            logs.drain(0..100);
                        }
                    });
                } else {
                    // 消息可能被分片，等待下一个包
                    // 不打印日志，这是TCP的正常行为
                }
            } else {
                // 如果消息太短，可能是文本消息（向后兼容）
                let message = String::from_utf8_lossy(&data).to_string();
                println!("收到文本消息: {}", message);
                
                super::event_emitter::ClientEventEmitter::new(client_id_clone).on_message(message.clone());
                
                let log = MessageLog {
                    client_id: client_id_clone,
                    log_type: "received".to_string(),
                    message: message,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                };
                
                let logs = logs_clone.clone();
                tokio::spawn(async move {
                    let mut logs = logs.write().await;
                    logs.push(log);
                    
                    if logs.len() > 1000 {
                        logs.drain(0..100);
                    }
                });
            }
        }).await;
        
        // 保存客户端
        {
            let mut clients = self.clients.write().await;
            clients.insert(client_id, client);
        }
        
        // 添加日志
        self.add_log(client_id, "info", format!("客户端 {} 已创建", client_id)).await;
        
        Ok(client_id)
    }
    
    // 删除客户端（Vue端删除客户端时调用）
    pub async fn delete_client(&self, client_id: i32) -> Result<(), String> {
        // 先断开连接
        if let Some(client) = self.get_client(client_id).await {
            let _ = client.disconnect().await;
        }
        
        // 移除客户端
        {
            let mut clients = self.clients.write().await;
            clients.remove(&client_id);
        }
        
        self.add_log(client_id, "info", format!("客户端 {} 已删除", client_id)).await;
        Ok(())
    }
    
    // 切换客户端类型
    pub async fn switch_client_type(&self, client_id: i32, client_type: ClientType) -> Result<(), String> {
        if let Some(client) = self.get_client(client_id).await {
            client.switch_type(client_type).await?;
            self.add_log(client_id, "info", format!("切换到 {:?} 模式", client_type)).await;
            Ok(())
        } else {
            Err(format!("客户端 {} 不存在", client_id))
        }
    }
    
    // 连接客户端
    pub async fn connect_client(&self, client_id: i32) -> Result<(), String> {
        println!("尝试连接客户端 {}", client_id);
        
        if let Some(client) = self.get_client(client_id).await {
            let info = client.get_info().await;
            println!("客户端信息: {:?}", info);
            
            // 尝试连接，如果失败则返回错误
            match client.connect().await {
                Ok(_) => {
                    println!("客户端 {} 连接成功", client_id);
                    let info = client.get_info().await;
                    self.add_log(client_id, "info", format!("已连接到 {}:{}", 
                        info.host,
                        match info.client_type {
                            ClientType::Tcp => info.tcp_port.unwrap_or(0),
                            ClientType::WebSocket => info.ws_port.unwrap_or(0),
                        }
                    )).await;
                    Ok(())
                }
                Err(e) => {
                    println!("客户端 {} 连接失败: {}", client_id, e);
                    self.add_log(client_id, "error", format!("连接失败: {}", e)).await;
                    Err(e)
                }
            }
        } else {
            println!("客户端 {} 不存在", client_id);
            Err(format!("客户端 {} 不存在", client_id))
        }
    }
    
    // 断开连接
    pub async fn disconnect_client(&self, client_id: i32) -> Result<(), String> {
        println!("尝试断开客户端 {}", client_id);
        
        if let Some(client) = self.get_client(client_id).await {
            match client.disconnect().await {
                Ok(_) => {
                    println!("客户端 {} 断开成功", client_id);
                    self.add_log(client_id, "info", "已断开连接".to_string()).await;
                    Ok(())
                }
                Err(e) => {
                    println!("客户端 {} 断开失败: {}", client_id, e);
                    Err(e)
                }
            }
        } else {
            println!("客户端 {} 不存在", client_id);
            Err(format!("客户端 {} 不存在", client_id))
        }
    }
    
    // 发送消息
    pub async fn send_message(&self, client_id: i32, message: Vec<u8>) -> Result<(), String> {
        println!("发送消息 {}", client_id);
        if let Some(client) = self.get_client(client_id).await {
            client.send_message(message.clone()).await?;
            
            // 添加发送日志
            let message_str = String::from_utf8_lossy(&message).to_string();
            self.add_log(client_id, "sent", message_str).await;
            
            Ok(())
        } else {
            Err(format!("客户端 {} 不存在", client_id))
        }
    }
    
    // 获取客户端
    async fn get_client(&self, client_id: i32) -> Option<Arc<Client>> {
        let clients = self.clients.read().await;
        clients.get(&client_id).cloned()
    }
    
    // 获取客户端信息
    pub async fn get_client_info(&self, client_id: i32) -> Option<ClientInfo> {
        if let Some(client) = self.get_client(client_id).await {
            Some(client.get_info().await)
        } else {
            None
        }
    }
    
    // 检查客户端是否真正连接
    pub async fn is_client_connected(&self, client_id: i32) -> bool {
        if let Some(client) = self.get_client(client_id).await {
            client.is_connected().await
        } else {
            false
        }
    }
    
    // 获取所有客户端信息
    pub async fn get_all_clients(&self) -> Vec<ClientInfo> {
        let clients = self.clients.read().await;
        let mut infos = Vec::new();
        
        for client in clients.values() {
            infos.push(client.get_info().await);
        }
        
        infos
    }
    
    // 获取客户端日志
    pub async fn get_client_logs(&self, client_id: Option<i32>) -> Vec<MessageLog> {
        let logs = self.message_logs.read().await;
        
        if let Some(id) = client_id {
            logs.iter()
                .filter(|log| log.client_id == id)
                .cloned()
                .collect()
        } else {
            logs.clone()
        }
    }
    
    // 清空日志
    pub async fn clear_logs(&self, client_id: Option<i32>) {
        let mut logs = self.message_logs.write().await;
        
        if let Some(id) = client_id {
            logs.retain(|log| log.client_id != id);
        } else {
            logs.clear();
        }
    }
    
    // 添加日志
    async fn add_log(&self, client_id: i32, log_type: &str, message: String) {
        let log = MessageLog {
            client_id,
            log_type: log_type.to_string(),
            message,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        let mut logs = self.message_logs.write().await;
        logs.push(log);
        
        // 限制日志数量
        if logs.len() > 1000 {
            logs.drain(0..100);
        }
    }
}

// ============ Tauri 命令 ============

#[tauri::command]
pub async fn create_client(
    server_id: String,
    server_name: String,
    host: String,
    tcp_port: Option<u16>,
    ws_port: Option<u16>,
) -> Result<i32, String> {
    CLIENT_MANAGER.create_client(server_id, server_name, host, tcp_port, ws_port).await
}

#[tauri::command]
pub async fn delete_client(client_id: i32) -> Result<(), String> {
    CLIENT_MANAGER.delete_client(client_id).await
}

#[tauri::command]
pub async fn switch_client_type(client_id: i32, client_type: String) -> Result<(), String> {
    let client_type = match client_type.as_str() {
        "tcp" => ClientType::Tcp,
        "websocket" | "ws" => ClientType::WebSocket,
        _ => return Err("不支持的客户端类型".to_string()),
    };
    
    CLIENT_MANAGER.switch_client_type(client_id, client_type).await
}

#[tauri::command]
pub async fn connect_client(client_id: i32) -> Result<(), String> {
    CLIENT_MANAGER.connect_client(client_id).await
}

#[tauri::command]
pub async fn disconnect_client(client_id: i32) -> Result<(), String> {
    CLIENT_MANAGER.disconnect_client(client_id).await
}

#[tauri::command]
pub async fn send_client_message(
    client_id: i32, 
    msg_id: u16,
    byte_len: usize,
    data: Vec<u8>
) -> Result<(), String> {
    // 构建完整的消息格式：msgId (2 bytes) + byteLen (2 bytes) + data
    let mut full_message = Vec::with_capacity(4 + byte_len);
    
    // 写入消息ID (big-endian)
    full_message.push((msg_id >> 8) as u8);
    full_message.push((msg_id & 0xff) as u8);
    
    // 写入消息长度 (big-endian)
    let len = byte_len as u16;
    full_message.push((len >> 8) as u8);
    full_message.push((len & 0xff) as u8);
    
    // 写入消息数据
    full_message.extend_from_slice(&data);
    
    println!("Sending message: msgId={}, byteLen={}, totalSize={}", msg_id, byte_len, full_message.len());
    
    CLIENT_MANAGER.send_message(client_id, full_message).await
}

#[tauri::command]
pub async fn get_client_info(client_id: i32) -> Result<Option<ClientInfo>, String> {
    Ok(CLIENT_MANAGER.get_client_info(client_id).await)
}

#[tauri::command]
pub async fn get_all_clients() -> Result<Vec<ClientInfo>, String> {
    Ok(CLIENT_MANAGER.get_all_clients().await)
}

#[tauri::command]
pub async fn get_client_logs(client_id: Option<i32>) -> Result<Vec<MessageLog>, String> {
    Ok(CLIENT_MANAGER.get_client_logs(client_id).await)
}

#[tauri::command]
pub async fn clear_client_logs(client_id: Option<i32>) -> Result<(), String> {
    CLIENT_MANAGER.clear_logs(client_id).await;
    Ok(())
}

#[tauri::command]
pub async fn is_client_connected(client_id: i32) -> Result<bool, String> {
    Ok(CLIENT_MANAGER.is_client_connected(client_id).await)
}