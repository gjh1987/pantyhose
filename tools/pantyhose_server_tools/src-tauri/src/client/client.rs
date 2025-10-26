use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use super::tcp_client::TcpClient;
use super::websocket_client::WebSocketClient;
use super::event_emitter::ClientEventEmitter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientType {
    Tcp,
    WebSocket,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClientInfo {
    pub id: i32,
    pub server_id: String,
    pub server_name: String,
    pub client_type: ClientType,
    pub connected: bool,
    pub host: String,
    pub tcp_port: Option<u16>,
    pub ws_port: Option<u16>,
}

pub struct Client {
    pub id: i32,
    pub server_id: String,
    pub server_name: String,
    pub host: String,
    pub tcp_port: Option<u16>,
    pub ws_port: Option<u16>,
    pub client_type: Arc<RwLock<ClientType>>,
    pub connected: Arc<RwLock<bool>>,
    tcp_client: Arc<RwLock<Option<Arc<TcpClient>>>>,
    ws_client: Arc<RwLock<Option<Arc<WebSocketClient>>>>,
    message_callback: Arc<RwLock<Option<Box<dyn Fn(Vec<u8>) + Send + Sync>>>>,
    event_emitter: ClientEventEmitter,
}

impl Client {
    pub fn new(
        id: i32,
        server_id: String,
        server_name: String,
        host: String,
        tcp_port: Option<u16>,
        ws_port: Option<u16>,
        initial_type: ClientType,
    ) -> Self {
        Self {
            id,
            server_id,
            server_name,
            host,
            tcp_port,
            ws_port,
            client_type: Arc::new(RwLock::new(initial_type)),
            connected: Arc::new(RwLock::new(false)),
            tcp_client: Arc::new(RwLock::new(None)),
            ws_client: Arc::new(RwLock::new(None)),
            message_callback: Arc::new(RwLock::new(None)),
            event_emitter: ClientEventEmitter::new(id),
        }
    }
    
    // 切换客户端类型（仅在未连接时允许）
    pub async fn switch_type(&self, new_type: ClientType) -> Result<(), String> {
        let connected = *self.connected.read().await;
        if connected {
            return Err("不能在连接状态下切换协议类型".to_string());
        }
        
        let mut client_type = self.client_type.write().await;
        *client_type = new_type;
        
        Ok(())
    }
    
    // 连接
    pub async fn connect(&self) -> Result<(), String> {
        let connected = *self.connected.read().await;
        if connected {
            return Err("已经连接".to_string());
        }
        
        let client_type = *self.client_type.read().await;
        
        match client_type {
            ClientType::Tcp => {
                if let Some(port) = self.tcp_port {
                    // 创建TCP客户端
                    let tcp_client = Arc::new(TcpClient::new(
                        self.id.to_string(),
                        super::tcp_client::TcpClientConfig {
                            host: self.host.clone(),
                            port,
                            auto_reconnect: false,
                            reconnect_interval: 5,
                        },
                    ));
                    
                    // 设置消息回调
                    {
                        let callback_lock = self.message_callback.read().await;
                        if callback_lock.is_some() {
                            drop(callback_lock); // 释放读锁
                            
                            // 重新创建回调
                            let message_callback = self.message_callback.clone();
                            tcp_client.set_receive_callback(move |data| {
                                let callback = message_callback.clone();
                                tokio::spawn(async move {
                                    let cb_lock = callback.read().await;
                                    if let Some(ref cb) = *cb_lock {
                                        cb(data);
                                    }
                                });
                            }).await;
                        }
                    }
                    
                    // 设置断开连接回调
                    {
                        let connected = self.connected.clone();
                        let event_emitter = ClientEventEmitter::new(self.id);
                        tcp_client.set_disconnect_callback(move || {
                            // 更新连接状态
                            let connected_clone = connected.clone();
                            let event_emitter_clone = event_emitter.clone();
                            tokio::spawn(async move {
                                let mut conn = connected_clone.write().await;
                                // 只有在确实是连接状态时才更新并发送事件
                                if *conn {
                                    *conn = false;
                                    // 发送断开连接事件
                                    event_emitter_clone.on_disconnect();
                                }
                            });
                        }).await;
                    }
                    
                    // 先保存客户端引用
                    {
                        let mut tcp = self.tcp_client.write().await;
                        *tcp = Some(tcp_client.clone());
                    }
                    
                    // 连接
                    match tcp_client.connect().await {
                        Ok(_) => {
                            // 更新状态
                            let mut connected_lock = self.connected.write().await;
                            *connected_lock = true;
                            
                            // 发送连接成功事件
                            self.event_emitter.on_connect();
                            
                            Ok(())
                        }
                        Err(e) => {
                            // 连接失败，清理客户端
                            {
                                let mut tcp = self.tcp_client.write().await;
                                *tcp = None;
                            }
                            
                            // 发送错误事件
                            self.event_emitter.on_error(e.clone());
                            Err(e)
                        }
                    }
                } else {
                    Err("TCP端口未配置".to_string())
                }
            }
            ClientType::WebSocket => {
                if let Some(port) = self.ws_port {
                    // 创建WebSocket客户端
                    let ws_client = Arc::new(WebSocketClient::new(
                        self.id.to_string(),
                        super::websocket_client::WebSocketClientConfig {
                            host: self.host.clone(),
                            port,
                            auto_reconnect: false,
                            reconnect_interval: 5,
                        },
                    ));
                    
                    // 设置消息回调
                    {
                        let callback_lock = self.message_callback.read().await;
                        if callback_lock.is_some() {
                            drop(callback_lock); // 释放读锁
                            
                            // 重新创建回调
                            let message_callback = self.message_callback.clone();
                            ws_client.set_receive_callback(move |data| {
                                let callback = message_callback.clone();
                                tokio::spawn(async move {
                                    let cb_lock = callback.read().await;
                                    if let Some(ref cb) = *cb_lock {
                                        cb(data);
                                    }
                                });
                            }).await;
                        }
                    }
                    
                    // 设置断开连接回调
                    {
                        let connected = self.connected.clone();
                        let event_emitter = ClientEventEmitter::new(self.id);
                        ws_client.set_disconnect_callback(move || {
                            // 更新连接状态
                            let connected_clone = connected.clone();
                            let event_emitter_clone = event_emitter.clone();
                            tokio::spawn(async move {
                                let mut conn = connected_clone.write().await;
                                // 只有在确实是连接状态时才更新并发送事件
                                if *conn {
                                    *conn = false;
                                    // 发送断开连接事件
                                    event_emitter_clone.on_disconnect();
                                }
                            });
                        }).await;
                    }
                    
                    // 先保存客户端引用
                    {
                        let mut ws = self.ws_client.write().await;
                        *ws = Some(ws_client.clone());
                    }
                    
                    // 连接
                    match ws_client.connect().await {
                        Ok(_) => {
                            // 更新状态
                            let mut connected_lock = self.connected.write().await;
                            *connected_lock = true;
                            
                            // 发送连接成功事件
                            self.event_emitter.on_connect();
                            
                            Ok(())
                        }
                        Err(e) => {
                            // 连接失败，清理客户端
                            {
                                let mut ws = self.ws_client.write().await;
                                *ws = None;
                            }
                            
                            // 发送错误事件
                            self.event_emitter.on_error(e.clone());
                            Err(e)
                        }
                    }
                } else {
                    Err("WebSocket端口未配置".to_string())
                }
            }
        }
    }
    
    // 断开连接
    pub async fn disconnect(&self) -> Result<(), String> {
        let was_connected = *self.connected.read().await;
        
        // 如果本来就没连接，直接返回
        if !was_connected {
            return Ok(());
        }
        
        // 先更新状态，防止重复调用
        {
            let mut connected_lock = self.connected.write().await;
            *connected_lock = false;
        }
        
        let client_type = *self.client_type.read().await;
        
        match client_type {
            ClientType::Tcp => {
                let mut tcp = self.tcp_client.write().await;
                if let Some(client) = tcp.take() {
                    let _ = client.disconnect().await; // 忽略错误
                }
            }
            ClientType::WebSocket => {
                let mut ws = self.ws_client.write().await;
                if let Some(client) = ws.take() {
                    let _ = client.disconnect().await; // 忽略错误
                }
            }
        }
        
        // 发送断开连接事件（只发送一次）
        self.event_emitter.on_disconnect();
        
        Ok(())
    }
    
    // 发送消息
    pub async fn send_message(&self, message: Vec<u8>) -> Result<(), String> {
        let connected = *self.connected.read().await;
        if !connected {
            return Err("未连接".to_string());
        }
        
        let client_type = *self.client_type.read().await;
        
        match client_type {
            ClientType::Tcp => {
                let tcp = self.tcp_client.read().await;
                if let Some(client) = tcp.as_ref() {
                    client.send(message).await
                } else {
                    Err("TCP客户端未初始化".to_string())
                }
            }
            ClientType::WebSocket => {
                let ws = self.ws_client.read().await;
                if let Some(client) = ws.as_ref() {
                    client.send(message).await
                } else {
                    Err("WebSocket客户端未初始化".to_string())
                }
            }
        }
    }
    
    // 设置消息回调
    pub async fn set_message_callback<F>(&self, callback: F)
    where
        F: Fn(Vec<u8>) + Send + Sync + 'static,
    {
        let mut cb = self.message_callback.write().await;
        *cb = Some(Box::new(callback));
    }
    
    // 获取客户端信息
    pub async fn get_info(&self) -> ClientInfo {
        ClientInfo {
            id: self.id,
            server_id: self.server_id.clone(),
            server_name: self.server_name.clone(),
            client_type: *self.client_type.read().await,
            connected: *self.connected.read().await,
            host: self.host.clone(),
            tcp_port: self.tcp_port,
            ws_port: self.ws_port,
        }
    }
    
    // 检查连接状态
    pub async fn is_connected(&self) -> bool {
        let connected = *self.connected.read().await;
        if !connected {
            return false;
        }
        
        // 验证实际连接状态
        let client_type = *self.client_type.read().await;
        match client_type {
            ClientType::Tcp => {
                let tcp = self.tcp_client.read().await;
                if let Some(client) = tcp.as_ref() {
                    matches!(
                        client.get_status().await,
                        super::tcp_client::ClientStatus::Connected
                    )
                } else {
                    false
                }
            }
            ClientType::WebSocket => {
                let ws = self.ws_client.read().await;
                if let Some(client) = ws.as_ref() {
                    matches!(
                        client.get_status().await,
                        super::websocket_client::ClientStatus::Connected
                    )
                } else {
                    false
                }
            }
        }
    }
}