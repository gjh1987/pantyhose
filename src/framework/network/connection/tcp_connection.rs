use super::connection::{Connection, ConnectionTrait, ConnectionState, ConnectionType};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, error, debug, warn};
use crate::framework::network::network_event_queue::{NetworkEventQueue, NetworkEventData, NetworkEventType, ServerType};

/// TCP连接实现
pub struct TcpConnection {
    /// 基础连接
    connection: Connection,
    /// TCP流读半部
    tcp_read_half: Option<Arc<tokio::sync::Mutex<tokio::net::tcp::OwnedReadHalf>>>,
    /// TCP流写半部
    tcp_write_half: Option<Arc<tokio::sync::Mutex<tokio::net::tcp::OwnedWriteHalf>>>,
    /// 远程地址
    remote_addr: SocketAddr,
    /// 连接状态
    connection_state: ConnectionState,
    /// 事件队列
    event_queue: NetworkEventQueue,
    /// 通知器
    notify: Arc<tokio::sync::Notify>,
    /// 服务器类型
    server_type: ServerType,
}

impl TcpConnection {
    /// 创建新的TCP连接
    /// 
    /// # 参数
    /// * `session_id` - 会话ID
    /// * `tcp_stream` - TCP流
    /// * `remote_addr` - 远程地址
    /// * `event_queue` - 事件队列
    /// * `notify` - 通知器
    /// * `server_type` - 服务器类型
    pub fn new(
        session_id: u64, 
        tcp_stream: TcpStream, 
        remote_addr: SocketAddr,
        event_queue: NetworkEventQueue,
        notify: Arc<tokio::sync::Notify>,
        server_type: ServerType,
    ) -> Self {
        let (read_half, write_half) = tcp_stream.into_split();
        Self {
            connection: Connection::new(session_id),
            tcp_read_half: Some(Arc::new(Mutex::new(read_half))),
            tcp_write_half: Some(Arc::new(Mutex::new(write_half))),
            remote_addr,
            connection_state: ConnectionState::Connected,
            event_queue,
            notify,
            server_type,
        }
    }
    
    /// 创建用于客户端连接的TCP连接
    /// 
    /// # 参数
    /// * `session_id` - 会话ID
    /// * `remote_addr` - 远程地址
    /// * `event_queue` - 事件队列
    /// * `notify` - 通知器
    /// * `server_type` - 服务器类型
    pub fn new_for_client(
        session_id: u64, 
        remote_addr: SocketAddr,
        event_queue: NetworkEventQueue,
        notify: Arc<tokio::sync::Notify>,
        server_type: ServerType,
    ) -> Self {
        Self {
            connection: Connection::new(session_id),
            tcp_read_half: None,
            tcp_write_half: None,
            remote_addr,
            connection_state: ConnectionState::Disconnected,
            event_queue,
            notify,
            server_type,
        }
    }
    
    /// 优雅关闭TCP连接
    /// 这个方法会先关闭写半部，让read task自然结束
    /// 同步方法，内部使用tokio::spawn处理异步操作
    pub fn shutdown(&mut self) {
        debug!("Shutting down TCP connection {}", self.connection.session_id);
        
        // 先将状态设为断开，防止新的send_message调用
        self.connection_state = ConnectionState::Disconnected;
        
        // 关闭写半部，这会导致对端收到EOF
        if let Some(write_half) = self.tcp_write_half.take() {
            let session_id = self.connection.session_id;
            
            // 在后台异步任务中执行shutdown
            tokio::spawn(async move {
                // 使用lock而不是try_lock，等待正在进行的发送操作完成
                match write_half.lock().await.shutdown().await {
                    Ok(_) => {
                        debug!("TCP connection {} write half shutdown successfully", session_id);
                    }
                    Err(e) => {
                        debug!("Failed to shutdown write half for connection {}: {}", session_id, e);
                    }
                }
                // write_half被drop，释放资源
            });
        }
        
        // 清理read_half
        // read task会在收到EOF后自然结束
        self.tcp_read_half = None;
        
        debug!("TCP connection {} shutdown complete", self.connection.session_id);
    }

