use crate::framework::session::{BackSessionMessageDispatcher, BackSession, SessionTrait};
use crate::proto::messages::protobuf::message::chat::{ChatTestBRequest, ChatTestBResponse};
use crate::proto::messages::protobuf::message::protobuf_message_id::{MSG_ID_CHAT_TEST_B_REQUEST, MSG_ID_CHAT_TEST_B_RESPONSE};
use tracing::{info, debug, error};
use std::any::Any;

/// 测试消息处理器
pub struct TestMessageHandler {
    // 可以添加需要的字段
}

impl TestMessageHandler {
    /// 创建新的测试消息处理器
    pub fn new() -> Self {
        Self {
            // 初始化字段
        }
    }

    /// 初始化处理器，注册后端消息处理函数
    pub fn init(&mut self, dispatcher: &mut BackSessionMessageDispatcher) {
        // 注册ChatTestBResponse的处理器
        dispatcher.register_handler(
            MSG_ID_CHAT_TEST_B_RESPONSE,
            Box::new(move |session, message| {
                Self::handle_chat_test_response(session, message);
            }),
        );
        
    }
    /// 处理聊天测试响应
    pub fn handle_chat_test_response(
        session: &mut BackSession,
        message: &dyn Any
    ) {
        debug!("Handling ChatTestBResponse from session {}", session.get_session_id());

        // 尝试将消息转换为ChatTestBResponse
        if let Some(response) = message.downcast_ref::<ChatTestBResponse>() {
            info!("Received ChatTestBResponse with content: {} from session {}", 
                  response.content, session.get_session_id());

            // 这里可以添加对响应消息的处理逻辑
            // 例如：转发给前端用户、记录日志、更新状态等
            debug!("Processed ChatTestBResponse from session {} in test handler", 
                   session.get_session_id());
        } else {
            error!("Failed to cast message to ChatTestBResponse");
        }
    }

    /// 清理处理器
    pub fn dispose(&mut self) {
        debug!("TestMessageHandler disposed");
    }
}