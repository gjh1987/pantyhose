use crate::framework::session::BackSession;
use tracing::{debug, error, warn};
use std::collections::HashMap;
use std::any::Any;

/// RPC请求消息处理函数类型
/// 参数：会话、消息唯一ID、前端会话ID、消息ID、内部消息
pub type RpcRequestHandler = Box<dyn Fn(&mut BackSession, u32, u64, u32, &dyn Any) + Send + Sync>;

/// RPC通知消息处理函数类型
/// 参数：会话、前端会话ID、消息ID、内部消息（通知消息没有唯一ID但有前端会话ID）
pub type RpcNotifyHandler = Box<dyn Fn(&mut BackSession, u64, u32, &dyn Any) + Send + Sync>;

/// RPC消息分发器
/// 负责将RPC转发消息中解码出的内部消息分发给相应的业务处理器
pub struct RpcMessageDispatcher {
    /// 请求消息处理器映射 (message_id -> handler)
    request_handlers: HashMap<u16, RpcRequestHandler>,
    /// 通知消息处理器映射 (message_id -> handler)
    notify_handlers: HashMap<u16, RpcNotifyHandler>,
}

impl RpcMessageDispatcher {
    /// 创建新的RPC消息分发器实例
    pub fn new() -> Self {
        Self {
            request_handlers: HashMap::new(),
            notify_handlers: HashMap::new(),
        }
    }

    /// 注册RPC请求消息处理器
    /// 
    /// # 参数
    /// * `message_id` - 消息ID
    /// * `handler` - 请求消息处理器
    pub fn register_request_handler(&mut self, message_id: u16, handler: RpcRequestHandler) {
        self.request_handlers.insert(message_id, handler);
        debug!("Registered RPC request handler for message id {}", message_id);
    }

    /// 注册RPC通知消息处理器
    /// 
    /// # 参数
    /// * `message_id` - 消息ID
    /// * `handler` - 通知消息处理器
    pub fn register_notify_handler(&mut self, message_id: u16, handler: RpcNotifyHandler) {
        self.notify_handlers.insert(message_id, handler);
        debug!("Registered RPC notify handler for message id {}", message_id);
    }

    /// 移除指定消息ID的请求处理器
    /// 
    /// # 参数
    /// * `message_id` - 消息ID
    /// 
    /// # 返回值
    /// 成功移除返回true，消息ID不存在返回false
    pub fn unregister_request_handler(&mut self, message_id: u16) -> bool {
        if self.request_handlers.remove(&message_id).is_some() {
            debug!("Unregistered RPC request handler for message id {}", message_id);
            return true;
        }
        false
    }

    /// 移除指定消息ID的通知处理器
    /// 
    /// # 参数
    /// * `message_id` - 消息ID
    /// 
    /// # 返回值
    /// 成功移除返回true，消息ID不存在返回false
    pub fn unregister_notify_handler(&mut self, message_id: u16) -> bool {
        if self.notify_handlers.remove(&message_id).is_some() {
            debug!("Unregistered RPC notify handler for message id {}", message_id);
            return true;
        }
        false
    }

    /// 分发请求消息到相应的处理器
    /// 
    /// # 参数
    /// * `message_id` - 消息ID
    /// * `session` - 后端会话
    /// * `msg_unique_id` - 消息唯一ID
    /// * `front_session_id` - 前端会话ID
    /// * `inner_message` - 解码后的内部消息
    /// 
    /// # 返回值
    /// 找到处理器并成功分发返回true，否则返回false
    pub fn dispatch_request_message(&self, message_id: u16, session: &mut BackSession, msg_unique_id: u32, front_session_id: u64, inner_message: &dyn Any) -> bool {
        if let Some(handler) = self.request_handlers.get(&message_id) {
            debug!("Dispatching RPC request message with id {} to handler", message_id);
            handler(session, msg_unique_id, front_session_id, message_id as u32, inner_message);
            true
        } else {
            warn!("No RPC request handler found for message id {}", message_id);
            false
        }
    }

    /// 分发通知消息到相应的处理器
    /// 
    /// # 参数
    /// * `message_id` - 消息ID
    /// * `session` - 后端会话
    /// * `front_session_id` - 前端会话ID
    /// * `inner_message` - 解码后的内部消息
    /// 
    /// # 返回值
    /// 找到处理器并成功分发返回true，否则返回false
    pub fn dispatch_notify_message(&self, message_id: u16, session: &mut BackSession, front_session_id: u64, inner_message: &dyn Any) -> bool {
        if let Some(handler) = self.notify_handlers.get(&message_id) {
            debug!("Dispatching RPC notify message with id {} to handler", message_id);
            handler(session, front_session_id, message_id as u32, inner_message);
            true
        } else {
            warn!("No RPC notify handler found for message id {}", message_id);
            false
        }
    }

    /// 检查是否有指定消息ID的请求处理器
    /// 
    /// # 参数
    /// * `message_id` - 消息ID
    /// 
    /// # 返回值
    /// 有处理器返回true，否则返回false
    pub fn has_request_handler(&self, message_id: u16) -> bool {
        self.request_handlers.contains_key(&message_id)
    }

    /// 检查是否有指定消息ID的通知处理器
    /// 
    /// # 参数
    /// * `message_id` - 消息ID
    /// 
    /// # 返回值
    /// 有处理器返回true，否则返回false
    pub fn has_notify_handler(&self, message_id: u16) -> bool {
        self.notify_handlers.contains_key(&message_id)
    }

    /// 获取所有已注册的请求消息ID
    /// 
    /// # 返回值
    /// 包含所有已注册请求消息ID的向量
    pub fn get_registered_request_message_ids(&self) -> Vec<u16> {
        self.request_handlers.keys().cloned().collect()
    }

    /// 获取所有已注册的通知消息ID
    /// 
    /// # 返回值
    /// 包含所有已注册通知消息ID的向量
    pub fn get_registered_notify_message_ids(&self) -> Vec<u16> {
        self.notify_handlers.keys().cloned().collect()
    }

    /// 获取已注册请求处理器的数量
    /// 
    /// # 返回值
    /// 已注册请求处理器的数量
    pub fn get_request_handler_count(&self) -> usize {
        self.request_handlers.len()
    }

    /// 获取已注册通知处理器的数量
    /// 
    /// # 返回值
    /// 已注册通知处理器的数量
    pub fn get_notify_handler_count(&self) -> usize {
        self.notify_handlers.len()
    }

    /// 清空所有处理器
    pub fn clear_all_handlers(&mut self) {
        let request_count = self.request_handlers.len();
        let notify_count = self.notify_handlers.len();
        self.request_handlers.clear();
        self.notify_handlers.clear();
        debug!("Cleared {} RPC request handlers and {} notify handlers", request_count, notify_count);
    }

    /// 清理分发器
    pub fn dispose(&mut self) {
        debug!("Disposing RpcMessageDispatcher with {} request handlers and {} notify handlers", 
               self.request_handlers.len(), self.notify_handlers.len());
        self.clear_all_handlers();
    }
}

impl Default for RpcMessageDispatcher {
    fn default() -> Self {
        Self::new()
    }
}