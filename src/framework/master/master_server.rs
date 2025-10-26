use crate::framework::server::{Server, ServerTrait};
use crate::framework::config::config::Config;
use crate::framework::session::BackSessionMessageDispatcher;
use tracing::{debug, info, error};

pub struct MasterServer {
    pub base_server: Server,
    pub back_message_dispatcher: BackSessionMessageDispatcher,
}

impl MasterServer {
    pub fn new() -> Self {
        Self {
            base_server: Server::new(),
            back_message_dispatcher: BackSessionMessageDispatcher::new(),
        }
    }
    
    /// 获取服务器管理器的引用（通过 base_server）
    pub fn get_server_manager(&self) -> &crate::framework::cluster::ServerManager {
        self.base_server.get_server_manager()
    }
    
    /// 获取服务器管理器的可变引用（通过 base_server）
    pub fn get_server_manager_mut(&mut self) -> &mut crate::framework::cluster::ServerManager {
        self.base_server.get_server_manager_mut()
    }

    /// 获取 master server 的 server_id
    pub fn get_server_id(&self) -> u32 {
        self.base_server.get_server_id()
    }

    pub fn ServerName() -> String {
        "master".to_string()
    }
}

impl ServerTrait for MasterServer {
    fn init(&mut self, server_id: u32, config: &Config) -> bool {
        // 先初始化基础服务器
        if !self.base_server.init(server_id, config) {
            return false;
        }
        
        true
    }
    
    fn lateInit(&mut self) -> bool {
        self.base_server.lateInit()
    }
    
    fn dispose(&mut self) {
        // Dispose in reverse order of initialization
        
        // 清理 BackSessionMessageDispatcher
        self.back_message_dispatcher.clear_all_handlers();
        
        // ServerManager 和 ClusterMessageHandler 现在由 base_server 管理
        
        // 清理基础服务器 (first initialized)
        self.base_server.dispose();
        
        debug!("MasterServer disposed");
    }
    
    async fn run(&mut self) {
        self.base_server.run().await
    }
}