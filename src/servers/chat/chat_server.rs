use crate::framework::server::{ServerTrait, Server};
use crate::framework::config::config::Config;

use super::test_message_handler::TestMessageHandler;

pub struct ChatServer {
    pub base_server: Server,
    test_handler: TestMessageHandler,
}

impl ChatServer {
    pub fn new() -> Self {
        Self {
            base_server: Server::new(),
            test_handler: TestMessageHandler::new(),
        }
    }

    pub fn ServerName() -> String {
        "chat".to_string()
    }
}

impl ServerTrait for ChatServer {
    fn init(&mut self, server_id: u32, config: &Config) -> bool {
        if self.base_server.init(server_id, config) == false {
            return false;
        }
        
        // 初始化测试消息处理器
        let rpc_dispatcher_ptr = self.base_server.get_rpc_message_dispatcher_mut() as *mut _;
        let rpc_manager_ptr = self.base_server.get_rpc_manager_mut() as *mut _;
        unsafe {
            self.test_handler.init(&mut *rpc_dispatcher_ptr, &mut *rpc_manager_ptr);
        }
        
        true
    }
    
    fn lateInit(&mut self) -> bool {
        self.base_server.lateInit()
    }
    
    fn dispose(&mut self) {
        self.test_handler.dispose();
        self.base_server.dispose();
    }
    
    async fn run(&mut self) {
        self.base_server.run().await
    }
}
