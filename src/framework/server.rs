use crate::framework::config::config::{Config, ServerConfig};
use crate::framework::config::server_config_manager::ServerConfigManager;
use crate::framework::log::log::LogGuard;
use crate::framework::session::{BackSessionManager, BackSessionMessageDispatcher, FrontSessionManager, FrontSessionGroupManager, FrontSessionMessageDispatcher};
use crate::framework::network::{NetworkEngine, NetworkEngineEventManager};
use crate::framework::cluster::{ClusterManager, ClusterMessageHandler, ServerManager};
use crate::framework::rpc::{RpcManager, RouterManager, RpcMessageDispatcher};
use crate::framework::msg::{MsgProcessor, ProtobufMsgProcessor};
use crate::framework::timer::TimeManager;
use crate::framework::task::TaskManager;
use crate::framework::rpc::ForwardManager;
use crate::framework::db::db_manager::DBManager;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};
use tokio::sync::Notify;
use tokio::time::timeout;

pub trait ServerTrait {
    fn init(&mut self, server_id: u32, config: &Config) -> bool;
    fn lateInit(&mut self) -> bool;
    fn dispose(&mut self);
    async fn run(&mut self);
}

pub struct Server {
    server_id: u32,
    server_config: ServerConfig,
    group_name: String,
    config_manager: ServerConfigManager,
    _log_guard: Option<LogGuard>,
    back_session_manager: BackSessionManager,
    front_session_manager: FrontSessionManager,
    front_session_group_manager: FrontSessionGroupManager,
    network_engine: NetworkEngine,
    network_event_manager: NetworkEngineEventManager,
    cluster_manager: ClusterManager,
    server_manager: ServerManager,
    cluster_message_handler: ClusterMessageHandler,
    back_message_dispatcher: BackSessionMessageDispatcher,
    front_message_dispatcher: FrontSessionMessageDispatcher,
    rpc_manager: RpcManager,
    router_manager: RouterManager,
    rpc_message_dispatcher: RpcMessageDispatcher,
    msg_processor: Arc<dyn MsgProcessor>,
    time_manager: TimeManager,
    task_manager: TaskManager,
    forward_manager: ForwardManager,
    db_manager: DBManager,

    // notify
    is_running:bool,
    notify:Arc<Notify>,
}

impl ServerTrait for Server {
    fn init(&mut self, server_id: u32, config: &Config) -> bool {
        // Initialize config manager first
        {
            if self.config_manager.init_from_config(config) == false {
                error!("Failed to initialize config manager");
                return false;
            }
        }
        
        // Get server info by server_id
        let server_info = match self.config_manager.get_server_by_id(server_id) {
            Some(info) => info,
            None => {
                error!("Server with ID {} not found in configuration", server_id);
                return false;
            }
        };
        
        // Set server information
        self.server_id = server_id;
        self.server_config = server_info.server_config.clone();
        self.group_name = server_info.server_type.clone();
        
        // Initialize log manager
        let (success, log_guard) = crate::framework::log::log::LogManager::init_logger(
            &config.log, 
            self.group_name.clone(), 
            server_id, None
        );
        if success == false {
            return false;
        }
        self._log_guard = log_guard;
        
        // Set notify for network engine
        self.network_engine.set_notify(Arc::clone(&self.notify));
        
        // Initialize RPC manager with session managers
        if self.rpc_manager.init(&mut self.front_session_manager, &mut self.back_session_manager) == false {
            error!("Failed to initialize RPC manager");
            return false;
        }
        
        // Initialize router manager
        if self.router_manager.init() == false {
            error!("Failed to initialize router manager");
            return false;
        }
        
        // Initialize task manager with notify
        if self.task_manager.init(Arc::clone(&self.notify)) == false {
            error!("Failed to initialize task manager");
            return false;
        }
        
        // 消息处理器已经是无状态的，不需要初始化
        
        // Initialize session managers with required parameters
        self.back_session_manager.init(
            &mut self.network_event_manager,
            Arc::clone(&self.msg_processor),
            self.network_engine.get_event_queue(),
            Arc::clone(&self.notify)
        );
        self.front_session_manager.init(
            &mut self.network_event_manager,
            Arc::clone(&self.msg_processor),
            self.network_engine.get_event_queue(),
            Arc::clone(&self.notify)
        );
        
        // Initialize back message dispatcher
        if self.back_message_dispatcher.init(&mut self.network_event_manager, &mut self.back_session_manager) == false {
            error!("Failed to initialize back message dispatcher");
            return false;
        }
        
        // Initialize front message dispatcher
        if self.front_message_dispatcher.init(&mut self.network_event_manager, &mut self.front_session_manager) == false {
            error!("Failed to initialize front message dispatcher");
            return false;
        }
        
        // Initialize forward manager
        if self.forward_manager.init(
            &mut self.front_message_dispatcher,
            &mut self.back_message_dispatcher,
            &mut self.router_manager,
            &mut self.front_session_manager,
            &mut self.back_session_manager,
            &mut self.task_manager,
            &mut self.rpc_manager,
            &mut self.rpc_message_dispatcher,
        ) {
            error!("Failed to initialize forward manager");
            return false;
        }
        
        // Initialize server manager
        if self.server_manager.init() == false {
            error!("Failed to initialize server manager");
            return false;
        }
        
        if self.cluster_manager.init(
            self.group_name.clone(),
            &mut self.network_engine,
            &mut self.back_session_manager,
            &mut self.network_event_manager,
            self.config_manager.get_master_config().cloned(),
            Some(self.server_config.clone()),
            self.config_manager.get_author_key().to_string(),
        ) {
            error!("Failed to initialize cluster manager");
            return false;
        }
        
        // Initialize cluster message handler with server_manager, cluster_manager, back_session_manager and server_config
        let server_manager_ptr = &mut self.server_manager as *mut ServerManager;
        let cluster_manager_ptr = &mut self.cluster_manager as *mut ClusterManager;
        let back_session_manager_ptr = &mut self.back_session_manager as *mut crate::framework::session::BackSessionManager;
        let master_server_id = self.config_manager.get_master_config().map(|config| config.id);
        self.cluster_message_handler.init(
            &mut self.back_message_dispatcher, 
            server_manager_ptr, 
            cluster_manager_ptr, 
            back_session_manager_ptr, 
            &self.server_config, 
            &self.group_name, 
            master_server_id,
            self.config_manager.get_author_key().to_string());

        // Initialize DB manager if MongoDB configuration is present
        if let Some(mongodb_config) = &config.mongodb {
            if self.db_manager.init(mongodb_config) == false {
                error!("Failed to initialize DB manager");
                return false;
            }
        }

        true
    }