    /// 启动读取任务
    pub fn start_read_task(&self) {
        if let Some(tcp_read_half) = &self.tcp_read_half {
            let session_id = self.connection.session_id;
            let tcp_read_half_clone = Arc::clone(tcp_read_half);
            let remote_addr = self.remote_addr;
            let event_queue = self.event_queue.clone();
            let notify = Arc::clone(&self.notify);
            let server_type = self.server_type;
            let msg_processor = self.connection.msg_processor.clone();
            
            tokio::spawn(async move {
                let mut temp_buffer = vec![0u8; 4096];
                let mut receive_buffer = crate::framework::data::dynamic_buffer::DynamicBuffer::new(8192, 1024);
                
                loop {
                    // 使用读半部进行读取
                    let read_result = {
                        let mut read_half = tcp_read_half_clone.lock().await;
                        read_half.read(&mut temp_buffer).await
                    };
                    
                    match read_result {
                        Ok(0) => {
                            // 连接关闭
                            debug!("TCP connection {} closed by peer", session_id);
                            let event = NetworkEventData::new(
                                NetworkEventType::Disconnect,
                                server_type,
                                session_id,
                                Some(remote_addr),
                            );
                            event_queue.push(event).await;
                            notify.notify_one();
                            break;
                        }
                        Ok(n) => {
                            // 收到数据，写入receive_buffer
                            debug!("TCP connection {} received {} bytes", session_id, n);
                            receive_buffer.write_slice(&temp_buffer[..n]);
                            
                            // Process messages in the buffer
                            while receive_buffer.readable_bytes() >= 4 {
                                // Peek at message header
                                if let (Some(msg_id), Some(msg_len)) = (
                                    receive_buffer.peek_u16(receive_buffer.read_index()),
                                    receive_buffer.peek_u16(receive_buffer.read_index() + 2)
                                ) {
                                    let msg_len = msg_len as usize;
                                    
                                    // Check if we have the complete message
                                    if receive_buffer.readable_bytes() >= msg_len + 4 {
                                        receive_buffer.skip(4); // Skip message header
                                        
                                        // Use MsgProcessor's decode_message to decode the message
                                        if let Some(ref processor) = msg_processor {
                                            if let Some(msg) = processor.decode_message(msg_id, &mut receive_buffer, msg_len) {
                                                // Create and send NewMessage event
                                                let event = NetworkEventData::new_with_message(
                                                    NetworkEventType::NewMessage,
                                                    server_type,
                                                    session_id,
                                                    Some(remote_addr),
                                                    msg,
                                                    msg_id,
                                                );
                                                
                                                // Clone event_queue and notify for async send
                                                let event_queue_clone = event_queue.clone();
                                                let notify_clone = notify.clone();
                                                
                                                tokio::spawn(async move {
                                                    event_queue_clone.push(event).await;
                                                    notify_clone.notify_one();
                                                });
                                            } else {
                                                // decode_message failed, send StreamDataNotExpected event
                                                let event = NetworkEventData::new(
                                                    NetworkEventType::StreamDataNotExpected,
                                                    server_type,
                                                    session_id,
                                                    Some(remote_addr),
                                                );
                                                let event_queue_clone = event_queue.clone();
                                                let notify_clone = notify.clone();
                                                tokio::spawn(async move {
                                                    event_queue_clone.push(event).await;
                                                    notify_clone.notify_one();
                                                });
                                                receive_buffer.skip(msg_len);
                                            }
                                        } else {
                                            // 没有处理器说明代码启动流程有问题
                                            error!("TcpConnection {} has no message processor, msg_id={}, msg_len={}", 
                                                   session_id, msg_id, msg_len);
                                            receive_buffer.skip(msg_len);
                                        }
                                    } else {
                                        // Not enough data for complete message, wait for more
                                        break;
                                    }
                                } else {
                                    // 这种情况永远不会发生，因为已经确保readable_bytes() >= 4
                                    // peek_u16在有足够数据时不应该返回None，不要添加任何代码
                                }
                            }
                        }
                        Err(e) => {
                            error!("TCP connection {} read error: {}", session_id, e);
                            let event = NetworkEventData::new(
                                NetworkEventType::Disconnect,
                                server_type,
                                session_id,
                                Some(remote_addr),
                            );
                            event_queue.push(event).await;
                            notify.notify_one();
                            break;
                        }
                    }
                }
            });
        } else {
            error!("Cannot start read task: TCP stream is None");
        }
    }

    /// 连接到指定地址
    /// 
    /// # 参数
    /// * `addr` - 目标地址
    /// * `event_queue` - 事件队列，用于发送连接成功事件
    /// * `notify` - 通知器，用于通知事件处理
    pub fn connect_to(
        &mut self, 
        addr: SocketAddr,
        event_queue: crate::framework::network::NetworkEventQueue,
        notify: Option<Arc<tokio::sync::Notify>>,
    ) {
        use crate::framework::network::network_event_queue::{NetworkEventData, NetworkEventType, ServerType};
        
        if self.connection_state != ConnectionState::Disconnected {
            warn!("TcpConnection {} already connected or connecting", self.connection.session_id);
            return;
        }

        self.connection_state = ConnectionState::Connecting;
        self.remote_addr = addr;
        debug!("TcpConnection {} connecting to {}", self.connection.session_id, addr);

        let session_id = self.connection.session_id;
        
        tokio::spawn(async move {
            match TcpStream::connect(addr).await {
                Ok(stream) => {
                    debug!("TcpConnection {} successfully connected to {}", session_id, addr);
                    
                    // 发送连接成功事件
                    let event = NetworkEventData::new_with_stream(
                        NetworkEventType::ClientConnectSuccess,
                        ServerType::BackTcp,
                        session_id,
                        Some(addr),
                        stream,
                    );
                    
                    event_queue.push(event).await;
                    
                    // 触发通知
                    if let Some(notify) = notify {
                        notify.notify_one();
                    }
                }
                Err(e) => {
                    error!("TcpConnection {} failed to connect to {}: {}", session_id, addr, e);
                }
            }
        });
    }
    
