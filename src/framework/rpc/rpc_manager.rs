use super::router_manager::{RouterManager, RouterFunction};
use crate::framework::session::{BackSessionManager, FrontSessionManager, FrontSession};
use crate::proto::messages::protobuf::message::cluster::RpcForwardMessageBResponse;
use crate::proto::messages::MessageIdSerialize;
use tracing::{info, error, debug};

/// RPC管理器，负责管理RPC调用和路由
/// 
/// ## 重要说明
/// 本类为单例，被Server持有，所有方法都在主线程调用，不存在线程安全问题。
pub struct RpcManager {
    /// 路由管理器
    router_manager: RouterManager,
    /// 前端会话管理器指针
    front_session_manager: *mut FrontSessionManager,
    /// 后端会话管理器指针
    back_session_manager: *mut BackSessionManager,
}

impl RpcManager {
    /// 创建新的RPC管理器
    /// 
    /// 注意：本方法在主线程调用
    pub fn new() -> Self {
        Self {
            router_manager: RouterManager::new(),
            front_session_manager: std::ptr::null_mut(),
            back_session_manager: std::ptr::null_mut(),
        }
    }

    /// 初始化RPC管理器
    /// 
    /// 注意：本方法在主线程调用
    /// 
    /// # 参数
    /// * `front_session_manager` - 前端会话管理器
    /// * `back_session_manager` - 后端会话管理器
    pub fn init(
        &mut self,
        front_session_manager: &mut FrontSessionManager,
        back_session_manager: &mut BackSessionManager,
    ) -> bool {
        // 保存管理器指针
        self.front_session_manager = front_session_manager as *mut FrontSessionManager;
        self.back_session_manager = back_session_manager as *mut BackSessionManager;
        
        if !self.router_manager.init() {
            error!("Failed to initialize RouterManager");
            return false;
        }

        true
    }

    /// 获取路由管理器的引用
    pub fn get_router_manager(&self) -> &RouterManager {
        &self.router_manager
    }

    /// 获取路由管理器的可变引用
    pub fn get_router_manager_mut(&mut self) -> &mut RouterManager {
        &mut self.router_manager
    }

    /// 添加路由函数
    /// 
    /// # 参数
    /// * `server_type` - 服务器类型
    /// * `router_fn` - 路由函数
    /// 
    /// # 返回值
    /// 成功返回true，失败返回false
    pub fn add_router(&mut self, server_type: String, router_fn: RouterFunction) -> bool {
        self.router_manager.add_router(server_type, router_fn)
    }

    /// 移除路由器
    /// 
    /// # 参数
    /// * `server_type` - 服务器类型
    /// 
    /// # 返回值
    /// 成功返回true，失败返回false
    pub fn remove_router(&mut self, server_type: &str) -> bool {
        self.router_manager.remove_router(server_type)
    }

    /// 获取路由器数量
    pub fn get_router_count(&self) -> usize {
        self.router_manager.get_router_count()
    }

    /// 获取所有服务器类型
    pub fn get_server_types(&self) -> Vec<String> {
        self.router_manager.get_server_types()
    }

    /// 发送通知消息到指定服务器类型（带前端会话）
    /// 
    /// 注意：本方法在主线程调用
    /// 
    /// # 参数
    /// * `server_type` - 目标服务器类型
    /// * `msg` - 要发送的消息（必须实现MessageIdSerialize trait）
    /// * `front_session` - 前端会话，用于路由决策
    /// 
    /// # 返回值
    /// 成功发送返回true，无法找到有效的后端会话返回false
    pub fn call_with_session<T>(
        &mut self,
        server_type: &str,
        msg: T,
        front_session: &mut FrontSession,
    ) -> bool 
    where
        T: MessageIdSerialize + Clone + Send + 'static,
    {
        // 检查管理器是否已初始化
        if self.back_session_manager.is_null() {
            error!("RpcManager not initialized with session managers");
            return false;
        }
        
        debug!("Sending notify to server type: {} with front session", server_type);
        
        // 获取路由函数
        let router_fn = self.router_manager.get_router(server_type);
        
        // 安全地使用管理器指针
        unsafe {
            let back_session_manager = &mut *self.back_session_manager;
            
            // 执行路由函数，获取目标后端会话ID
            if let Some(target_session_id) = router_fn(
                server_type,
                Some(front_session),
                back_session_manager
            ) {
                debug!("Found target back session {} for server type: {}", target_session_id, server_type);
                
                // 从back_session_manager获取会话并发送消息
                if let Some(back_session) = back_session_manager.get_session_mut(target_session_id) {
                    if back_session.send_message(msg) {
                        debug!("Successfully sent notify to back session {}", target_session_id);
                        return true;
                    } else {
                        error!("Failed to send message to back session {}", target_session_id);
                    }
                } else {
                    error!("Back session {} not found", target_session_id);
                }
            } else {
                debug!("No suitable back session found for server type: {}", server_type);
            }
        }
        
        false
    }

