use super::{FrontSession, SessionTrait};
use crate::framework::msg::MsgProcessor;
use crate::framework::network::{NetworkEventHandler, NetworkEngineEventManager, network_event_queue::{NetworkEventData, NetworkEventType, NetworkEventQueue, ServerType}};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio_tungstenite::WebSocketStream;
use tokio::net::TcpStream;
use tracing::{info, error, debug};

pub struct FrontSessionManager {
    sessions: HashMap<u64, FrontSession>,
    next_session_id: u64,
    msg_processor: Option<Arc<dyn MsgProcessor>>,
    event_queue: Option<NetworkEventQueue>,
    notify: Option<Arc<tokio::sync::Notify>>,
    is_initialized: bool,
}

impl FrontSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            next_session_id: 1,
            msg_processor: None,
            event_queue: None,
            notify: None,
            is_initialized: false,
        }
    }

    /// 初始化FrontSessionManager
    pub fn init(
        &mut self,
        event_manager: &mut NetworkEngineEventManager,
        msg_processor: Arc<dyn MsgProcessor>,
        event_queue: NetworkEventQueue,
        notify: Arc<tokio::sync::Notify>,
    ) {
        if self.is_initialized {
            return;
        }

        // 设置消息处理器
        self.msg_processor = Some(msg_processor);
        self.event_queue = Some(event_queue);
        self.notify = Some(notify);

        // 注册自己作为事件处理器
        event_manager.add_handler(self as *mut dyn NetworkEventHandler);

        self.is_initialized = true;
    }

    /// 清理FrontSessionManager
    pub fn dispose(&mut self) {
        if !self.is_initialized {
            return;
        }

        // 关闭所有会话
        self.close_all();

        self.is_initialized = false;
        
        info!("FrontSessionManager disposed");
    }

    /// Get message processor reference
    pub fn get_msg_processor(&self) -> Option<Arc<dyn MsgProcessor>> {
        self.msg_processor.clone()
    }

    pub fn get_session(&self, session_id: u64) -> Option<&FrontSession> {
        self.sessions.get(&session_id)
    }

    pub fn get_session_mut(&mut self, session_id: u64) -> Option<&mut FrontSession> {
        self.sessions.get_mut(&session_id)
    }

    pub fn get_session_by_user_id(&self, user_id: u64) -> Option<&FrontSession> {
        self.sessions.values().find(|session| session.get_user_id() == Some(user_id))
    }

    pub fn get_session_by_user_id_mut(&mut self, user_id: u64) -> Option<&mut FrontSession> {
        self.sessions.values_mut().find(|session| session.get_user_id() == Some(user_id))
    }

    pub fn get_session_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn get_connected_count(&self) -> usize {
        self.sessions.values().filter(|s| s.is_connected()).count()
    }

    pub fn get_authenticated_count(&self) -> usize {
        self.sessions.values().filter(|s| s.is_authenticated()).count()
    }


    /// 创建一个TCP会话
    pub fn create_tcp_session(&mut self, tcp_stream: TcpStream, remote_addr: SocketAddr)  {
        let session_id = self.next_session_id;
        self.next_session_id += 1;

        // 创建 session，传入所有必要的参数
        let mut session = FrontSession::new_with_tcp(
            session_id, 
            tcp_stream, 
            remote_addr,
            self.event_queue.as_ref().expect("FrontSessionManager event_queue should be initialized").clone(),
            self.notify.as_ref().expect("FrontSessionManager notify should be initialized").clone(),
            ServerType::FrontTcp,
        );
        
        // Set message processor to the TCP connection if we have one
        if let Some(msg_processor_ref) = &self.msg_processor {
            debug!("FrontSessionManager has msg_processor, setting it to TCP connection {}", session_id);
            if let Some(connection) = session.get_connection_mut() {
                // Create a clone of the Arc to share the same processor instance
                let processor_clone = Arc::clone(msg_processor_ref);
                use crate::framework::session::front_session::ConnectionType;
                use crate::framework::network::connection::ConnectionTrait;
                // In create_tcp_session, connection is always TCP
                if let ConnectionType::Tcp(tcp) = connection {
                    tcp.set_msg_processor(processor_clone);
                    debug!("Message processor set successfully for TCP connection {}", session_id);
                    // 现在启动读取任务，在设置了msg_processor之后
                    tcp.start_read_task();
                    debug!("Started read task for TCP connection {}", session_id);
                } else {
                    error!("Expected TCP connection but got other type for session {}", session_id);
                }
            } else {
                error!("Failed to get connection for session {}", session_id);
            }
        } else {
            error!("FrontSessionManager has no msg_processor when creating TCP session {}", session_id);
        }
        
        self.sessions.insert(session_id, session);
        
        debug!("Created TCP front session {} from {} with read task", session_id, remote_addr);
    }
    
    /// 创建一个WebSocket会话
    pub fn create_websocket_session(&mut self, websocket: WebSocketStream<TcpStream>, remote_addr: SocketAddr) {
        let session_id = self.next_session_id;
        self.next_session_id += 1;

        // 获取事件队列和通知器
        let event_queue = self.event_queue.as_ref().expect("FrontSessionManager not initialized").clone();
        let notify = self.notify.as_ref().expect("FrontSessionManager not initialized").clone();
        
        let mut session = FrontSession::new_with_websocket(
            session_id, 
            websocket, 
            remote_addr,
            event_queue,
            notify,
            crate::framework::network::network_event_queue::ServerType::FrontWebSocket,
        );
        
        // Set message processor to the WebSocket connection if we have one
        if let Some(msg_processor_ref) = &self.msg_processor {
            if let Some(connection) = session.get_connection_mut() {
                // Create a clone of the Arc to share the same processor instance
                let processor_clone = Arc::clone(msg_processor_ref);
                use crate::framework::session::front_session::ConnectionType;
                use crate::framework::network::connection::ConnectionTrait;
                // In create_websocket_session, connection is always WebSocket
                if let ConnectionType::WebSocket(ws) = connection {
                    ws.set_msg_processor(processor_clone);
                    // 现在启动读取任务，在设置了msg_processor之后
                    ws.start_read_task();
                    debug!("Started read task for WebSocket connection {}", session_id);
                }
            }
        }
        
        self.sessions.insert(session_id, session);
        
        debug!("Created WebSocket front session {} from {}", session_id, remote_addr);
    }
    
    pub fn remove_session(&mut self, session_id: u64) -> bool {
        if let Some(mut session) = self.sessions.remove(&session_id) {
            session.close();
            debug!("Removed front session {}", session_id);
            true
        } else {
            error!("Attempt to remove non-existent front session {}", session_id);
            false
        }
    }

    pub fn update_all(&mut self) -> bool {
        let mut disconnected_sessions = Vec::new();
        
        for (session_id, session) in self.sessions.iter_mut() {
            if !session.is_connected() {
                disconnected_sessions.push(*session_id);
            }
        }

        for session_id in disconnected_sessions {
            self.remove_session(session_id);
        }

        true
    }

    pub fn close_all(&mut self) -> bool {
        for session in self.sessions.values_mut() {
            session.close();
        }
        self.sessions.clear();
        info!("Closed all front sessions");
        true
    }


    /// 处理新的TCP连接事件
    fn process_new_tcp_connection(&mut self, event_data: &mut crate::framework::network::network_event_queue::NetworkEventData) {
        // 取出tcp_stream
        if let Some(tcp_stream) = event_data.tcp_stream.take() {
            if let Some(remote_addr) = event_data.remote_addr {
                // 创建新的TCP front session
                self.create_tcp_session(tcp_stream, remote_addr);
            } else {
                error!("FrontSessionManager: NewTcpConnection event has no remote address");
            }
        } else {
            error!("FrontSessionManager: NewTcpConnection event has no TCP stream");
        }
    }

    /// 处理新的WebSocket连接事件
    fn process_new_websocket_connection(&mut self, event_data: &mut crate::framework::network::network_event_queue::NetworkEventData) {
        // 取出websocket_stream
        if let Some(websocket_stream) = event_data.websocket_stream.take() {
            if let Some(remote_addr) = event_data.remote_addr {
                // 创建新的WebSocket front session
                self.create_websocket_session(websocket_stream, remote_addr);
            } else {
                error!("FrontSessionManager: NewWebSocketConnection event has no remote address");
            }
        } else {
            error!("FrontSessionManager: NewWebSocketConnection event has no WebSocket stream");
        }
    }
    
    
    /// 处理连接断开事件
    fn process_disconnect(&mut self, event_data: &NetworkEventData) {
        let session_id = event_data.session_id;
        
        if self.sessions.remove(&session_id).is_some() {
            info!("FrontSessionManager: Removed session {} due to disconnect", session_id);
        } else {
            debug!("FrontSessionManager: Disconnect event for unknown session {}", session_id);
        }
    }

    /// 处理流数据异常事件
    fn process_stream_data_not_expected(&mut self, event_data: &NetworkEventData) {
        let session_id = event_data.session_id;
        
        info!("FrontSessionManager: Stream data not expected for session {}, closing session", session_id);
        
        if let Some(mut session) = self.sessions.remove(&session_id) {
            session.close();
            info!("FrontSessionManager: Closed session {} due to unexpected stream data", session_id);
        } else {
            debug!("FrontSessionManager: StreamDataNotExpected event for unknown session {}", session_id);
        }
    }
}

impl NetworkEventHandler for FrontSessionManager {
    fn handle_event(&mut self, event: &mut NetworkEventData) {
        if event.server_type != ServerType::FrontTcp {
            return;
        }
        match event.event_type {
            NetworkEventType::NewTcpConnection => {
                // 只处理 FrontTcp 类型的连接
                self.process_new_tcp_connection(event);
            }
            NetworkEventType::NewWebSocketConnection => {
                self.process_new_websocket_connection(event);
            }
            NetworkEventType::Disconnect => {
                // 处理连接断开
                self.process_disconnect(event);
            }
            NetworkEventType::StreamDataNotExpected => {
                // 处理流数据异常
                
                self.process_stream_data_not_expected(event);
            }
            _ => {
                // 其他事件不处理
            }
        }
    }
}