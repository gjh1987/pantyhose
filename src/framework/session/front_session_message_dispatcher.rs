use crate::framework::network::network_engine_event_manager::{NetworkEventHandler, NetworkEngineEventManager};
use crate::framework::network::network_event_queue::{NetworkEventData, NetworkEventType, ServerType};
use super::{FrontSession, FrontSessionManager};
use tracing::{debug, info, error};
use std::collections::HashMap;

/// 消息处理函数类型
/// 第一个参数是会话，第二个参数是消息
/// 支持闭包和函数指针
pub type FrontMessageHandler = Box<dyn Fn(&mut FrontSession, &dyn std::any::Any) + Send + Sync>;

/// 前端会话消息分发器
/// 作为单例被Server持有，负责处理客户端连接的消息事件
pub struct FrontSessionMessageDispatcher {
    /// 消息处理器映射 (message_id -> handler)
    handlers: HashMap<u16, FrontMessageHandler>,
    /// 前端会话管理器指针
    session_manager: *mut FrontSessionManager,
}

// 安全性：FrontSessionMessageDispatcher只在单线程环境中使用
unsafe impl Send for FrontSessionMessageDispatcher {}

impl FrontSessionMessageDispatcher {
    /// 创建新的消息分发器实例
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            session_manager: std::ptr::null_mut(),
        }
    }

    /// 初始化分发器并注册到NetworkEngineEventManager
    pub fn init(&mut self, event_manager: &mut NetworkEngineEventManager, session_manager: &mut FrontSessionManager) -> bool {
        // 检查是否已初始化
        if !self.session_manager.is_null() {
            return false;
        }
        
        // 注册自己作为NewMessage事件的处理器
        event_manager.add_handler(self as *mut dyn NetworkEventHandler);
        
        // 保存会话管理器指针
        self.session_manager = session_manager as *mut FrontSessionManager;
        
        true
    }

    /// 注册消息处理器
    pub fn register_handler(&mut self, message_id: u16, handler: FrontMessageHandler) {
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
            debug!("FrontSessionMessageDispatcher already disposed");
            return;
        }
        
        self.clear_all_handlers();
        self.session_manager = std::ptr::null_mut();
        debug!("FrontSessionMessageDispatcher disposed");
    }

}

impl NetworkEventHandler for FrontSessionMessageDispatcher {
    fn handle_event(&mut self, event: &mut NetworkEventData) {
        // 只处理前端服务器的NewMessage事件
        if event.event_type != NetworkEventType::NewMessage {
            return;
        }

        // 处理TCP和WebSocket两种前端连接
        match event.server_type {
            ServerType::FrontTcp | ServerType::FrontWebSocket => {
                debug!("FrontSessionMessageDispatcher handling NewMessage from {:?} session {}", 
                       event.server_type, event.session_id);

                // 处理消息
                if let Some(message_id) = event.message_id {
                    if let Some(ref message) = event.message {
                        debug!("Processing message id {} from front session {} (type: {:?})", 
                               message_id, event.session_id, event.server_type);
                        
                        // 查找对应的消息处理器
                        if let Some(handler) = self.handlers.get(&message_id) {
                            debug!("Found handler for message id {}", message_id);
                            
                            // 获取会话管理器并查找对应的会话
                            unsafe {
                                let session_manager = &mut *self.session_manager;
                                if let Some(session) = session_manager.get_session_mut(event.session_id) {
                                    // 执行处理器
                                    handler(session, message.as_ref());
                                } else {
                                    error!("FrontSession {} not found for message dispatch", event.session_id);
                                }
                            }
                        } else {
                            debug!("No handler found for message id {}", message_id);
                        }
                    }
                }
            }
            _ => {
                // 不处理后端服务器的消息
                return;
            }
        }
    }
}