    /// 发送通知消息到指定服务器ID
    /// 
    /// 注意：本方法在主线程调用
    /// 
    /// # 参数
    /// * `server_id` - 目标服务器ID
    /// * `msg` - 要发送的消息（必须实现MessageIdSerialize trait）
    /// 
    /// # 返回值
    /// 成功发送返回true，无法找到有效的后端会话返回false
    pub fn call_to_server<T>(
        &mut self,
        server_id: u64,
        msg: T,
    ) -> bool 
    where
        T: MessageIdSerialize + Clone + Send + 'static,
    {
        // 检查管理器是否已初始化
        if self.back_session_manager.is_null() {
            error!("RpcManager not initialized with session managers");
            return false;
        }
        
        debug!("Sending notify to server ID: {}", server_id);
        
        // 安全地使用管理器指针
        unsafe {
            let back_session_manager = &mut *self.back_session_manager;
            
            // 直接根据server_id获取会话并发送消息
            if let Some(back_session) = back_session_manager.get_session_mut(server_id) {
                if back_session.send_message(msg) {
                    debug!("Successfully sent notify to back session {}", server_id);
                    return true;
                } else {
                    error!("Failed to send message to back session {}", server_id);
                }
            } else {
                error!("Back session {} not found", server_id);
            }
        }
        
        false
    }

    /// 发送响应转发消息到前端会话
    /// 
    /// 注意：本方法在主线程调用
    /// 
    /// # 参数
    /// * `msg_unique_id` - 消息唯一ID
    /// * `front_session_id` - 前端会话ID
    /// * `response_msg` - 响应消息（必须实现MessageIdSerialize trait）
    /// 
    /// # 返回值
    /// 成功发送返回true，无法找到前端会话返回false
    pub fn send_response_forward_message<T>(
        &mut self,
        msg_unique_id: u32,
        front_session_id: u64,
        response_msg: T,
    ) -> bool 
    where
        T: MessageIdSerialize + Clone + Send + 'static,
    {
        // 检查管理器是否已初始化
        if self.front_session_manager.is_null() {
            error!("RpcManager not initialized with session managers");
            return false;
        }

        debug!("Sending response forward message to front session {}, msg_unique_id={}", 
               front_session_id, msg_unique_id);

        // 序列化响应消息
        match response_msg.serialize_to_buffer() {
            Ok(serialized_response) => {
                // 创建RPC转发响应
                let rpc_response = RpcForwardMessageBResponse {
                    msg_unique_id,
                    front_session_id,
                    meta: std::collections::HashMap::new(),  // 空的元数据map
                    msg_id: response_msg.msg_id() as u32,
                    message: serialized_response.to_vec(),
                };

                // 安全地使用前端会话管理器指针
                unsafe {
                    let front_session_manager = &mut *self.front_session_manager;
                    
                    // 获取前端会话并发送响应
                    if let Some(front_session) = front_session_manager.get_session_mut(front_session_id) {
                        if front_session.send_message(rpc_response) {
                            debug!("Successfully sent response forward message to front session {}", front_session_id);
                            return true;
                        } else {
                            error!("Failed to send response forward message to front session {}", front_session_id);
                        }
                    } else {
                        error!("Front session {} not found", front_session_id);
                    }
                }
            }
            Err(e) => {
                error!("Failed to serialize response message: {:?}", e);
            }
        }
        
        false
    }

    /// 销毁RPC管理器
    /// 
    /// 注意：本方法在主线程调用
    pub fn dispose(&mut self) {
        info!("Disposing RpcManager");
        
        // 清空管理器指针
        self.front_session_manager = std::ptr::null_mut();
        self.back_session_manager = std::ptr::null_mut();
        
        // 销毁路由管理器
        self.router_manager.dispose();
        
        info!("RpcManager disposed");
    }
}

impl Drop for RpcManager {
    fn drop(&mut self) {
        self.dispose();
    }
}