use crate::framework::msg::MsgProcessor;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tracing::{info, error, debug, warn};

/// 基础连接结构，包含所有连接类型共用的字段
pub struct Connection {
    /// 会话ID
    pub session_id: u64,
    /// 消息处理器
    pub msg_processor: Option<Arc<dyn MsgProcessor>>,
    /// 连接创建时间
    pub created_time: Instant,
}

impl Connection {
    /// 创建新的基础连接
    pub fn new(session_id: u64) -> Self {
        let now = Instant::now();
        Self {
            session_id,
            msg_processor: None,
            created_time: now,
        }
    }
}

/// 连接状态枚举
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    /// 未连接
    Disconnected,
    /// 连接中
    Connecting,
    /// 已连接
    Connected,
    /// 断开连接中
    Disconnecting,
}

/// 连接类型枚举
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionType {
    /// TCP连接
    Tcp,
    /// WebSocket连接
    WebSocket,
    /// 客户端连接
    Client,
}

/// 连接trait，定义所有连接类型的通用接口
pub trait ConnectionTrait: Send + Sync {
    /// 获取连接ID
    fn get_session_id(&self) -> u64;

    /// 获取远程地址
    fn get_remote_addr(&self) -> Option<SocketAddr>;

    /// 获取本地地址
    fn get_local_addr(&self) -> Option<SocketAddr>;

    /// 获取连接状态
    fn get_connection_state(&self) -> ConnectionState;

    /// 获取连接类型
    fn get_connection_type(&self) -> ConnectionType;

    /// 检查连接是否活跃
    fn is_active(&self) -> bool {
        self.get_connection_state() == ConnectionState::Connected
    }


    /// 初始化连接
    /// 
    /// # 返回值
    /// 成功返回true，失败返回false
    fn init(&mut self) -> bool;

    /// 启动连接
    /// 
    /// # 返回值
    /// 成功返回true，失败返回false
    fn start(&mut self) -> bool;

    /// 停止连接
    /// 
    /// # 返回值
    /// 成功返回true，失败返回false
    fn stop(&mut self) -> bool;

    /// 发送消息（发送protobuf消息）
    /// 
    /// # 参数
    /// * `message` - 要发送的protobuf消息，需要实现MessageIdSerialize + Clone + Send
    /// 
    /// # 返回值
    /// 成功返回true，失败返回false
    fn send_message<T>(&mut self, message: T) -> bool
    where
        T: crate::proto::messages::MessageIdSerialize + Clone + Send + 'static;

    /// 获取基础连接的引用
    fn get_connection(&self) -> &Connection;

    /// 获取基础连接的可变引用
    fn get_connection_mut(&mut self) -> &mut Connection;

    /// 销毁连接
    fn dispose(&mut self);

    /// 获取消息处理器的引用
    fn get_msg_processor(&self) -> Option<Arc<dyn MsgProcessor>> {
        self.get_connection().msg_processor.clone()
    }

    /// 设置消息处理器
    /// 
    /// # 参数
    /// * `processor` - 消息处理器
    /// 
    /// # 返回值
    /// 成功返回true，失败返回false
    fn set_msg_processor(&mut self, processor: Arc<dyn MsgProcessor>) -> bool {
        self.get_connection_mut().msg_processor = Some(processor);
        debug!("Connection {}: Message processor set", self.get_session_id());
        true
    }

    /// 移除消息处理器
    /// 
    /// # 返回值
    /// 返回被移除的消息处理器，如果没有则返回None
    fn remove_msg_processor(&mut self) -> Option<Arc<dyn MsgProcessor>> {
        let processor = self.get_connection_mut().msg_processor.take();
        if processor.is_some() {
            debug!("Connection {}: Message processor removed", self.get_session_id());
        }
        processor
    }

    /// 获取连接创建时间
    fn get_created_time(&self) -> std::time::Instant {
        self.get_connection().created_time
    }

    /// 获取最后活跃时间
    fn get_last_activity_time(&self) -> std::time::Instant {
        // 目前返回创建时间，未来可以扩展为真正的最后活跃时间
        self.get_connection().created_time
    }

    /// 设置连接状态
    /// 
    /// # 参数
    /// * `state` - 新的连接状态
    fn set_connection_state(&mut self, state: ConnectionState);

    /// 连接建立事件回调
    fn on_connected(&mut self) -> bool {
        debug!("Connection {} established", self.get_session_id());
        self.set_connection_state(ConnectionState::Connected);
        true
    }

    /// 连接断开事件回调
    fn on_disconnected(&mut self) -> bool {
        debug!("Connection {} disconnected", self.get_session_id());
        self.set_connection_state(ConnectionState::Disconnected);
        true
    }

    /// 连接错误事件回调
    /// 
    /// # 参数
    /// * `error_msg` - 错误信息
    fn on_error(&mut self, error_msg: &str) -> bool {
        error!("Connection {} error: {}", self.get_session_id(), error_msg);
        self.set_connection_state(ConnectionState::Disconnected);
        false
    }


    /// 检查连接是否超时
    /// 
    /// # 参数
    /// * `timeout_duration` - 超时时长（秒）
    /// 
    /// # 返回值
    /// 超时返回true，未超时返回false
    fn is_timeout(&self, timeout_duration: u64) -> bool {
        let elapsed = self.get_last_activity_time().elapsed();
        elapsed.as_secs() > timeout_duration
    }

    /// 处理接收到的原始数据
    /// 
    /// # 参数
    /// * `raw_data` - 原始数据
    /// 
    /// # 返回值
    /// 成功返回处理的字节数，失败返回0
    fn process_raw_data(&mut self, raw_data: &[u8]) -> usize {
        debug!("Processing {} bytes of raw data for connection {}", 
               raw_data.len(), self.get_session_id());

        if raw_data.is_empty() {
            return 0;
        }

        // TODO: 使用新的 decode_message 方法处理消息
        // 需要先从 raw_data 中解析出 message_id 和 length，
        // 然后创建 DynamicBuffer 并调用 decode_message
        
        raw_data.len()
    }
}