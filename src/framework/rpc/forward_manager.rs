use crate::framework::session::{FrontSession, BackSession, FrontSessionMessageDispatcher, BackSessionMessageDispatcher};
use crate::framework::session::session_trait::SessionTrait;
use crate::framework::rpc::{RouterManager, RpcManager, RpcMessageDispatcher};
use crate::framework::session::{FrontSessionManager, BackSessionManager};
use crate::proto::messages::protobuf::cluster::{
    RpcMessageFRequest, RpcMessageFNotify, RpcMessageFResponse,
    RpcForwardMessageBRequest, RpcForwardMessageBNotify, RpcForwardMessageBResponse
};
use crate::proto::messages::protobuf::message::protobuf_message_id::{
    MSG_ID_RPC_MESSAGE_F_REQUEST, MSG_ID_RPC_MESSAGE_F_NOTIFY,
    MSG_ID_RPC_FORWARD_MESSAGE_B_REQUEST, MSG_ID_RPC_FORWARD_MESSAGE_B_NOTIFY,
    MSG_ID_RPC_FORWARD_MESSAGE_B_RESPONSE, MessageFactory
};
use crate::framework::data::DynamicBuffer;
use crate::proto::messages::MessageIdSerialize;
use crate::framework::task::TaskManager;
use tracing::{debug, info, error, warn};

/// 转发管理器
/// 处理RPC消息转发，负责将前端消息路由到正确的后端服务器
/// 
/// ## 重要说明
/// 本类为单例，被Server持有，所有方法都在主线程调用，不存在线程安全问题。
/// 因此内部使用原始指针是安全的。
pub struct ForwardManager {
    /// RPC管理器指针，用于发送消息
    rpc_manager: *mut RpcManager,
    /// 前端会话管理器指针，用于获取前端会话
    front_session_manager: *mut FrontSessionManager,
    /// RPC消息分发器指针，用于分发解码后的内部消息
    rpc_message_dispatcher: *mut RpcMessageDispatcher,
}

// 安全性：ForwardManager只在单线程环境中使用
unsafe impl Send for ForwardManager {}

// 包装器结构体，使原始指针满足Send + Sync要求（仅用于满足闭包类型约束）
// 实际运行时所有调用都在主线程，不会有并发访问
#[derive(Clone, Copy)]
struct ForwardManagerPtr(*mut ForwardManager);
unsafe impl Send for ForwardManagerPtr {}
unsafe impl Sync for ForwardManagerPtr {}

impl ForwardManager {
    /// 创建新的转发管理器
    pub fn new() -> Self {
        Self {
            rpc_manager: std::ptr::null_mut(),
            front_session_manager: std::ptr::null_mut(),
            rpc_message_dispatcher: std::ptr::null_mut(),
        }
    }
    