    /// 设置TCP流（用于连接成功后设置流）
    pub fn set_tcp_stream(&mut self, stream: TcpStream) {
        // local_addr can be obtained from stream when needed
        let (read_half, write_half) = stream.into_split();
        self.tcp_read_half = Some(Arc::new(Mutex::new(read_half)));
        self.tcp_write_half = Some(Arc::new(Mutex::new(write_half)));
        self.on_connected();
        
        // 自动启动读取任务
        self.start_read_task();
    }



    /// 连接建立时的回调
    fn on_connected(&mut self) {
        self.connection_state = ConnectionState::Connected;
        debug!("TcpConnection {} connected successfully", self.connection.session_id);
    }
}

impl ConnectionTrait for TcpConnection {
    fn get_session_id(&self) -> u64 {
        self.connection.session_id
    }

    fn get_remote_addr(&self) -> Option<SocketAddr> {
        Some(self.remote_addr)
    }

    fn get_local_addr(&self) -> Option<SocketAddr> {
        if let Some(_read_half) = &self.tcp_read_half {
            // Note: This requires async context, so we return None for now
            // In practice, local_addr should be obtained differently
            None
        } else {
            None
        }
    }

    fn get_connection_state(&self) -> ConnectionState {
        self.connection_state.clone()
    }

    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Tcp
    }

    fn get_connection(&self) -> &Connection {
        &self.connection
    }

    fn get_connection_mut(&mut self) -> &mut Connection {
        &mut self.connection
    }

    fn init(&mut self) -> bool {
        info!("Initializing TcpConnection {}", self.connection.session_id);
        
        // 消息处理器已经是无状态的，不需要初始化

        true
    }

    fn start(&mut self) -> bool {
        debug!("Starting TcpConnection {}", self.connection.session_id);
        
        if self.connection_state == ConnectionState::Disconnected && self.tcp_read_half.is_some() {
            self.connection_state = ConnectionState::Connected;
        }

        true
    }

    fn stop(&mut self) -> bool {
        debug!("Stopping TcpConnection {}", self.connection.session_id);
        
        self.connection_state = ConnectionState::Disconnecting;
        
        // 在实际应用中，这里会触发异步关闭
        self.connection_state = ConnectionState::Disconnected;
        true
    }

    fn send_message<T>(&mut self, message: T) -> bool
    where
        T: crate::proto::messages::MessageIdSerialize + Clone + Send + 'static,
    {
        // 检查连接状态
        if self.connection_state != ConnectionState::Connected {
            debug!("TcpConnection {} is not connected, cannot send message", self.connection.session_id);
            return false;
        }
        
        // 发送消息
        if let Some(ref write_half_arc) = self.tcp_write_half {
            let session_id = self.connection.session_id;
            
            // 克隆Arc以便在异步任务中使用
            let write_half_clone = write_half_arc.clone();
            
            // 使用tokio::spawn异步序列化和发送数据
            tokio::spawn(async move {
                
                // 在异步任务中序列化消息
                match message.serialize_to_buffer() {
                    Ok(buffer) => {
                        let msg_id = message.msg_id();
                        let total_len = buffer.len();
                        
                        debug!("TcpConnection {} serialized protobuf message (id={}) {} bytes", 
                               session_id, msg_id, total_len);
                        
                        // 发送数据
                        let write_result = {
                            let mut write_half = write_half_clone.lock().await;
                            let write_result = write_half.write_all(&buffer).await;
                            if write_result.is_ok() {
                                write_half.flush().await
                            } else {
                                write_result
                            }
                        };
                        
                        match write_result {
                            Ok(_) => {
                                debug!("TcpConnection {} sent {} bytes successfully", 
                                       session_id, total_len);
                            }
                            Err(e) => {
                                error!("TcpConnection {} failed to send data: {}", session_id, e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("TcpConnection {}: Failed to serialize protobuf message: {}", session_id, e);
                    }
                }
            });
            
            true
        } else {
            error!("TcpConnection {}: TCP stream is None", self.connection.session_id);
            false
        }
    }

    fn dispose(&mut self) {
        info!("Disposing TcpConnection {}", self.connection.session_id);
        
        // 消息处理器是无状态的，只需要移除引用
        self.connection.msg_processor = None;

        // 缓冲区已移除，无需清空
        
        // 关闭TCP流
        self.tcp_read_half = None;
        self.tcp_write_half = None;
        self.connection_state = ConnectionState::Disconnected;
    }

    fn set_connection_state(&mut self, state: ConnectionState) {
        if self.connection_state != state {
            debug!("TcpConnection {} state changed from {:?} to {:?}", 
                   self.connection.session_id, self.connection_state, state);
            self.connection_state = state;
        }
    }
}