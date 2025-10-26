use tracing::{info, debug};
use super::network_event_queue::{NetworkEventData, NetworkEventType};

// 事件处理器trait
pub trait NetworkEventHandler: Send {
    /// 处理网络事件
    fn handle_event(&mut self, event: &mut NetworkEventData);
}

pub struct NetworkEngineEventManager {
    // 事件处理器指针列表
    handlers: Vec<*mut dyn NetworkEventHandler>,
}

impl NetworkEngineEventManager {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }
    
    // 添加事件处理器（通过指针）
    pub fn add_handler(&mut self, handler: *mut dyn NetworkEventHandler) {
        self.handlers.push(handler);
    }
    
    
    // 分发事件（同步）
    pub fn dispatch(&mut self, event: &mut NetworkEventData) {
        debug!("Dispatching event {:?} to {} handlers", event.event_type, self.handlers.len());
        
        for handler_ptr in &self.handlers {
            unsafe {
                if let Some(handler) = handler_ptr.as_mut() {
                    handler.handle_event(event);
                }
            }
        }
    }
    
    
    // 清空所有处理器（同步）
    pub fn clear_all(&mut self) {
        let count = self.handlers.len();
        self.handlers.clear();
        
        if count > 0 {
            info!("Cleared {} handlers", count);
        }
    }
    
    // 获取处理器数量（同步）
    pub fn get_handler_count(&self) -> usize {
        self.handlers.len()
    }
}