    /// 初始化转发管理器并注册消息处理器
    /// 
    /// 注意：本方法在主线程调用
    pub fn init(
        &mut self,
        front_dispatcher: &mut FrontSessionMessageDispatcher,
        back_dispatcher: &mut BackSessionMessageDispatcher,
        router_manager: &mut RouterManager,
        front_session_manager: &mut FrontSessionManager,
        back_session_manager: &mut BackSessionManager,
        task_manager: &mut TaskManager,
        rpc_manager: &mut RpcManager,
        rpc_message_dispatcher: &mut RpcMessageDispatcher,
    ) -> bool {
        // 保存RPC管理器和前端会话管理器指针
        self.rpc_manager = rpc_manager as *mut RpcManager;
        self.front_session_manager = front_session_manager as *mut FrontSessionManager;
        self.rpc_message_dispatcher = rpc_message_dispatcher as *mut RpcMessageDispatcher;
        
        // 直接使用ForwardManagerPtr包装原始指针
        // 因为所有调用都在主线程，不需要Mutex
        let forward_manager_ptr = ForwardManagerPtr(self as *mut ForwardManager);
        
        // 注册前端消息处理器：RpcMessageFRequest
        front_dispatcher.register_handler(
            MSG_ID_RPC_MESSAGE_F_REQUEST,
            Box::new(move |session, message| {
                if let Some(request) = message.downcast_ref::<RpcMessageFRequest>() {
                    // 在unsafe块内部才解引用，避免闭包直接捕获原始指针
                    unsafe {
                        let ptr = forward_manager_ptr;
                        let forward_manager = &mut *ptr.0;
                        forward_manager.handle_rpc_message_request(session, request);
                    }
                }
            }),
        );
        
        // 重新创建指针（因为前一个已经被move）
        let forward_manager_ptr = ForwardManagerPtr(self as *mut ForwardManager);
        
        // 注册前端消息处理器：RpcMessageFNotify
        front_dispatcher.register_handler(
            MSG_ID_RPC_MESSAGE_F_NOTIFY,
            Box::new(move |session, message| {
                if let Some(notify) = message.downcast_ref::<RpcMessageFNotify>() {
                    // 在unsafe块内部才解引用，避免闭包直接捕获原始指针
                    unsafe {
                        let ptr = forward_manager_ptr;
                        let forward_manager = &mut *ptr.0;
                        forward_manager.handle_rpc_message_notify(session, notify);
                    }
                }
            }),
        );
        
        // 重新创建指针（因为前一个已经被move）
        let forward_manager_ptr = ForwardManagerPtr(self as *mut ForwardManager);
        
        // 注册后端消息处理器：RpcForwardMessageBRequest
        back_dispatcher.register_handler(
            MSG_ID_RPC_FORWARD_MESSAGE_B_REQUEST,
            Box::new(move |session, message| {
                if let Some(request) = message.downcast_ref::<RpcForwardMessageBRequest>() {
                    unsafe {
                        let ptr = forward_manager_ptr;
                        let forward_manager = &mut *ptr.0;
                        forward_manager.handle_rpc_forward_message_request(session, request);
                    }
                }
            }),
        );
        
        // 重新创建指针
        let forward_manager_ptr = ForwardManagerPtr(self as *mut ForwardManager);
        
        // 注册后端消息处理器：RpcForwardMessageBNotify
        back_dispatcher.register_handler(
            MSG_ID_RPC_FORWARD_MESSAGE_B_NOTIFY,
            Box::new(move |session, message| {
                if let Some(notify) = message.downcast_ref::<RpcForwardMessageBNotify>() {
                    unsafe {
                        let ptr = forward_manager_ptr;
                        let forward_manager = &mut *ptr.0;
                        forward_manager.handle_rpc_forward_message_notify(session, notify);
                    }
                }
            }),
        );
        
        // 重新创建指针
        let forward_manager_ptr = ForwardManagerPtr(self as *mut ForwardManager);
        
        // 注册后端消息处理器：RpcForwardMessageBResponse
        back_dispatcher.register_handler(
            MSG_ID_RPC_FORWARD_MESSAGE_B_RESPONSE,
            Box::new(move |session, message| {
                if let Some(response) = message.downcast_ref::<RpcForwardMessageBResponse>() {
                    // 在unsafe块内部才解引用，避免闭包直接捕获原始指针
                    unsafe {
                        let ptr = forward_manager_ptr;
                        let forward_manager = &mut *ptr.0;
                        forward_manager.handle_rpc_forward_message_response(session, response);
                    }
                }
            }),
        );

        true
    }
    
    
    /// 处理RpcMessageFRequest
    /// 
    /// 注意：本方法在主线程调用
    pub fn handle_rpc_message_request(&mut self, front_session: &mut FrontSession, request: &RpcMessageFRequest) {
        let session_id = front_session.get_session_id();
        debug!("Handling RpcMessageFRequest from session {}: msg_unique_id={}, server_type={}, msg_id={}, message_size={}", 
               session_id, request.msg_unique_id, request.server_type, request.msg_id, request.message.len());
        
        // 创建RpcForwardMessageBRequest，包含前端会话ID和元数据
        let forward_request = RpcForwardMessageBRequest {
            msg_unique_id: request.msg_unique_id,
            front_session_id: session_id,  // 前端会话ID
            meta: std::collections::HashMap::new(),  // 空的元数据map，可以后续扩展
            msg_id: request.msg_id,
            message: request.message.clone(),
        };
        
        // 直接调用RPC发送消息
        unsafe {
            if !self.rpc_manager.is_null() {
                let rpc_manager = &mut *self.rpc_manager;
                let success = rpc_manager.call_with_session(&request.server_type, forward_request, front_session);
                if success {
                    debug!("Successfully sent RpcForwardMessageBRequest to server_type {}", request.server_type);
                } else {
                    error!("Failed to send RpcForwardMessageBRequest to server_type {}", request.server_type);
                }
            } else {
                error!("RpcManager is null");
            }
        }
    }
    
    /// 处理RpcMessageFNotify
    /// 
    /// 注意：本方法在主线程调用
    pub fn handle_rpc_message_notify(&mut self, front_session: &mut FrontSession, notify: &RpcMessageFNotify) {
        let session_id = front_session.get_session_id();
        debug!("Handling RpcMessageFNotify from session {}: server_type={}, msg_id={}, message_size={}", 
               session_id, notify.server_type, notify.msg_id, notify.message.len());
        
        // 创建RpcForwardMessageBNotify
        let forward_notify = RpcForwardMessageBNotify {
            msg_id: notify.msg_id,
            front_session_id: session_id,
            meta: std::collections::HashMap::new(),  // 空的元数据map，可以后续扩展
            message: notify.message.clone(),
        };
        
        // 直接调用RPC发送消息
        unsafe {
            if !self.rpc_manager.is_null() {
                let rpc_manager = &mut *self.rpc_manager;
                let success = rpc_manager.call_with_session(&notify.server_type, forward_notify, front_session);
                if success {
                    debug!("Successfully sent RpcForwardMessageBNotify to server_type {}", notify.server_type);
                } else {
                    error!("Failed to send RpcForwardMessageBNotify to server_type {}", notify.server_type);
                }
            } else {
                error!("RpcManager is null");
            }
        }
    }
    
