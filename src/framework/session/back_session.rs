use super::session_trait::SessionTrait;
use crate::framework::network::connection::tcp_connection::TcpConnection;
use crate::framework::network::connection::ConnectionTrait;
use crate::framework::network::network_event_queue::ServerType;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{info, error, debug};

pub struct BackSession {
    session_id: u64,
    server_id: u32,
    user_id: Option<u64>,
    remote_addr: Option<SocketAddr>,
    authenticated: bool,
    server_type: Option<String>,
    tcp_connection: Option<TcpConnection>,
}

impl BackSession {
    // ========== new methods ==========
    pub fn new(session_id: u64, server_id: u32, tcp_connection: Option<TcpConnection>, remote_addr: Option<SocketAddr>) -> Self {
        Self {
            session_id,
            server_id,
            user_id: None,
            remote_addr,
            authenticated: false,
            server_type: None,
            tcp_connection,
        }
    }

    /// 创建一个新的客户端会话
    pub fn new_client(
        session_id: u64,
        server_id: u32,
        host: &str,
        port: u16,
        event_queue: crate::framework::network::network_event_queue::NetworkEventQueue,
        notify: Arc<tokio::sync::Notify>,
    ) -> Self {
        let addr = format!("{}:{}", host, port);
        let remote_addr = addr.parse::<SocketAddr>().expect("Invalid socket address");
        
        let tcp_connection = TcpConnection::new_for_client(
            session_id, 
            remote_addr,
            event_queue,
            notify,
            ServerType::BackTcp,
        );
        
        Self::new(session_id, server_id, Some(tcp_connection), Some(remote_addr))
    }


    // ========== get/set methods ==========
    /// 获取服务器ID
    pub fn get_server_id(&self) -> u32 {
        self.server_id
    }

    /// 设置服务器ID
    pub fn set_server_id(&mut self, server_id: u32) {
        self.server_id = server_id;
    }

    /// 获取服务器类型
    pub fn get_server_type(&self) -> Option<&String> {
        self.server_type.as_ref()
    }

    /// 设置服务器类型
    pub fn set_server_type(&mut self, server_type: Option<String>) {
        self.server_type = server_type;
    }

    /// 获取TCP连接的可变引用
    pub fn get_tcp_connection_mut(&mut self) -> Option<&mut TcpConnection> {
        self.tcp_connection.as_mut()
    }


    /// 设置消息处理器到TCP连接
    pub fn set_msg_processor(&mut self, msg_processor: Arc<dyn crate::framework::msg::MsgProcessor>) {
        if let Some(ref mut tcp_connection) = self.tcp_connection {
            tcp_connection.set_msg_processor(msg_processor);
        } else {
            error!("No TCP connection available for back session {}", self.session_id);
        }
    }
}

impl SessionTrait for BackSession {
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
        if let Some(ref tcp_connection) = self.tcp_connection {
            tcp_connection.is_active()
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
            debug!("BackSession {} authentication status changed to: {}", self.session_id, authenticated);
        }
        true
    }

    fn close(&mut self) -> bool {
        if self.is_connected() {
            // 优雅关闭TCP连接
            if let Some(mut tcp_conn) = self.tcp_connection.take() {
                debug!("BackSession {} gracefully shutting down TCP connection", self.session_id);
                
                // 直接调用shutdown，不使用tokio::spawn
                // shutdown内部会处理异步操作
                tcp_conn.shutdown();
                debug!("BackSession {} initiated TCP connection shutdown", self.session_id);
            }
            
            debug!("BackSession {} closed", self.session_id);
        }
        true
    }

}

impl BackSession {
    /// 发送消息
    pub fn send_message<T>(&mut self, message: T) -> bool
    where
        T: crate::proto::messages::MessageIdSerialize + Clone + Send + 'static,
    {
        if !self.is_connected() {
            error!("Attempt to send message on disconnected back session");
            return false;
        }

        if let Some(ref mut tcp_conn) = self.tcp_connection {
            let result = tcp_conn.send_message(message);
            if result {
            } else {
                error!("BackSession {} failed to send message", self.session_id);
            }
            result
        } else {
            error!("BackSession {} has no tcp_connection", self.session_id);
            false
        }
    }
}