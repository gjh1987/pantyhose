use crate::framework::network::network_engine_event_manager::{NetworkEventHandler, NetworkEngineEventManager};
use crate::framework::network::network_event_queue::{NetworkEventData, NetworkEventType, ServerType};
use super::{BackSession, BackSessionManager};
use tracing::{debug, info, error, warn};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// 消息处理函数类型
/// 第一个参数是会话，第二个参数是消息
/// 支持闭包和函数指针
pub type BackMessageHandler = Box<dyn Fn(&mut BackSession, &dyn std::any::Any) + Send + Sync>;

/// 后端会话消息分发器
/// 作为单例被Server持有，负责处理后端服务器的消息事件
pub struct BackSessionMessageDispatcher {
    /// 消息处理器映射 (message_id -> handler)
    handlers: HashMap<u16, BackMessageHandler>,
    /// 后端会话管理器指针
    session_manager: *mut BackSessionManager,
}

// 安全性：BackSessionMessageDispatcher只在单线程环境中使用
unsafe impl Send for BackSessionMessageDispatcher {}

impl BackSessionMessageDispatcher {
    /// 创建新的消息分发器实例
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            session_manager: std::ptr::null_mut(),
        }
    }

    /// 初始化分发器并注册到NetworkEngineEventManager
    pub fn init(&mut self, event_manager: &mut NetworkEngineEventManager, session_manager: &mut BackSessionManager) -> bool {
        // 检查是否已初始化
        if !self.session_manager.is_null() {
            return false;
        }
        
        // 注册自己作为NewMessage事件的处理器
        event_manager.add_handler(self as *mut dyn NetworkEventHandler);
        
        // 保存会话管理器指针
        self.session_manager = session_manager as *mut BackSessionManager;
        
        true
    }
    

    /// 注册消息处理器
    pub fn register_handler(&mut self, message_id: u16, handler: BackMessageHandler) {
        self.handlers.insert(message_id, handler);
    }

    /// 移除指定消息ID的处理器
    pub fn unregister_handler(&mut self, message_id: u16) -> bool {
        if self.handlers.remove(&message_id).is_some() {
            debug!("Unregistered handler for message id {}", message_id);
            return true;
        }
        false
    }

    /// 清空所有处理器
    pub fn clear_all_handlers(&mut self) {
        let total_count = self.handlers.len();
        self.handlers.clear();
        debug!("Cleared {} message handlers", total_count);
    }

    /// 检查是否有指定消息ID的处理器
    pub fn has_handler(&self, message_id: u16) -> bool {
        self.handlers.contains_key(&message_id)
    }

    /// 获取所有已注册的消息ID
    pub fn get_registered_message_ids(&self) -> Vec<u16> {
        self.handlers.keys().cloned().collect()
    }

    /// 清理分发器
    /// 注意：由于NetworkEngineEventManager使用原始指针，无法安全地从中移除处理器
    /// 调用者应该确保在dispose后不再使用该实例
    pub fn dispose(&mut self) {
        if self.session_manager.is_null() {
            debug!("BackSessionMessageDispatcher already disposed");
            return;
        }
        
        self.clear_all_handlers();
        self.session_manager = std::ptr::null_mut();
        debug!("BackSessionMessageDispatcher disposed");
    }

}

impl NetworkEventHandler for BackSessionMessageDispatcher {
    fn handle_event(&mut self, event: &mut NetworkEventData) {
        // 只处理后端TCP服务器的NewMessage事件
        if event.event_type != NetworkEventType::NewMessage {
            return;
        }

        if event.server_type != ServerType::BackTcp {
            return;
        }

        debug!("BackSessionMessageDispatcher handling NewMessage from session {}", 
               event.session_id);

        // 处理消息
        if let Some(message_id) = event.message_id {
            if let Some(ref message) = event.message {
                debug!("Processing message id {} from back session {}", 
                       message_id, event.session_id);
                
                
                // 查找对应的消息处理器
                if let Some(handler) = self.handlers.get(&message_id) {
                    debug!("Found handler for message id {}", message_id);
                    
                    // 获取会话管理器并查找对应的会话
                    unsafe {
                        let session_manager = &mut *self.session_manager;
                        
                        // 首先尝试在已授权会话中查找
                        if let Some(session) = session_manager.get_session_mut(event.session_id) {
                            debug!("Found authorized session {} for message dispatch", event.session_id);
                            handler(session, message.as_ref());
                        } 
                        // 如果在已授权会话中找不到，尝试未授权会话
                        else if let Some(session) = session_manager.get_unauthorized_session_mut(event.session_id) {
                            debug!("Found unauthorized session {} for message dispatch", event.session_id);
                            handler(session, message.as_ref());
                        } else {
                            error!("BackSession {} not found in both authorized and unauthorized sessions for message dispatch", event.session_id);
                        }
                    }
                } else {
                    debug!("No handler found for message id {}", message_id);
                }
            }
        }
    }
}