    /// 处理后端RpcForwardMessageBRequest（消息转发到业务处理器）
    /// 
    /// 注意：本方法在主线程调用
    pub fn handle_rpc_forward_message_request(&mut self, back_session: &mut BackSession, request: &RpcForwardMessageBRequest) {
        debug!("Handling RpcForwardMessageBRequest: msg_unique_id={}, msg_id={}, message_size={}", 
               request.msg_unique_id, request.msg_id, request.message.len());
        
        // 根据msg_id反序列化内部消息
        let mut buffer = DynamicBuffer::new(request.message.len(), 1024);
        buffer.write_slice(&request.message);
        
        if let Some(inner_message) = MessageFactory::decode_message(request.msg_id as u16, &mut buffer, request.message.len()) {
            debug!("Successfully decoded inner message with msg_id={}", request.msg_id);
            
            // 通过RPC消息分发器分发内部消息给业务处理器
            unsafe {
                if !self.rpc_message_dispatcher.is_null() {
                    let rpc_dispatcher = &mut *self.rpc_message_dispatcher;
                    if rpc_dispatcher.dispatch_request_message(request.msg_id as u16, back_session, request.msg_unique_id, request.front_session_id, inner_message.as_ref()) {
                        debug!("Successfully dispatched RPC message with msg_id={}", request.msg_id);
                    } else {
                        warn!("No handler found for RPC message with msg_id={}", request.msg_id);
                    }
                } else {
                    error!("RpcMessageDispatcher is null");
                }
            }
        } else {
            error!("Failed to decode inner message with msg_id={}", request.msg_id);
        }
    }
    
    /// 处理后端RpcForwardMessageBNotify（消息转发到业务处理器）
    /// 
    /// 注意：本方法在主线程调用
    pub fn handle_rpc_forward_message_notify(&mut self, back_session: &mut BackSession, notify: &RpcForwardMessageBNotify) {
        debug!("Handling RpcForwardMessageBNotify: msg_id={}, front_session_id={}, message_size={}", 
               notify.msg_id, notify.front_session_id, notify.message.len());
        
        // 根据msg_id反序列化内部消息
        let mut buffer = DynamicBuffer::new(notify.message.len(), 1024);
        buffer.write_slice(&notify.message);
        
        if let Some(inner_message) = MessageFactory::decode_message(notify.msg_id as u16, &mut buffer, notify.message.len()) {
            debug!("Successfully decoded inner message with msg_id={}", notify.msg_id);
            
            // 通过RPC消息分发器分发内部消息给业务处理器
            unsafe {
                if !self.rpc_message_dispatcher.is_null() {
                    let rpc_dispatcher = &mut *self.rpc_message_dispatcher;
                    if rpc_dispatcher.dispatch_notify_message(notify.msg_id as u16, back_session, notify.front_session_id, inner_message.as_ref()) {
                        debug!("Successfully dispatched RPC notification with msg_id={}", notify.msg_id);
                    } else {
                        warn!("No handler found for RPC notification with msg_id={}", notify.msg_id);
                    }
                } else {
                    error!("RpcMessageDispatcher is null");
                }
            }
        } else {
            error!("Failed to decode inner message with msg_id={}", notify.msg_id);
        }
    }
    
    /// 处理RpcForwardMessageBResponse
    /// 
    /// 注意：本方法在主线程调用
    pub fn handle_rpc_forward_message_response(&mut self, back_session: &mut BackSession, response: &RpcForwardMessageBResponse) {
        let back_session_id = back_session.get_session_id();
        debug!("Handling RpcForwardMessageBResponse from back session {}: msg_unique_id={}, front_session_id={}, msg_id={}, message_size={}", 
               back_session_id, response.msg_unique_id, response.front_session_id, response.msg_id, response.message.len());
        
        // 创建RpcMessageFResponse
        let front_response = RpcMessageFResponse {
            msg_unique_id: response.msg_unique_id,
            msg_id: response.msg_id,
            message: response.message.clone(),
        };
        
        // 获取前端会话并发送响应
        unsafe {
            if !self.front_session_manager.is_null() {
                let front_session_manager = &mut *self.front_session_manager;
                
                // 根据front_session_id获取前端会话
                if let Some(front_session) = front_session_manager.get_session_mut(response.front_session_id) {
                    if front_session.send_message(front_response) {
                        debug!("Successfully sent RpcMessageFResponse to front session {}", response.front_session_id);
                    } else {
                        error!("Failed to send RpcMessageFResponse to front session {}", response.front_session_id);
                    }
                } else {
                    error!("Front session {} not found", response.front_session_id);
                }
            } else {
                error!("FrontSessionManager is null");
            }
        }
    }
    
    /// 清理管理器
    /// 
    /// 注意：本方法在主线程调用
    pub fn dispose(&mut self) {
        // 清空指针（不负责dispose RpcMessageDispatcher，由Server管理）
        self.rpc_manager = std::ptr::null_mut();
        self.front_session_manager = std::ptr::null_mut();
        self.rpc_message_dispatcher = std::ptr::null_mut();
        
        debug!("ForwardManager disposed");
    }
}