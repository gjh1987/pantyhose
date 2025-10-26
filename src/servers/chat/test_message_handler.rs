use crate::framework::session::{BackSession, SessionTrait};
use crate::framework::rpc::{RpcMessageDispatcher, RpcManager};
use crate::proto::messages::protobuf::message::chat::{ChatTestBRequest, ChatTestBResponse};
use crate::proto::messages::protobuf::message::protobuf_message_id::MSG_ID_CHAT_TEST_B_REQUEST;
use crate::proto::messages::MessageIdSerialize;
use tracing::{info, debug, error};
use std::any::Any;

/// 聊天测试消息处理器
pub struct TestMessageHandler {
    /// RPC管理器指针，用于发送响应消息
    rpc_manager: *mut RpcManager,
}



impl TestMessageHandler {
    /// 创建新的测试消息处理器
    pub fn new() -> Self {
        Self {
            rpc_manager: std::ptr::null_mut(),
        }
    }

    /// 初始化处理器，注册RPC消息处理函数
    pub fn init(&mut self, dispatcher: &mut RpcMessageDispatcher, rpc_manager: &mut RpcManager) {
        // 保存RPC管理器指针
        self.rpc_manager = rpc_manager as *mut RpcManager;
        
        // 注册ChatTestBRequest的处理器（这是一个请求消息）
        dispatcher.register_request_handler(
            MSG_ID_CHAT_TEST_B_REQUEST,
            Box::new(|session, msg_unique_id, front_session_id, msg_id, message| {
                Self::handle_test_request_static(session, msg_unique_id, front_session_id, msg_id, message);
            }),
        );
        
    }

    /// 处理测试请求（静态方法）
    pub fn handle_test_request_static(
        session: &mut BackSession,
        msg_unique_id: u32,
        front_session_id: u64,
        msg_id: u32,
        message: &dyn Any
    ) {
        debug!("Handling ChatTestBRequest from session {}, msg_unique_id={}, front_session_id={}, msg_id={}", 
               session.get_session_id(), msg_unique_id, front_session_id, msg_id);

        // 尝试将消息转换为ChatTestBRequest
        if let Some(request) = message.downcast_ref::<ChatTestBRequest>() {
            info!("Received ChatTestBRequest with content: {} from session {}", 
                  request.content, session.get_session_id());

            // 创建响应消息
            let response = ChatTestBResponse {
                content: format!("Echo from chat server: {}", request.content),
            };

            // 手动创建RPC转发响应消息并发送
            match response.serialize_to_buffer() {
                Ok(serialized_response) => {
                    use crate::proto::messages::protobuf::message::cluster::RpcForwardMessageBResponse;
                    use crate::proto::messages::protobuf::message::protobuf_message_id::MSG_ID_CHAT_TEST_B_RESPONSE;
                    
                    let rpc_response = RpcForwardMessageBResponse {
                        msg_unique_id,
                        front_session_id,
                        meta: std::collections::HashMap::new(),
                        msg_id: MSG_ID_CHAT_TEST_B_RESPONSE as u32,
                        message: serialized_response.to_vec(),
                    };
                    
                    if session.send_message(rpc_response) {
                        info!("Successfully sent ChatTestBResponse via RPC to front session {} with content: Echo from chat server: {}", 
                              front_session_id, request.content);
                    } else {
                        error!("Failed to send ChatTestBResponse via RPC to front session {}", front_session_id);
                    }
                }
                Err(e) => {
                    error!("Failed to serialize ChatTestBResponse: {:?}", e);
                }
            }
        } else {
            error!("Failed to cast message to ChatTestBRequest");
        }
    }

    /// 清理处理器
    pub fn dispose(&mut self) {
        self.rpc_manager = std::ptr::null_mut();
        debug!("TestMessageHandler disposed");
    }
}