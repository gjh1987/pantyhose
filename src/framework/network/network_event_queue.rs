use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::Mutex;
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;

// 事件类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NetworkEventType {
    ServerOpen,
    ServerClose,
    NewTcpConnection,
    NewWebSocketConnection,
    ClientConnectSuccess,
    Disconnect,
    NewMessage,
    StreamDataNotExpected,
}

// 服务器类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServerType {
    BackTcp,
    FrontTcp,
    FrontWebSocket,
}

// 网络事件数据
#[derive(Debug)]
pub struct NetworkEventData {
    pub event_type: NetworkEventType,
    pub server_type: ServerType,
    pub session_id: u64,
    pub remote_addr: Option<SocketAddr>,
    pub timestamp: std::time::Instant,
    pub tcp_stream: Option<TcpStream>,
    pub websocket_stream: Option<WebSocketStream<TcpStream>>,
    pub message: Option<Box<dyn std::any::Any + Send>>,
    pub message_id: Option<u16>,
}

impl NetworkEventData {
    pub fn new(
        event_type: NetworkEventType,
        server_type: ServerType,
        session_id: u64,
        remote_addr: Option<SocketAddr>,
    ) -> Self {
        Self {
            event_type,
            server_type,
            session_id,
            remote_addr,
            timestamp: std::time::Instant::now(),
            tcp_stream: None,
            websocket_stream: None,
            message: None,
            message_id: None,
        }
    }
    
    pub fn new_with_stream(
        event_type: NetworkEventType,
        server_type: ServerType,
        session_id: u64,
        remote_addr: Option<SocketAddr>,
        tcp_stream: TcpStream,
    ) -> Self {
        Self {
            event_type,
            server_type,
            session_id,
            remote_addr,
            timestamp: std::time::Instant::now(),
            tcp_stream: Some(tcp_stream),
            websocket_stream: None,
            message: None,
            message_id: None,
        }
    }
    
    pub fn new_with_websocket(
        event_type: NetworkEventType,
        server_type: ServerType,
        session_id: u64,
        remote_addr: Option<SocketAddr>,
        websocket_stream: WebSocketStream<TcpStream>,
    ) -> Self {
        Self {
            event_type,
            server_type,
            session_id,
            remote_addr,
            timestamp: std::time::Instant::now(),
            tcp_stream: None,
            websocket_stream: Some(websocket_stream),
            message: None,
            message_id: None,
        }
    }
    
    pub fn new_with_message(
        event_type: NetworkEventType,
        server_type: ServerType,
        session_id: u64,
        remote_addr: Option<SocketAddr>,
        message: Box<dyn std::any::Any + Send>,
        message_id: u16,
    ) -> Self {
        Self {
            event_type,
            server_type,
            session_id,
            remote_addr,
            timestamp: std::time::Instant::now(),
            tcp_stream: None,
            websocket_stream: None,
            message: Some(message),
            message_id: Some(message_id),
        }
    }
    
}

// 网络事件队列
#[derive(Clone)]
pub struct NetworkEventQueue {
    queue: Arc<Mutex<VecDeque<NetworkEventData>>>,
}

impl NetworkEventQueue {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
    
    pub async fn push(&self, event: NetworkEventData) {
        let mut queue = self.queue.lock().await;
        queue.push_back(event);
    }
    
    pub async fn pop(&self) -> Option<NetworkEventData> {
        let mut queue = self.queue.lock().await;
        queue.pop_front()
    }
    
    pub async fn is_empty(&self) -> bool {
        let queue = self.queue.lock().await;
        queue.is_empty()
    }
    
    pub async fn len(&self) -> usize {
        let queue = self.queue.lock().await;
        queue.len()
    }
}