    fn lateInit(&mut self) -> bool {
        // Late initialization for other components if needed
        info!("Server late initialization completed");
        true
    }

    fn dispose(&mut self) {
        // Dispose in reverse order of initialization
        
        // Dispose cluster message handler (last initialized)
        self.cluster_message_handler.dispose(&mut self.back_message_dispatcher);
        
        // Dispose cluster manager
        self.cluster_manager.dispose();
        
        // Dispose server manager
        self.server_manager.dispose();
        
        // Dispose forward manager
        self.forward_manager.dispose();
        
        // Dispose session managers (reverse order)
        self.front_session_manager.dispose();
        self.back_session_manager.dispose();
        
        // Dispose message processor
        // 消息处理器是无状态的，不需要销毁
        
        // Dispose task manager
        self.task_manager.dispose();
        
        // Dispose router manager
        self.router_manager.dispose();
        
        // Dispose RPC manager
        self.rpc_manager.dispose();
        
        // Clear all timers (cleanup)
        self.time_manager.clear_all_timers();

        // Dispose DB manager
        self.db_manager.dispose();

        // TODO: 实现其他资源清理
    }

    async fn run(&mut self) {
        // Main server loop
        if self.server_config.back_tcp_port != 0 {
            self.network_engine.start_back_tcp(self.server_config.back_tcp_port);
        }
        if let Some(front_tcp_port) = self.server_config.front_tcp_port {
            self.network_engine.start_front_tcp(front_tcp_port);
        }
        if let Some(front_ws_port) = self.server_config.front_ws_port {
            self.network_engine.start_front_websocket(front_ws_port);
        }

        // 获取事件队列
        let event_queue = self.network_engine.get_event_queue();
        
        self.is_running = true;
        loop {
            // 先处理网络事件队列
            while event_queue.is_empty().await == false {
                if let Some(mut event) = event_queue.pop().await {
                    // 使用EventManager分发事件给注册的处理器
                    self.network_event_manager.dispatch(&mut event);
                }
            }
            
            // 处理完成的任务（直接调用，内部会检查是否为空）
            self.task_manager.process_finished_tasks();
            
            // 然后等待下一次循环
            let wait_time = self.time_manager.first_time_wait();
            timeout(Duration::from_millis(wait_time), self.notify.notified()).await;
            
            if self.is_running == false{
                break;
            }
        }
    }
}

impl Server {
    
    pub fn new() -> Self {
        Self {
            server_id: 0,
            server_config: ServerConfig{
                id:0,
                host:"".to_string(),
                front_host:"".to_string(),
                back_host:"".to_string(),
                back_tcp_port: 0,
                front_tcp_port: None,
                front_ws_port:None,
            },
            group_name: "".to_string(),
            config_manager: ServerConfigManager::new(),
            _log_guard: None,
            back_session_manager: BackSessionManager::new(),
            front_session_manager: FrontSessionManager::new(),
            front_session_group_manager: FrontSessionGroupManager::new(),
            network_engine: NetworkEngine::new(),
            network_event_manager: NetworkEngineEventManager::new(),
            cluster_manager: ClusterManager::new(),
            server_manager: ServerManager::new(),
            cluster_message_handler: ClusterMessageHandler::new(),
            back_message_dispatcher: BackSessionMessageDispatcher::new(),
            front_message_dispatcher: FrontSessionMessageDispatcher::new(),
            rpc_manager: RpcManager::new(),
            router_manager: RouterManager::new(),
            rpc_message_dispatcher: RpcMessageDispatcher::new(),
            msg_processor: Arc::new(ProtobufMsgProcessor::new()),
            time_manager: TimeManager::new(),
            task_manager: TaskManager::new(),
            forward_manager: ForwardManager::new(),
            db_manager: DBManager::new(),

            is_running:(false),
            notify:(Arc::new(Notify::new())),
        }
    }
    
