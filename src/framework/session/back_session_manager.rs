use super::{BackSession, SessionTrait};
use crate::framework::msg::MsgProcessor;
use crate::framework::network::{NetworkEventHandler, NetworkEngineEventManager, network_event_queue::{NetworkEventData, NetworkEventType, NetworkEventQueue, ServerType}, connection::ConnectionTrait};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tracing::{info, error, debug};

pub struct BackSessionManager {
    sessions: HashMap<u64, BackSession>,
    unauthorized_sessions: HashMap<u64, BackSession>,
    next_session_id: u64,
    server_id: u32,
    msg_processor: Option<Arc<dyn MsgProcessor>>,
    event_queue: Option<NetworkEventQueue>,
    notify: Option<Arc<tokio::sync::Notify>>,
    is_initialized: bool,
}

// 安全性：BackSessionManager只在单线程环境中使用
unsafe impl Send for BackSessionManager {}

impl BackSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            unauthorized_sessions: HashMap::new(),
            next_session_id: 1,
            server_id: 0,
            msg_processor: None,
            event_queue: None,
            notify: None,
            is_initialized: false,
        }
    }

    /// 初始化BackSessionManager
    pub fn init(
        &mut self, 
        event_manager: &mut NetworkEngineEventManager,
        msg_processor: Arc<dyn MsgProcessor>,
        event_queue: NetworkEventQueue,
        notify: Arc<tokio::sync::Notify>
    ) {
        if self.is_initialized {
            return;
        }

        // 设置消息处理器
        self.msg_processor = Some(msg_processor);
        
        // 设置网络组件
        self.event_queue = Some(event_queue);
        self.notify = Some(notify);

        // 注册自己作为事件处理器
        event_manager.add_handler(self as *mut dyn NetworkEventHandler);

        self.is_initialized = true;
    }

    /// 清理BackSessionManager
    pub fn dispose(&mut self) {
        if !self.is_initialized {
            return;
        }

        // 关闭所有会话
        self.close_all();
        self.close_all_unauthorized();

        self.is_initialized = false;
        
        info!("BackSessionManager disposed");
    }

    /// Get message processor reference
    pub fn get_msg_processor(&self) -> Option<Arc<dyn MsgProcessor>> {
        self.msg_processor.clone()
    }

    pub fn get_session(&self, session_id: u64) -> Option<&BackSession> {
        self.sessions.get(&session_id)
    }

    pub fn get_session_mut(&mut self, session_id: u64) -> Option<&mut BackSession> {
        self.sessions.get_mut(&session_id)
    }

    pub fn get_session_by_user_id(&self, user_id: u64) -> Option<&BackSession> {
        self.sessions.values().find(|session| session.get_user_id() == Some(user_id))
    }

    pub fn get_session_by_user_id_mut(&mut self, user_id: u64) -> Option<&mut BackSession> {
        self.sessions.values_mut().find(|session| session.get_user_id() == Some(user_id))
    }

    pub fn find_session_by_server_id(&self, server_id: u32) -> Option<&BackSession> {
        self.sessions.values().find(|session| session.get_server_id() == server_id)
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

    pub fn get_unauthorized_count(&self) -> usize {
        self.unauthorized_sessions.len()
    }

    pub fn get_unauthorized_session(&self, session_id: u64) -> Option<&BackSession> {
        self.unauthorized_sessions.get(&session_id)
    }

    pub fn get_unauthorized_session_mut(&mut self, session_id: u64) -> Option<&mut BackSession> {
        self.unauthorized_sessions.get_mut(&session_id)
    }

    /// 获取所有会话（包括已授权和未授权的）
    pub fn get_all_sessions(&self) -> Vec<&BackSession> {
        self.sessions.values()
            .chain(self.unauthorized_sessions.values())
            .collect()
    }

    /// 对所有会话执行操作（包括已授权和未授权的）
    pub fn for_each_session<F>(&mut self, mut f: F) 
    where 
        F: FnMut(&mut BackSession)
    {
        for session in self.sessions.values_mut() {
            f(session);
        }
        for session in self.unauthorized_sessions.values_mut() {
            f(session);
        }
    }

    /// 获取所有会话的总数（包括已授权和未授权的）
    pub fn get_all_session_count(&self) -> usize {
        self.sessions.len() + self.unauthorized_sessions.len()
    }

    /// 在所有会话中查找指定会话ID的会话（包括已授权和未授权的）
    pub fn get_any_session(&self, session_id: u64) -> Option<&BackSession> {
        self.sessions.get(&session_id)
            .or_else(|| self.unauthorized_sessions.get(&session_id))
    }

    /// 在所有会话中查找指定会话ID的可变会话（包括已授权和未授权的）
    pub fn get_any_session_mut(&mut self, session_id: u64) -> Option<&mut BackSession> {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            Some(session)
        } else {
            self.unauthorized_sessions.get_mut(&session_id)
        }
    }

    /// 将未授权会话移动到已授权会话，并更新服务器信息
    pub fn authorize_session(&mut self, session_id: u64, server_id: u32, server_type: String) -> bool {
        if let Some(mut session) = self.unauthorized_sessions.remove(&session_id) {
            session.set_authenticated(true);  // 设置为已认证
            
            // 更新服务器信息
            session.set_server_id(server_id);
            session.set_server_type(Some(server_type.clone()));
            debug!("Updated session {} server_id to {}, server_type to {}", session_id, server_id, server_type);
            
            self.sessions.insert(session_id, session);
            debug!("Session {} moved from unauthorized to authorized", session_id);
            true
        } else {
            error!("Session {} not found in unauthorized sessions", session_id);
            false
        }
    }

    /// 移除未授权会话
    pub fn remove_unauthorized_session(&mut self, session_id: u64) -> bool {
        if let Some(mut session) = self.unauthorized_sessions.remove(&session_id) {
            session.close();
            debug!("Removed unauthorized session {}", session_id);
            true
        } else {
            error!("Attempt to remove non-existent unauthorized session {}", session_id);
            false
        }
    }

    /// 关闭所有未授权会话
    pub fn close_all_unauthorized(&mut self) -> bool {
        for session in self.unauthorized_sessions.values_mut() {
            session.close();
        }
        self.unauthorized_sessions.clear();
        info!("Closed all unauthorized sessions");
        true
    }

    /// 更新所有未授权会话
    pub fn update_all_unauthorized(&mut self) -> bool {
        let mut disconnected_sessions = Vec::new();
        
        for (session_id, session) in self.unauthorized_sessions.iter_mut() {
            if !session.is_connected() {
                disconnected_sessions.push(*session_id);
            }
        }

        for session_id in disconnected_sessions {
            self.remove_unauthorized_session(session_id);
        }

        true
    }

    /// 获取指定服务器类型的活跃session
    /// 
    /// # 参数
    /// * `server_type` - 服务器类型
    /// 
    /// # 返回值
    /// 活跃session的引用列表
    pub fn get_active_sessions(&self, server_type: &str) -> Vec<&BackSession> {
        self.sessions
            .values()
            .filter(|session| {
                session.is_connected() && 
                session.get_server_type()
                    .map_or(false, |s| s == server_type)
            })
            .collect()
    }

    /// 获取所有可用的服务器类型
    /// 
    /// # 返回值
    /// 活跃session中包含的所有服务器类型
    pub fn get_available_server_types(&self) -> Vec<String> {
        let mut server_types: std::collections::HashSet<String> = std::collections::HashSet::new();
        
        for session in self.sessions.values() {
            if session.is_connected() {
                if let Some(server_type) = session.get_server_type() {
                    server_types.insert(server_type.clone());
                }
            }
        }
        
        server_types.into_iter().collect()
    }

    /// 获取指定服务器类型的活跃session数量
    /// 
    /// # 参数
    /// * `server_type` - 服务器类型
    /// 
    /// # 返回值
    /// 活跃session的数量
    pub fn get_active_session_count(&self, server_type: &str) -> usize {
        self.sessions
            .values()
            .filter(|session| {
                session.is_connected() && 
                session.get_server_type()
                    .map_or(false, |s| s == server_type)
            })
            .count()
    }

    pub fn create_session(&mut self, server_id: u32, tcp_stream: TcpStream, remote_addr: SocketAddr) {
        let session_id = self.next_session_id;
        self.next_session_id += 1;

        // 创建 TCP 连接，传入所有必要的参数
        let mut tcp_connection = crate::framework::network::connection::TcpConnection::new(
            session_id, 
            tcp_stream, 
            remote_addr,
            self.event_queue.as_ref().expect("BackSessionManager event_queue should be initialized").clone(),
            self.notify.as_ref().expect("BackSessionManager notify should be initialized").clone(),
            ServerType::BackTcp,
        );
        
        // 设置消息处理器
        if let Some(msg_processor_ref) = &self.msg_processor {
            let processor_clone = Arc::clone(msg_processor_ref);
            tcp_connection.set_msg_processor(processor_clone);
            debug!("Set message processor for back session {}", session_id);
        } else {
            error!("No message processor available for back session {}", session_id);
        }
        
        // 启动读取任务
        tcp_connection.start_read_task();
        
        let mut session = BackSession::new(session_id, server_id, Some(tcp_connection), Some(remote_addr));
        session.set_authenticated(false);  // 明确设置为未认证
        
        // 新创建的session先加入未授权列表
        self.unauthorized_sessions.insert(session_id, session);
        
        debug!("Created unauthorized back session {} from {} for server {} with read task", session_id, remote_addr, server_id);
    }

    /// 创建新的客户端会话
    pub fn create_client_session(&mut self, server_id: u32, host: &str, port: u16) {
        let session_id = self.next_session_id;
        self.next_session_id += 1;

        let mut session = BackSession::new_client(
            session_id, 
            server_id, 
            host, 
            port,
            self.event_queue.as_ref().expect("BackSessionManager event_queue should be initialized").clone(),
            self.notify.as_ref().expect("BackSessionManager notify should be initialized").clone(),
        );
        
        // 设置消息处理器
        if let Some(msg_processor_ref) = &self.msg_processor {
            let processor_clone = Arc::clone(msg_processor_ref);
            session.set_msg_processor(processor_clone);
        } else {
            error!("No message processor available for client session {}", session_id);
        }
        
        // 获取远程地址并调用connect_to
        let addr_str = format!("{}:{}", host, port);
        if let Ok(socket_addr) = addr_str.parse::<SocketAddr>() {
            if let Some(tcp_connection) = session.get_tcp_connection_mut() {
                if let (Some(ref event_queue), Some(ref notify)) = (&self.event_queue, &self.notify) {
                    tcp_connection.connect_to(socket_addr, event_queue.clone(), Some(Arc::clone(notify)));
                    debug!("TCP connection connecting to {}", socket_addr);
                } else {
                    error!("BackSessionManager: Network components not set, cannot connect");
                }
            }
        } else {
            error!("Invalid address: {}:{}", host, port);
        }
        
        session.set_authenticated(false);  // 客户端会话初始状态为未认证
        self.unauthorized_sessions.insert(session_id, session);
        debug!("Created client back session {} for server {} connecting to {}:{}", session_id, server_id, host, port);
    }

    pub fn remove_session(&mut self, session_id: u64) -> bool {
        if let Some(mut session) = self.sessions.remove(&session_id) {
            session.close();
            debug!("Removed back session {}", session_id);
            true
        } else {
            error!("Attempt to remove non-existent back session {}", session_id);
            false
        }
    }

    /// 移除验证token失败的会话
    /// 这个方法会记录详细的安全日志，并立即关闭连接
    pub fn remove_bad_token_session(&mut self, session_id: u64, server_id: u32) -> bool {
        // 先尝试在已授权会话中查找
        if let Some(mut session) = self.sessions.remove(&session_id) {
            error!("Security: Removing session {} from server_id {} due to invalid token (authorized session)", 
                   session_id, server_id);
            session.close();
            info!("Security: Session {} closed and removed due to invalid token authentication", session_id);
            return true;
        }
        
        // 再尝试在未授权会话中查找
        if let Some(mut session) = self.unauthorized_sessions.remove(&session_id) {
            error!("Security: Removing session {} from server_id {} due to invalid token (unauthorized session)", 
                   session_id, server_id);
            session.close();
            info!("Security: Unauthorized session {} closed and removed due to invalid token authentication", session_id);
            return true;
        }
        
        error!("Security: Failed to remove bad token session {} - session not found", session_id);
        false
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

        // 同时更新未授权会话
        self.update_all_unauthorized();

        true
    }

    pub fn close_all(&mut self) -> bool {
        for session in self.sessions.values_mut() {
            session.close();
        }
        self.sessions.clear();
        info!("Closed all back sessions");
        true
    }



    /// 处理新的TCP连接事件
    fn process_new_tcp_connection(&mut self, event_data: &mut NetworkEventData) {
        // 取出tcp_stream
        if let Some(tcp_stream) = event_data.tcp_stream.take() {
            if let Some(remote_addr) = event_data.remote_addr {
                // 创建新的back session，会自动加入unauthorized_sessions
                self.create_session(self.server_id, tcp_stream, remote_addr);
                debug!("BackSessionManager: Created new unauthorized back session for connection from {}", remote_addr);
            } else {
                error!("BackSessionManager: NewTcpConnection event has no remote address");
            }
        } else {
            error!("BackSessionManager: NewTcpConnection event has no TCP stream");
        }
    }

    /// 处理客户端连接成功事件
    fn process_client_connect_event(&mut self, event_data: &mut NetworkEventData) {
        let session_id = event_data.session_id;
        
        // 先检查session是否存在（可能是front session，所以不存在是正常的）
        if let Some(session) = self.get_any_session_mut(session_id) {
            // session存在，再取出tcp_stream
            if let Some(tcp_stream) = event_data.tcp_stream.take() {
                // 获取tcp_connection并设置stream
                if let Some(tcp_connection) = session.get_tcp_connection_mut() {
                    tcp_connection.set_tcp_stream(tcp_stream);
                    debug!("BackSessionManager: Successfully set TCP stream for session {}", session_id);
                } else {
                    error!("BackSessionManager: Session {} has no TCP connection", session_id);
                }
            } else {
                error!("BackSessionManager: ClientConnectSuccess event for session {} has no TCP stream", session_id);
            }
        }
    }
    
    
    /// 处理连接断开事件
    fn process_disconnect(&mut self, event_data: &NetworkEventData) {
        let session_id = event_data.session_id;
        
        // 先在未授权列表中查找并移除
        if self.unauthorized_sessions.remove(&session_id).is_some() {
            info!("BackSessionManager: Removed unauthorized session {} due to disconnect", session_id);
            return;
        }
        
        // 再在授权列表中查找并移除
        if self.sessions.remove(&session_id).is_some() {
            info!("BackSessionManager: Removed authorized session {} due to disconnect", session_id);
        } else {
            debug!("BackSessionManager: Disconnect event for unknown session {}", session_id);
        }
    }

    /// 处理流数据异常事件
    fn process_stream_data_not_expected(&mut self, event_data: &NetworkEventData) {
        let session_id = event_data.session_id;
        
        info!("BackSessionManager: Stream data not expected for session {}, closing session", session_id);
        
        // 先在未授权列表中查找并移除
        if let Some(mut session) = self.unauthorized_sessions.remove(&session_id) {
            session.close();
            info!("BackSessionManager: Closed unauthorized session {} due to unexpected stream data", session_id);
            return;
        }
        
        // 再在授权列表中查找并移除
        if let Some(mut session) = self.sessions.remove(&session_id) {
            session.close();
            info!("BackSessionManager: Closed authorized session {} due to unexpected stream data", session_id);
        } else {
            debug!("BackSessionManager: StreamDataNotExpected event for unknown session {}", session_id);
        }
    }
}

impl NetworkEventHandler for BackSessionManager {
    fn handle_event(&mut self, event: &mut NetworkEventData) {
        if event.server_type != ServerType::BackTcp {
                return;
        }
        match event.event_type {
            NetworkEventType::NewTcpConnection => {
                // 只处理BackTcp类型的连接
                self.process_new_tcp_connection(event);
            }
            NetworkEventType::ClientConnectSuccess => {
                self.process_client_connect_event(event);
            }
            NetworkEventType::Disconnect => {
                // 处理连接断开
                if event.server_type == ServerType::BackTcp {
                    self.process_disconnect(event);
                }
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