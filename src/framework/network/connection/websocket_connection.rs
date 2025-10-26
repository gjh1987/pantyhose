use super::connection::{Connection, ConnectionTrait, ConnectionState, ConnectionType};
use crate::framework::msg::MsgProcessor;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_tungstenite::{WebSocketStream, tungstenite::Message, MaybeTlsStream};
use futures_util::{SinkExt, StreamExt, stream::SplitSink, stream::SplitStream};
use tracing::{info, error, debug};
use tokio::sync::Mutex;
use crate::framework::network::network_event_queue::{NetworkEventQueue, NetworkEventData, NetworkEventType, ServerType};

/// WebSocket连接实现
/// 支持服务器端（TcpStream）和客户端（MaybeTlsStream）连接
pub enum WebSocketStream_ {
    Server(WebSocketStream<TcpStream>),
    Client(WebSocketStream<MaybeTlsStream<TcpStream>>),
}

/// WebSocket发送半部（Sink）
pub enum WebSocketSink_ {
    Server(SplitSink<WebSocketStream<TcpStream>, Message>),
    Client(SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>),
}

/// WebSocket接收半部（Stream）
pub enum WebSocketStream_Split {
    Server(SplitStream<WebSocketStream<TcpStream>>),
    Client(SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>),
}

pub struct WebSocketConnection {
    /// 基础连接
    connection: Connection,
    /// WebSocket发送半部
    websocket_sink: Option<Arc<Mutex<WebSocketSink_>>>,
    /// WebSocket接收半部
    websocket_stream: Option<Arc<Mutex<WebSocketStream_Split>>>,
    /// 远程地址
    remote_addr: Option<SocketAddr>,
    /// 本地地址
    local_addr: Option<SocketAddr>,
    /// 连接状态
    connection_state: ConnectionState,
    /// 事件队列
    event_queue: NetworkEventQueue,
    /// 通知器
    notify: Arc<tokio::sync::Notify>,
    /// 服务器类型
    server_type: ServerType,
}

impl WebSocketSink_ {
    /// 异步发送消息（处理不同的Sink类型）
    pub async fn send(&mut self, msg: Message) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        match self {
            WebSocketSink_::Server(sink) => sink.send(msg).await,
            WebSocketSink_::Client(sink) => sink.send(msg).await,
        }
    }

    /// 关闭连接（处理不同的Sink类型）
    pub async fn close(&mut self) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        match self {
            WebSocketSink_::Server(sink) => sink.close().await,
            WebSocketSink_::Client(sink) => sink.close().await,
        }
    }
}

impl WebSocketStream_Split {
    /// 异步接收消息（处理不同的Stream类型）
    pub async fn next(&mut self) -> Option<Result<Message, tokio_tungstenite::tungstenite::Error>> {
        match self {
            WebSocketStream_Split::Server(stream) => stream.next().await,
            WebSocketStream_Split::Client(stream) => stream.next().await,
        }
    }
}

impl WebSocketStream_ {
    /// 异步接收消息（处理不同的流类型）
    pub async fn next(&mut self) -> Option<Result<Message, tokio_tungstenite::tungstenite::Error>> {
        match self {
            WebSocketStream_::Server(stream) => stream.next().await,
            WebSocketStream_::Client(stream) => stream.next().await,
        }
    }

    /// 异步发送消息（处理不同的流类型）
    pub async fn send(&mut self, msg: Message) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        match self {
            WebSocketStream_::Server(stream) => stream.send(msg).await,
            WebSocketStream_::Client(stream) => stream.send(msg).await,
        }
    }

    /// 分离WebSocket流为发送和接收半部
    pub fn split(self) -> (WebSocketSink_, WebSocketStream_Split) {
        match self {
            WebSocketStream_::Server(stream) => {
                let (sink, stream) = stream.split();
                (WebSocketSink_::Server(sink), WebSocketStream_Split::Server(stream))
            }
            WebSocketStream_::Client(stream) => {
                let (sink, stream) = stream.split();
                (WebSocketSink_::Client(sink), WebSocketStream_Split::Client(stream))
            }
        }
    }

    /// 关闭连接（处理不同的流类型）
    pub async fn close(&mut self, close_frame: Option<tokio_tungstenite::tungstenite::protocol::CloseFrame>) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        match self {
            WebSocketStream_::Server(stream) => stream.close(close_frame).await,
            WebSocketStream_::Client(stream) => stream.close(close_frame).await,
        }
    }
}