    pub fn get_network_event_manager(&self) -> &NetworkEngineEventManager {
        &self.network_event_manager
    }
    
    pub fn get_front_message_dispatcher_mut(&mut self) -> &mut FrontSessionMessageDispatcher {
        &mut self.front_message_dispatcher
    }
    
    /// Get mutable network event manager
    pub fn get_network_event_manager_mut(&mut self) -> &mut NetworkEngineEventManager {
        &mut self.network_event_manager
    }

    /// Get server configuration
    pub fn get_server_config(&self) -> &ServerConfig {
        &self.server_config
    }

    /// Get server group name
    pub fn get_group_name(&self) -> &String {
        &self.group_name
    }

    /// Get server ID
    pub fn get_server_id(&self) -> u32 {
        self.server_id
    }

    /// Get config manager
    pub fn get_config_manager(&self) -> &ServerConfigManager {
        &self.config_manager
    }

    /// Get back session manager
    pub fn get_back_session_manager(&self) -> &BackSessionManager {
        &self.back_session_manager
    }

    /// Get mutable back session manager
    pub fn get_back_session_manager_mut(&mut self) -> &mut BackSessionManager {
        &mut self.back_session_manager
    }

    /// Get front session manager
    pub fn get_front_session_manager(&self) -> &FrontSessionManager {
        &self.front_session_manager
    }

    /// Get mutable front session manager
    pub fn get_front_session_manager_mut(&mut self) -> &mut FrontSessionManager {
        &mut self.front_session_manager
    }


    /// Get front session group manager
    pub fn get_front_session_group_manager(&self) -> &FrontSessionGroupManager {
        &self.front_session_group_manager
    }

    /// Get mutable front session group manager
    pub fn get_front_session_group_manager_mut(&mut self) -> &mut FrontSessionGroupManager {
        &mut self.front_session_group_manager
    }

    /// Get network engine
    pub fn get_network_engine(&self) -> &NetworkEngine {
        &self.network_engine
    }

    /// Get mutable network engine
    pub fn get_network_engine_mut(&mut self) -> &mut NetworkEngine {
        &mut self.network_engine
    }

    /// Get cluster manager
    pub fn get_cluster_manager(&self) -> &ClusterManager {
        &self.cluster_manager
    }

    /// Get mutable cluster manager
    pub fn get_cluster_manager_mut(&mut self) -> &mut ClusterManager {
        &mut self.cluster_manager
    }

    /// Get server manager
    pub fn get_server_manager(&self) -> &ServerManager {
        &self.server_manager
    }

    /// Get mutable server manager
    pub fn get_server_manager_mut(&mut self) -> &mut ServerManager {
        &mut self.server_manager
    }

    /// Get RPC manager
    pub fn get_rpc_manager(&self) -> &RpcManager {
        &self.rpc_manager
    }

    /// Get mutable RPC manager
    pub fn get_rpc_manager_mut(&mut self) -> &mut RpcManager {
        &mut self.rpc_manager
    }

    /// Get mutable back message dispatcher
    pub fn get_back_message_dispatcher_mut(&mut self) -> &mut BackSessionMessageDispatcher {
        &mut self.back_message_dispatcher
    }

    /// Get router manager
    pub fn get_router_manager(&self) -> &RouterManager {
        &self.router_manager
    }

    /// Get mutable router manager
    pub fn get_router_manager_mut(&mut self) -> &mut RouterManager {
        &mut self.router_manager
    }

    /// Get message processor
    pub fn get_msg_processor(&self) -> Arc<dyn MsgProcessor> {
        Arc::clone(&self.msg_processor)
    }

    /// Get time manager
    pub fn get_time_manager(&self) -> &TimeManager {
        &self.time_manager
    }

    /// Get mutable time manager
    pub fn get_time_manager_mut(&mut self) -> &mut TimeManager {
        &mut self.time_manager
    }

    /// Get task manager
    pub fn get_task_manager(&self) -> &TaskManager {
        &self.task_manager
    }

    /// Get mutable task manager
    pub fn get_task_manager_mut(&mut self) -> &mut TaskManager {
        &mut self.task_manager
    }

    /// Get RPC message dispatcher
    pub fn get_rpc_message_dispatcher(&self) -> &RpcMessageDispatcher {
        &self.rpc_message_dispatcher
    }

    /// Get mutable RPC message dispatcher
    pub fn get_rpc_message_dispatcher_mut(&mut self) -> &mut RpcMessageDispatcher {
        &mut self.rpc_message_dispatcher
    }

    pub fn stop(&mut self) {
        self.is_running = false;
    }

    pub fn wake(&self){
        if self.is_running == false {
            return;
        }
        self.notify.notify_one();
    }
}
