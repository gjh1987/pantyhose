use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use serde::{Serialize, Deserialize};
use std::time::Duration;
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketClientConfig {
    pub host: String,
    pub port: u16,
    pub auto_reconnect: bool,
    pub reconnect_interval: u64, // 秒
}

#[derive(Debug, Clone, Serialize)]
pub enum ClientStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

type WsStream = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

pub struct WebSocketClient {
    pub id: String,
    pub config: WebSocketClientConfig,
    pub status: Arc<Mutex<ClientStatus>>,
    stream: Arc<Mutex<Option<WsStream>>>,
    receive_callback: Arc<Mutex<Option<Box<dyn Fn(Vec<u8>) + Send + Sync>>>>,
    disconnect_callback: Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>,
    shutdown_flag: Arc<Mutex<bool>>,
}

impl WebSocketClient {
    pub fn new(id: String, config: WebSocketClientConfig) -> Self {
        Self {
            id,
            config,
            status: Arc::new(Mutex::new(ClientStatus::Disconnected)),
            stream: Arc::new(Mutex::new(None)),
            receive_callback: Arc::new(Mutex::new(None)),
            disconnect_callback: Arc::new(Mutex::new(None)),
            shutdown_flag: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn connect(&self) -> Result<(), String> {
        {
            let mut status = self.status.lock().await;
            *status = ClientStatus::Connecting;
        }

        let url = format!("ws://{}:{}", self.config.host, self.config.port);
        
        match timeout(
            Duration::from_secs(10),
            connect_async(&url)
        ).await {
            Ok(Ok((ws_stream, _))) => {
                {
                    let mut stream_lock = self.stream.lock().await;
                    *stream_lock = Some(ws_stream);
                }
                
                {
                    let mut status = self.status.lock().await;
                    *status = ClientStatus::Connected;
                }
                
                // 重置关闭标志
                {
                    let mut flag = self.shutdown_flag.lock().await;
                    *flag = false;
                }
                
                // 启动接收任务
                self.start_receive_loop().await;
                
                Ok(())
            }
            Ok(Err(e)) => {
                let error_msg = format!("WebSocket连接失败: {}", e);
                {
                    let mut status = self.status.lock().await;
                    *status = ClientStatus::Error(error_msg.clone());
                }
                Err(error_msg)
            }
            Err(_) => {
                let error_msg = "WebSocket连接超时".to_string();
                {
                    let mut status = self.status.lock().await;
                    *status = ClientStatus::Error(error_msg.clone());
                }
                Err(error_msg)
            }
        }
    }

    pub async fn disconnect(&self) -> Result<(), String> {
        // 设置关闭标志
        {
            let mut flag = self.shutdown_flag.lock().await;
            *flag = true;
        }
        
        let mut stream = self.stream.lock().await;
        if let Some(mut s) = stream.take() {
            let _ = s.close(None).await;
        }
        
        let mut status = self.status.lock().await;
        *status = ClientStatus::Disconnected;
        
        Ok(())
    }

    pub async fn send(&self, data: Vec<u8>) -> Result<(), String> {
        let mut stream = self.stream.lock().await;
        
        if let Some(ref mut s) = *stream {
            let message = Message::Binary(data);
            s.send(message).await.map_err(|e| format!("发送消息失败: {}", e))?;
            Ok(())
        } else {
            Err("未连接到WebSocket服务器".to_string())
        }
    }

    pub async fn send_text(&self, text: String) -> Result<(), String> {
        let mut stream = self.stream.lock().await;
        
        if let Some(ref mut s) = *stream {
            let message = Message::Text(text);
            s.send(message).await.map_err(|e| format!("发送文本消息失败: {}", e))?;
            Ok(())
        } else {
            Err("未连接到WebSocket服务器".to_string())
        }
    }

    pub async fn set_receive_callback<F>(&self, callback: F)
    where
        F: Fn(Vec<u8>) + Send + Sync + 'static,
    {
        let mut cb = self.receive_callback.lock().await;
        *cb = Some(Box::new(callback));
    }

    pub async fn set_disconnect_callback<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        let mut cb = self.disconnect_callback.lock().await;
        *cb = Some(Box::new(callback));
    }

    async fn start_receive_loop(&self) {
        let stream_clone = self.stream.clone();
        let status_clone = self.status.clone();
        let callback_clone = self.receive_callback.clone();
        let disconnect_clone = self.disconnect_callback.clone();
        let shutdown_flag_clone = self.shutdown_flag.clone();
        let auto_reconnect = self.config.auto_reconnect;
        let reconnect_interval = self.config.reconnect_interval;
        let host = self.config.host.clone();
        let port = self.config.port;
        let client_id = self.id.clone();
        
        tokio::spawn(async move {
            'reconnect_loop: loop {
                // 内部接收循环
                'receive_loop: loop {
                    // 检查关闭标志
                    {
                        let flag = shutdown_flag_clone.lock().await;
                        if *flag {
                            break 'reconnect_loop;
                        }
                    }
                    
                    let mut stream_lock = stream_clone.lock().await;
                    
                    if let Some(ref mut stream) = *stream_lock {
                        // 使用 next() 接收消息
                        match stream.next().await {
                            Some(Ok(msg)) => {
                                match msg {
                                    Message::Binary(data) => {
                                        // 调用回调函数
                                        let callback = callback_clone.lock().await;
                                        if let Some(ref cb) = *callback {
                                            cb(data);
                                        }
                                    }
                                    Message::Text(text) => {
                                        // 文本消息转换为字节
                                        let callback = callback_clone.lock().await;
                                        if let Some(ref cb) = *callback {
                                            cb(text.into_bytes());
                                        }
                                    }
                                    Message::Close(_) => {
                                        println!("WebSocket客户端 {} 收到关闭消息", client_id);
                                        break 'receive_loop;
                                    }
                                    Message::Ping(data) => {
                                        // 自动回复 Pong
                                        let _ = stream.send(Message::Pong(data)).await;
                                    }
                                    Message::Pong(_) => {
                                        // Pong 消息，通常不需要处理
                                    }
                                    _ => {}
                                }
                            }
                            Some(Err(e)) => {
                                println!("WebSocket客户端 {} 接收消息错误: {}", client_id, e);
                                break 'receive_loop;
                            }
                            None => {
                                println!("WebSocket客户端 {} 连接关闭", client_id);
                                break 'receive_loop;
                            }
                        }
                    } else {
                        // 没有连接，退出循环
                        break 'receive_loop;
                    }
                }
                
                // 连接断开
                {
                    let mut status = status_clone.lock().await;
                    *status = ClientStatus::Disconnected;
                }
                
                // 调用断开连接回调
                {
                    let disconnect_cb = disconnect_clone.lock().await;
                    if let Some(ref cb) = *disconnect_cb {
                        cb();
                    }
                }
                
                // 清理stream
                {
                    let mut stream = stream_clone.lock().await;
                    *stream = None;
                }
                
                // 检查是否需要自动重连
                let should_reconnect = {
                    let flag = shutdown_flag_clone.lock().await;
                    auto_reconnect && !*flag
                };
                
                if should_reconnect {
                    println!("WebSocket客户端 {} 将在 {} 秒后重连", client_id, reconnect_interval);
                    tokio::time::sleep(Duration::from_secs(reconnect_interval)).await;
                    
                    // 尝试重新连接
                    let url = format!("ws://{}:{}", host, port);
                    match connect_async(&url).await {
                        Ok((new_stream, _)) => {
                            let mut stream = stream_clone.lock().await;
                            *stream = Some(new_stream);
                            
                            let mut status = status_clone.lock().await;
                            *status = ClientStatus::Connected;
                            
                            println!("WebSocket客户端 {} 重连成功", client_id);
                            
                            // 继续外层循环，重新开始接收
                            continue 'reconnect_loop;
                        }
                        Err(e) => {
                            println!("WebSocket客户端 {} 重连失败: {}", client_id, e);
                            break 'reconnect_loop;
                        }
                    }
                } else {
                    break 'reconnect_loop;
                }
            }
            
            println!("WebSocket客户端 {} 接收循环结束", client_id);
        });
    }

    pub async fn get_status(&self) -> ClientStatus {
        let status = self.status.lock().await;
        status.clone()
    }
}