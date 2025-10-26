use crate::framework::server::{ServerTrait, Server};
use crate::framework::config::config::Config;
use super::login_message_handler::LoginMessageHandler;
use super::unlogin::unlogin_player_manager::UnloginPlayerManager;
use super::login::player_manager::PlayerManager;

pub struct SessionServer {
    pub base_server: Server,
    login_handler: LoginMessageHandler,
    unlogin_player_manager: UnloginPlayerManager,
    player_manager: PlayerManager,
}

impl SessionServer {
    pub fn new() -> Self {
        Self {
            base_server: Server::new(),
            login_handler: LoginMessageHandler::new(),
            unlogin_player_manager: UnloginPlayerManager::new(),
            player_manager: PlayerManager::new(),
        }
    }

    pub fn ServerName() -> String {
        "session".to_string()
    }
}

impl ServerTrait for SessionServer {
    fn init(&mut self, server_id: u32, config: &Config) -> bool {
        // 先初始化基础服务器
        if !self.base_server.init(server_id, config) {
            return false;
        }
        
        // 初始化管理器
        if !self.unlogin_player_manager.init() {
            return false;
        }
        
        if !self.player_manager.init() {
            return false;
        }
        
        true
    }
    
    fn lateInit(&mut self) -> bool {
        // 先调用基础服务器的lateInit
        if !self.base_server.lateInit() {
            return false;
        }
        
        // 注册登录消息处理器
        self.login_handler.init(self.base_server.get_front_message_dispatcher_mut());
        
        true
    }
    
    fn dispose(&mut self) {
        // 按照与初始化相反的顺序进行清理
        self.login_handler.dispose();
        self.player_manager.dispose();
        self.unlogin_player_manager.dispose();
        self.base_server.dispose();
    }
    
    async fn run(&mut self) {
        self.base_server.run().await
    }
}
