use super::session_trait::SessionTrait;
use super::front_session_meta_data::FrontSessionMetaData;
use crate::framework::network::connection::{ConnectionTrait, TcpConnection, WebSocketConnection};
use crate::framework::network::{NetworkEventQueue, network_event_queue::ServerType};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::WebSocketStream;
use tokio::net::TcpStream;
use tungstenite::Message;
use tracing::{info, error, debug};

pub enum ConnectionType {
    Tcp(TcpConnection),
    WebSocket(WebSocketConnection),
}

pub struct FrontSession {
    session_id: u64,
    user_id: Option<u64>,
    remote_addr: Option<SocketAddr>,
    authenticated: bool,
    metadata: FrontSessionMetaData,
    connection: Option<ConnectionType>,
    send_queue: mpsc::UnboundedSender<Message>,
    _recv_queue: mpsc::UnboundedReceiver<Message>,
}

impl FrontSession {
    // ========== new methods ==========
    /// 使用TCP连接创建FrontSession
    pub fn new_with_tcp(
        session_id: u64, 
        tcp_stream: TcpStream, 
        remote_addr: SocketAddr,
        event_queue: NetworkEventQueue,
        notify: Arc<tokio::sync::Notify>,
        server_type: ServerType,
    ) -> Self {
        let (send_tx, recv_rx) = mpsc::unbounded_channel();
        
        let tcp_connection = TcpConnection::new(
            session_id, 
            tcp_stream, 
            remote_addr, 
            event_queue,
            notify,
            server_type,
        );
        
        // 注意：不在这里启动读取任务，需要在设置了msg_processor之后手动启动
        // tcp_connection.start_read_task();
        
        let connection = ConnectionType::Tcp(tcp_connection);
        
        Self {
            session_id,
            user_id: None,
            remote_addr: Some(remote_addr),
            authenticated: false,
            metadata: FrontSessionMetaData::new(),
            connection: Some(connection),
            send_queue: send_tx,
            _recv_queue: recv_rx,
        }
    }
    
    /// 使用WebSocket连接创建FrontSession
    pub fn new_with_websocket(
        session_id: u64, 
        ws_stream: WebSocketStream<TcpStream>, 
        remote_addr: SocketAddr,
        event_queue: crate::framework::network::NetworkEventQueue,
        notify: Arc<tokio::sync::Notify>,
        server_type: crate::framework::network::network_event_queue::ServerType,
    ) -> Self {
        let (send_tx, recv_rx) = mpsc::unbounded_channel();
        
        let connection = ConnectionType::WebSocket(WebSocketConnection::new(
            session_id, 
            ws_stream, 
            remote_addr,
            event_queue,
            notify,
            server_type,
        ));
        
        Self {
            session_id,
            user_id: None,
            remote_addr: Some(remote_addr),
            authenticated: false,
            metadata: FrontSessionMetaData::new(),
            connection: Some(connection),
            send_queue: send_tx,
            _recv_queue: recv_rx,
        }
    }
    
    /// 使用已有的TcpConnection创建FrontSession
    pub fn new_with_tcp_connection(session_id: u64, tcp_connection: TcpConnection, remote_addr: SocketAddr) -> Self {
        let (send_tx, recv_rx) = mpsc::unbounded_channel();
        
        let connection = ConnectionType::Tcp(tcp_connection);
        
        Self {
            session_id,
            user_id: None,
            remote_addr: Some(remote_addr),
            authenticated: false,
            metadata: FrontSessionMetaData::new(),
            connection: Some(connection),
            send_queue: send_tx,
            _recv_queue: recv_rx,
        }
    }

    // ========== get/set methods ==========
    /// Get connection reference
    pub fn get_connection(&self) -> Option<&ConnectionType> {
        self.connection.as_ref()
    }

    /// Get mutable connection reference
    pub fn get_connection_mut(&mut self) -> Option<&mut ConnectionType> {
        self.connection.as_mut()
    }

    /// 获取会话元数据
    pub fn get_metadata(&self) -> &FrontSessionMetaData {
        &self.metadata
    }

    /// 获取可变会话元数据
    pub fn get_metadata_mut(&mut self) -> &mut FrontSessionMetaData {
        &mut self.metadata
    }
}

impl SessionTrait for FrontSession {
    fn get_session_id(&self) -> u64 {
        self.session_id
    }

    fn get_user_id(&self) -> Option<u64> {
        self.user_id
    }

    fn set_user_id(&mut self, user_id: u64) -> bool {
        self.user_id = Some(user_id);
        true
    }

    fn get_remote_addr(&self) -> Option<SocketAddr> {
        self.remote_addr
    }

    fn is_connected(&self) -> bool {
        if let Some(ref connection) = self.connection {
            match connection {
                ConnectionType::Tcp(tcp) => tcp.is_active(),
                ConnectionType::WebSocket(ws) => ws.is_active(),
            }
        } else {
            false
        }
    }

    fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    fn set_authenticated(&mut self, authenticated: bool) -> bool {
        if self.authenticated != authenticated {
            self.authenticated = authenticated;
            debug!("FrontSession {} authentication status changed to: {}", self.session_id, authenticated);
        }
        true
    }

    fn close(&mut self) -> bool {
        if self.is_connected() {
            if let Some(connection) = self.connection.take() {
                match connection {
                    ConnectionType::Tcp(mut tcp) => {
                        tcp.stop();
                        tcp.dispose();
                    }
                    ConnectionType::WebSocket(mut ws) => {
                        ws.stop();
                        ws.dispose();
                    }
                }
            }
            debug!("FrontSession {} closed", self.session_id);
        }
        true
    }

}

impl FrontSession {
    /// 发送消息
    pub fn send_message<T>(&mut self, message: T) -> bool
    where
        T: crate::proto::messages::MessageIdSerialize + Clone + Send + 'static,
    {
        if !self.is_connected() {
            error!("Attempt to send message on disconnected front session");
            return false;
        }

        if let Some(ref mut connection) = self.connection {
            match connection {
                ConnectionType::Tcp(tcp) => tcp.send_message(message.clone()),
                ConnectionType::WebSocket(ws) => ws.send_message(message),
            }
        } else {
            error!("FrontSession {}: No connection available for sending message", self.session_id);
            false
        }
    }
}