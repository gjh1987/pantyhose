use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};
use std::time::Duration;
use tokio::time::timeout;
use super::dynamic_buffer::DynamicBuffer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpClientConfig {
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

pub struct TcpClient {
    pub id: String,
    pub config: TcpClientConfig,
    pub status: Arc<Mutex<ClientStatus>>,
    read_half: Arc<Mutex<Option<OwnedReadHalf>>>,
    write_half: Arc<Mutex<Option<OwnedWriteHalf>>>,
    receive_callback: Arc<Mutex<Option<Box<dyn Fn(Vec<u8>) + Send + Sync>>>>,
    disconnect_callback: Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>,
}

impl TcpClient {
    pub fn new(id: String, config: TcpClientConfig) -> Self {
        Self {
            id,
            config,
            status: Arc::new(Mutex::new(ClientStatus::Disconnected)),
            read_half: Arc::new(Mutex::new(None)),
            write_half: Arc::new(Mutex::new(None)),
            receive_callback: Arc::new(Mutex::new(None)),
            disconnect_callback: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn connect(&self) -> Result<(), String> {
        {
            let mut status = self.status.lock().await;
            *status = ClientStatus::Connecting;
        }

        let addr = format!("{}:{}", self.config.host, self.config.port);
        
        match timeout(Duration::from_secs(10), TcpStream::connect(&addr)).await {
            Ok(Ok(stream)) => {
                // 分离读写通道
                let (read, write) = stream.into_split();
                
                {
                    let mut read_lock = self.read_half.lock().await;
                    *read_lock = Some(read);
                }
                
                {
                    let mut write_lock = self.write_half.lock().await;
                    *write_lock = Some(write);
                }
                
                {
                    let mut status = self.status.lock().await;
                    *status = ClientStatus::Connected;
                }
                
                // 启动接收任务
                self.start_receive_loop().await;
                
                Ok(())
            }
            Ok(Err(e)) => {
                let error_msg = format!("连接失败: {}", e);
                {
                    let mut status = self.status.lock().await;
                    *status = ClientStatus::Error(error_msg.clone());
                }
                Err(error_msg)
            }
            Err(_) => {
                let error_msg = "连接超时".to_string();
                {
                    let mut status = self.status.lock().await;
                    *status = ClientStatus::Error(error_msg.clone());
                }
                Err(error_msg)
            }
        }
    }

    pub async fn disconnect(&self) -> Result<(), String> {
        // 关闭写通道（这会让对端收到 EOF）
        {
            let mut write = self.write_half.lock().await;
            if let Some(mut w) = write.take() {
                let _ = w.shutdown().await;
            }
        }
        
        // 清理读通道
        {
            let mut read = self.read_half.lock().await;
            *read = None;
        }
        
        let mut status = self.status.lock().await;
        *status = ClientStatus::Disconnected;
        
        Ok(())
    }

    pub async fn send(&self, data: Vec<u8>) -> Result<(), String> {
        let mut write = self.write_half.lock().await;
        
        if let Some(ref mut w) = *write {
            // 直接发送数据（数据应该已经包含了完整的协议格式）
            // 协议格式应该由调用者构建：消息ID（2字节）+ 消息长度（2字节）+ 消息内容
            w.write_all(&data).await.map_err(|e| format!("发送数据失败: {}", e))?;
            w.flush().await.map_err(|e| format!("刷新缓冲区失败: {}", e))?;
            
            Ok(())
        } else {
            Err("未连接到服务器".to_string())
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
        let read_clone = self.read_half.clone();
        let write_clone = self.write_half.clone();
        let status_clone = self.status.clone();
        let callback_clone = self.receive_callback.clone();
        let disconnect_clone = self.disconnect_callback.clone();
        let auto_reconnect = self.config.auto_reconnect;
        let reconnect_interval = self.config.reconnect_interval;
        let host = self.config.host.clone();
        let port = self.config.port;
        let client_id = self.id.clone();
        
        tokio::spawn(async move {
            // 创建动态缓冲区
            let mut buffer = DynamicBuffer::new(4096, 4096);
            buffer.set_little_endian(false); // 使用大端序（与服务器保持一致）
            
            'reconnect_loop: loop {
                // 内部接收循环
                'receive_loop: loop {
                    let mut read_lock = read_clone.lock().await;
                    
                    if let Some(ref mut read) = *read_lock {
                        // 准备临时读取缓冲区
                        let mut temp_buffer = vec![0u8; 4096];
                        
                        // 从网络读取数据到临时缓冲区
                        match read.read(&mut temp_buffer).await {
                            Ok(0) => {
                                // 连接关闭
                                println!("TCP客户端 {} 连接关闭（收到 EOF）", client_id);
                                break 'receive_loop;
                            }
                            Ok(n) => {
                                // 将读取到的数据写入动态缓冲区
                                buffer.write_bytes(&temp_buffer, 0, n);
                                
                                // 尝试解析完整的消息
                                while buffer.readable_bytes() >= 4 {
                                    // 服务器协议格式：消息ID（2字节）+ 消息长度（2字节）+ 消息内容
                                    
                                    // Peek 消息头，不移动读指针
                                    if let (Some(msg_id), Some(msg_len)) = (buffer.peek_u16(0), buffer.peek_u16(2)) {
                                        let msg_len = msg_len as usize;
                                        
                                        // 检查是否有完整的消息（头部4字节 + 消息体）
                                        if buffer.readable_bytes() >= 4 + msg_len {
                                            // 读取完整的消息（包括header和data）
                                            let total_len = 4 + msg_len;
                                            let mut full_msg = vec![0u8; total_len];
                                            
                                            // 将消息ID写入（大端序）
                                            full_msg[0] = (msg_id >> 8) as u8;
                                            full_msg[1] = (msg_id & 0xff) as u8;
                                            // 将消息长度写入（大端序）
                                            full_msg[2] = (msg_len >> 8) as u8;
                                            full_msg[3] = (msg_len & 0xff) as u8;
                                            
                                            // 跳过已经peek的header，读取实际数据
                                            buffer.skip(4); // 跳过消息头
                                            buffer.read_bytes(&mut full_msg[4..], 0, msg_len);
                                            
                                            // 打印日志
                                            println!("TCP客户端 {} 收到完整消息，ID: {}, 长度: {}", 
                                                client_id, 
                                                msg_id, 
                                                msg_len
                                            );
                                            
                                            // 调用回调函数，传递完整的原始消息
                                            let callback = callback_clone.lock().await;
                                            if let Some(ref cb) = *callback {
                                                cb(full_msg);
                                            }
                                        } else {
                                            // 消息不完整，等待更多数据
                                            break;
                                        }
                                    } else {
                                        // 头部不完整，等待更多数据
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                println!("TCP客户端 {} 读取数据失败: {}", client_id, e);
                                break 'receive_loop;
                            }
                        }
                    } else {
                        break 'receive_loop;
                    }
                }
                
                // 连接断开
                {
                    let mut status = status_clone.lock().await;
                    *status = ClientStatus::Disconnected;
                }
                
                // 清理读写通道
                {
                    let mut read = read_clone.lock().await;
                    *read = None;
                }
                {
                    let mut write = write_clone.lock().await;
                    *write = None;
                }
                
                // 调用断开连接回调
                {
                    let disconnect_cb = disconnect_clone.lock().await;
                    if let Some(ref cb) = *disconnect_cb {
                        cb();
                    }
                }
                
                // 自动重连
                if auto_reconnect {
                    println!("TCP客户端 {} 将在 {} 秒后重连", client_id, reconnect_interval);
                    tokio::time::sleep(Duration::from_secs(reconnect_interval)).await;
                    
                    // 清空缓冲区，准备新的连接
                    buffer.clear();
                    
                    // 尝试重新连接
                    let addr = format!("{}:{}", host, port);
                    match TcpStream::connect(&addr).await {
                        Ok(stream) => {
                            // 分离读写通道
                            let (read, write) = stream.into_split();
                            
                            {
                                let mut read_lock = read_clone.lock().await;
                                *read_lock = Some(read);
                            }
                            
                            {
                                let mut write_lock = write_clone.lock().await;
                                *write_lock = Some(write);
                            }
                            
                            let mut status = status_clone.lock().await;
                            *status = ClientStatus::Connected;
                            
                            println!("TCP客户端 {} 重连成功", client_id);
                            // 继续外层循环，重新开始接收
                            continue 'reconnect_loop;
                        }
                        Err(e) => {
                            println!("TCP客户端 {} 重连失败: {}", client_id, e);
                            break 'reconnect_loop;
                        }
                    }
                } else {
                    break 'reconnect_loop;
                }
            }
            
            println!("TCP客户端 {} 接收循环结束", client_id);
        });
    }

    pub async fn get_status(&self) -> ClientStatus {
        let status = self.status.lock().await;
        status.clone()
    }
}