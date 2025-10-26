use crate::framework::session::{FrontSessionMessageDispatcher, FrontSession, SessionTrait};
use crate::proto::messages::protobuf::message::session::{LoginFRequest, LoginFResponse};
use crate::proto::messages::protobuf::message::protobuf_message_id::MSG_ID_LOGIN_F_REQUEST;
use tracing::{info, debug, error};
use std::any::Any;

/// 登录消息处理器
pub struct LoginMessageHandler {
    // 可以添加需要的字段，比如数据库连接等
}

impl LoginMessageHandler {
    /// 创建新的登录消息处理器
    pub fn new() -> Self {
        Self {
            // 初始化字段
        }
    }

    /// 初始化处理器，注册前端消息处理函数
    pub fn init(&mut self, dispatcher: &mut FrontSessionMessageDispatcher) {
        // 注册LoginFRequest的处理器
        dispatcher.register_handler(
            MSG_ID_LOGIN_F_REQUEST,
            Box::new(move |session, message| {
                Self::handle_login_request(session, message);
            }),
        );
    }

    /// 处理登录请求
    pub fn handle_login_request(
        session: &mut FrontSession,
        message: &dyn Any
    ) {
        debug!("Handling LoginFRequest from session {}", session.get_session_id());

        // 尝试将消息转换为LoginFRequest
        if let Some(request) = message.downcast_ref::<LoginFRequest>() {
            info!("Received LoginFRequest with token: {} from session {}", 
                  request.token, session.get_session_id());

            // TODO: 这里应该验证token，查询数据库获取玩家信息
            // 现在先返回模拟数据
            
            // 生成模拟的玩家ID和名称
            let player_id = 10000 + session.get_session_id(); // 简单的ID生成
            let player_name = format!("Player_{}", player_id);

            // 创建响应消息
            let response = LoginFResponse {
                player_id,
                name: player_name.clone(),
            };

            // 发送响应
            if session.send_message(response) {
                info!("Sent LoginFResponse to session {} - player_id: {}, name: {}", 
                      session.get_session_id(), player_id, player_name);
                
                // 设置session的用户ID
                session.set_user_id(player_id);
                session.set_authenticated(true);
            } else {
                error!("Failed to send LoginFResponse to session {}", session.get_session_id());
            }
        } else {
            error!("Failed to cast message to LoginFRequest");
        }
    }

    /// 清理处理器
    pub fn dispose(&mut self) {
        debug!("LoginMessageHandler disposed");
    }
}