impl WebSocketConnection {
    /// 创建新的WebSocket连接
    /// 
    /// # 参数
    /// * `session_id` - 连接ID
    /// * `websocket_stream` - WebSocket流
    /// * `remote_addr` - 远程地址
    /// * `event_queue` - 事件队列
    /// * `notify` - 通知器
    /// * `server_type` - 服务器类型
    pub fn new(
        session_id: u64, 
        websocket_stream: WebSocketStream<TcpStream>, 
        remote_addr: SocketAddr,
        event_queue: NetworkEventQueue,
        notify: Arc<tokio::sync::Notify>,
        server_type: ServerType,
    ) -> Self {
        let ws_stream = WebSocketStream_::Server(websocket_stream);
        let (sink, stream) = ws_stream.split();
        
        Self {
            connection: Connection::new(session_id),
            websocket_sink: Some(Arc::new(Mutex::new(sink))),
            websocket_stream: Some(Arc::new(Mutex::new(stream))),
            remote_addr: Some(remote_addr),
            local_addr: None,
            connection_state: ConnectionState::Connected,
            event_queue,
            notify,
            server_type,
        }
    }

    /// 启动读取任务
    pub fn start_read_task(&self) {
        if let Some(websocket_stream) = &self.websocket_stream {
            if let Some(websocket_sink) = &self.websocket_sink {
                let session_id = self.connection.session_id;
                let ws_stream_clone = Arc::clone(websocket_stream);
                let ws_sink_clone = Arc::clone(websocket_sink);
                let remote_addr = self.remote_addr;
                let event_queue = self.event_queue.clone();
                let notify = Arc::clone(&self.notify);
                let server_type = self.server_type;
                let msg_processor = self.connection.msg_processor.clone();
            
            tokio::spawn(async move {
                let mut receive_buffer = crate::framework::data::dynamic_buffer::DynamicBuffer::new(8192, 1024);
                
                loop {
                    // 获取下一条消息
                    let message_result = {
                        let mut stream = ws_stream_clone.lock().await;
                        stream.next().await
                    };
                    
                    match message_result {
                        Some(Ok(message)) => {
                            match message {
                                Message::Binary(data) => {
                                    debug!("WebSocketConnection {} received binary message: {} bytes", 
                                           session_id, data.len());
                                    
                                    // 将数据写入缓冲区
                                    receive_buffer.write_slice(&data);
                                    
                                    // 处理缓冲区中的消息
                                    while receive_buffer.readable_bytes() >= 4 {
                                        // 查看消息头
                                        if let (Some(msg_id), Some(msg_len)) = (
                                            receive_buffer.peek_u16(receive_buffer.read_index()),
                                            receive_buffer.peek_u16(receive_buffer.read_index() + 2)
                                        ) {
                                            let msg_len = msg_len as usize;
                                            
                                            // 检查是否有完整消息
                                            if receive_buffer.readable_bytes() >= msg_len + 4 {
                                                receive_buffer.skip(4); // 跳过消息头
                                                
                                                // 使用 MsgProcessor 解码消息
                                                if let Some(ref processor) = msg_processor {
                                                    if let Some(msg) = processor.decode_message(msg_id, &mut receive_buffer, msg_len) {
                                                        // 创建并发送 NewMessage 事件
                                                        let event = NetworkEventData::new_with_message(
                                                            NetworkEventType::NewMessage,
                                                            server_type,
                                                            session_id,
                                                            remote_addr,
                                                            msg,
                                                            msg_id,
                                                        );
                                                        
                                                        // 克隆 event_queue 和 notify 以便异步发送
                                                        let event_queue_clone = event_queue.clone();
                                                        let notify_clone = notify.clone();
                                                        
                                                        tokio::spawn(async move {
                                                            event_queue_clone.push(event).await;
                                                            notify_clone.notify_one();
                                                        });
                                                    } else {
                                                        error!("msg decode err");
                                                        // decode_message 失败，发送 StreamDataNotExpected 事件
                                                        let event = NetworkEventData::new(
                                                            NetworkEventType::StreamDataNotExpected,
                                                            server_type,
                                                            session_id,
                                                            remote_addr,
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
                                                    error!("WebSocketConnection {} has no message processor, msg_id={}, msg_len={}", 
                                                           session_id, msg_id, msg_len);
                                                    receive_buffer.skip(msg_len);
                                                }
                                            } else {
                                                // 数据不足，等待更多数据
                                                break;
                                            }
                                        } else {
                                            // 这种情况永远不会发生，因为已经确保readable_bytes() >= 4
                                            // peek_u16在有足够数据时不应该返回None，不要添加任何代码
                                        }
                                    }
                                }
                                Message::Text(text) => {
                                    debug!("WebSocketConnection {} received text message: {} chars", 
                                           session_id, text.len());
                                }
                                Message::Ping(data) => {
                                    debug!("WebSocketConnection {} received ping", session_id);
                                    // 自动回复 Pong
                                    let ws_sink_clone2 = ws_sink_clone.clone();
                                    tokio::spawn(async move {
                                        let mut sink = ws_sink_clone2.lock().await;
                                        if let Err(e) = sink.send(Message::Pong(data)).await {
                                            error!("WebSocketConnection {} failed to send pong: {}", session_id, e);
                                        }
                                    });
                                }
                                Message::Pong(_) => {
                                    debug!("WebSocketConnection {} received pong", session_id);
                                }
                                Message::Close(close_frame) => {
                                    debug!("WebSocketConnection {} received close frame: {:?}", 
                                          session_id, close_frame);
                                    // 发送断开连接事件
                                    let event = NetworkEventData::new(
                                        NetworkEventType::Disconnect,
                                        server_type,
                                        session_id,
                                        remote_addr,
                                    );
                                    event_queue.push(event).await;
                                    notify.notify_one();
                                    break;
                                }
                                Message::Frame(_) => {
                                    debug!("WebSocketConnection {} received raw frame", session_id);
                                }
                            }
                        }
                        Some(Err(e)) => {
                            error!("WebSocketConnection {} error: {}", session_id, e);
                            // 发送断开连接事件
                            let event = NetworkEventData::new(
                                NetworkEventType::Disconnect,
                                server_type,
                                session_id,
                                remote_addr,
                            );
                            event_queue.push(event).await;
                            notify.notify_one();
                            break;
                        }
                        None => {
                            debug!("WebSocketConnection {} stream ended", session_id);
                            // 发送断开连接事件
                            let event = NetworkEventData::new(
                                NetworkEventType::Disconnect,
                                server_type,
                                session_id,
                                remote_addr,
                            );
                            event_queue.push(event).await;
                            notify.notify_one();
                            break;
                        }
                    }
                }
            });
            } else {
                error!("Cannot start read task: WebSocket sink is None");
            }
        } else {
            error!("Cannot start read task: WebSocket stream is None");
        }
    }

}

impl ConnectionTrait for WebSocketConnection {
    fn get_session_id(&self) -> u64 {
        self.connection.session_id
    }

    fn get_remote_addr(&self) -> Option<SocketAddr> {
        self.remote_addr
    }

    fn get_local_addr(&self) -> Option<SocketAddr> {
        self.local_addr
    }

    fn get_connection_state(&self) -> ConnectionState {
        self.connection_state.clone()
    }

    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::WebSocket
    }

    fn get_connection(&self) -> &Connection {
        &self.connection
    }

    fn get_connection_mut(&mut self) -> &mut Connection {
        &mut self.connection
    }



    fn init(&mut self) -> bool {
        info!("Initializing WebSocketConnection {}", self.connection.session_id);
        
        // 消息处理器已经是无状态的，不需要初始化

        true
    }

    fn start(&mut self) -> bool {
        debug!("Starting WebSocketConnection {}", self.connection.session_id);
        
        if self.connection_state == ConnectionState::Disconnected && self.websocket_stream.is_some() && self.websocket_sink.is_some() {
            self.connection_state = ConnectionState::Connected;
        }
        
        // 启动读取任务
        // self.start_read_task();

        true
    }

    fn stop(&mut self) -> bool {
        debug!("Stopping WebSocketConnection {}", self.connection.session_id);
        
        self.connection_state = ConnectionState::Disconnecting;
        
        // 在实际应用中，这里会触发异步关闭
        self.connection_state = ConnectionState::Disconnected;
        true
    }

    fn send_message<T>(&mut self, message: T) -> bool
    where
        T: crate::proto::messages::MessageIdSerialize + Clone + Send + 'static,
    {
        // 发送消息
        if let Some(ref ws_sink_arc) = self.websocket_sink {
            let session_id = self.connection.session_id;
            
            // 克隆Arc以便在异步任务中使用
            let ws_sink_clone = ws_sink_arc.clone();
            
            // 使用tokio::spawn异步序列化和发送数据
            tokio::spawn(async move {
                
                // 在异步任务中序列化消息
                match message.serialize_to_buffer() {
                    Ok(buffer) => {
                        let msg_id = message.msg_id();
                        let total_len = buffer.len();
                        
                        debug!("WebSocketConnection {} serialized protobuf message (id={}) {} bytes", 
                               session_id, msg_id, total_len);
                        
                        // 发送数据
                        let mut sink = ws_sink_clone.lock().await;
                        let ws_message = Message::Binary(buffer.to_vec().into());
                        
                        match sink.send(ws_message).await {
                            Ok(_) => {
                                debug!("WebSocketConnection {} sent {} bytes successfully", 
                                       session_id, total_len);
                            }
                            Err(e) => {
                                error!("WebSocketConnection {} failed to send data: {}", session_id, e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("WebSocketConnection {}: Failed to serialize protobuf message: {}", session_id, e);
                    }
                }
            });
            
            true
        } else {
            error!("WebSocketConnection {}: WebSocket stream is None", self.connection.session_id);
            false
        }
    }

    fn dispose(&mut self) {
        info!("Disposing WebSocketConnection {}", self.connection.session_id);
        
        // 消息处理器是无状态的，只需要移除引用
        self.connection.msg_processor = None;

        // 缓冲区已移除，无需清空
        
        // 关闭WebSocket流
        self.websocket_sink = None;
        self.websocket_stream = None;
        self.connection_state = ConnectionState::Disconnected;
    }


    fn get_msg_processor(&self) -> Option<Arc<dyn MsgProcessor>> {
        self.connection.msg_processor.clone()
    }


    fn set_msg_processor(&mut self, processor: Arc<dyn MsgProcessor>) -> bool {
        self.connection.msg_processor = Some(processor);
        debug!("WebSocketConnection {}: Message processor set", self.connection.session_id);
        true
    }

    fn remove_msg_processor(&mut self) -> Option<Arc<dyn MsgProcessor>> {
        let processor = self.connection.msg_processor.take();
        if processor.is_some() {
            debug!("WebSocketConnection {}: Message processor removed", self.connection.session_id);
        }
        processor
    }

    fn set_connection_state(&mut self, state: ConnectionState) {
        if self.connection_state != state {
            debug!("WebSocketConnection {} state changed from {:?} to {:?}", 
                   self.connection.session_id, self.connection_state, state);
            self.connection_state = state;
        }
    }
}