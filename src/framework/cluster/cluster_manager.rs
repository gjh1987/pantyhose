use crate::framework::network::{
    NetworkEngine, 
    NetworkEngineEventManager, NetworkEventData, NetworkEventType, NetworkEventHandler,
    network_event_queue::ServerType
};
use crate::framework::config::config::ServerConfig;
use crate::framework::session::SessionTrait;
use crate::framework::author::server_token::generate_token;
use tracing::{info, error, debug};

pub struct ClusterManager {
    network_engine: Option<*mut NetworkEngine>,
    back_session_manager: Option<*mut crate::framework::session::BackSessionManager>,
    master_config: Option<ServerConfig>,
    server_group_name: String,
    server_config: Option<ServerConfig>,  // 服务器配置
    author_key: String,  // 作者密钥
    client_token: String,  // 客户端令牌
}

// 安全性：ClusterManager只在单线程环境中使用
unsafe impl Send for ClusterManager {}

impl ClusterManager {
    // ========== new methods ==========
    pub fn new() -> Self {
        Self {
            network_engine: None,
            back_session_manager: None,
            master_config: None,
            server_group_name: String::new(),
            server_config: None,
            author_key: String::new(),
            client_token: String::new(),
        }
    }

    // ========== init/dispose methods ==========
    // 初始化方法：设置配置并注册事件处理器
    pub fn init(
        &mut self, 
        server_group_name: String,
        network_engine: &mut NetworkEngine,
        back_session_manager: &mut crate::framework::session::BackSessionManager,
        event_manager: &mut NetworkEngineEventManager,
        master_config: Option<ServerConfig>,
        server_config: Option<ServerConfig>,
        author_key: String,
    ) -> bool {
        
        // Store pointers for later use
        self.network_engine = Some(network_engine as *mut NetworkEngine);
        self.back_session_manager = Some(back_session_manager as *mut crate::framework::session::BackSessionManager);
        self.master_config = master_config.clone();
        self.server_group_name = server_group_name.clone();
        self.server_config = server_config;
        self.author_key = author_key;
        
        // 生成client_token
        self.client_token = generate_token(&self.author_key);
        
        // 如果有master配置，说明当前不是master，需要连接到master
        if server_group_name != "master" {
            if let Some(ref config) = master_config {
                
                // 直接注册ClusterManager的指针作为处理器
                event_manager.add_handler(self as *mut dyn NetworkEventHandler);
            }
        }
        
        true
    }

    pub fn dispose(&mut self) {
        info!("Disposing ClusterManager");
        
        if let Some(_network_engine_ptr) = self.network_engine {
            // Clear all pointers
            self.network_engine = None;
            self.back_session_manager = None;
            self.master_config = None;
            info!("ClusterManager disposed and unregistered from NetworkEngine");
        }
    }

    // ========== other methods ==========
    // 连接到主服务器
    fn handle_client_connect_success(&mut self, event: &NetworkEventData) {
        let session_id = event.session_id;
        
        // 获取BackSessionManager并检查session
        if let Some(back_session_mgr) = self.back_session_manager {
            unsafe {
                let mgr = &mut *back_session_mgr;
                
                // 获取session
                if let Some(session) = mgr.get_unauthorized_session(session_id) {
                    // 检查是否是连接到master服务器的session
                    if let Some(ref master_config) = self.master_config {
                        if session.get_server_id() == master_config.id {
                            info!("Connected to master server, sending node registration");
                            
                            // 发送节点注册消息到master
                            self.send_node_register(session_id);
                        } else {
                            info!("Connected to other node server {}, sending node connect request", session.get_server_id());
                            
                            // 发送节点连接消息（其他节点不需要延迟）
                            self.send_node_connect(session_id);
                        }
                    }
                }
            }
        }
    }
    
    fn send_node_register(&mut self, session_id: u64) {
        use crate::proto::messages::protobuf::message::cluster::{NodeRegisterBRequest, ServerConfig as ClusterServerConfig};
        use crate::proto::messages::{MessageId, MessageIdSerialize};
        
        // 检查是否有server_config
        if let Some(ref config) = self.server_config {
            // 创建当前服务器的ClusterServerConfig（用于发送给master）
            let current_server_config = ClusterServerConfig {
                server_id: config.id,
                server_type: self.server_group_name.clone(),
                back_host: config.back_host.clone(),
                back_tcp_port: config.back_tcp_port as u32,
            };
            
            // 创建节点注册请求（msg_unique_id字段现在用作请求ID，不是消息类型ID）
            let register_request = NodeRegisterBRequest {
                msg_unique_id: chrono::Utc::now().timestamp_millis() as u32,  // 使用时间戳作为请求ID
                client_token: self.client_token.clone(),
                server_config: Some(current_server_config),
            };
            
            // 获取BackSessionManager并发送消息
            if let Some(back_session_mgr) = self.back_session_manager {
                unsafe {
                    let mgr = &mut *back_session_mgr;
                    if let Some(session) = mgr.get_unauthorized_session_mut(session_id) {
                        // 发送消息 - send_message会处理编码
                        if session.send_message(register_request) {
                            // 发送成功，将session设置为可信任（已认证）
                            // 这个session连接的是master，所以使用master的信息
                            if let Some(ref master_config) = self.master_config {
                                if mgr.authorize_session(session_id, master_config.id, "master".to_string()) {
                                    info!("Session {} moved to authorized list after successfully sending NodeRegisterBRequest to master, server_id: {}, server_type: master", 
                                          session_id, master_config.id);
                                } else {
                                    error!("Failed to authorize session {} after sending NodeRegisterBRequest", session_id);
                                }
                            } else {
                                error!("Master config not available for session authorization");
                            }
                        } else {
                            error!("Failed to send node register request");
                        }
                    }
                }
            }
        } else {
            error!("Cannot send node register request: server_config is None");
        }
    }
    
    fn send_node_connect(&mut self, session_id: u64) {
        use crate::proto::messages::protobuf::message::cluster::{NodeConnectBRequest, ServerConfig as ClusterServerConfig};
        
        // 检查是否有server_config
        if let Some(ref config) = self.server_config {
            // 创建当前服务器的ClusterServerConfig（用于发送给其他节点）
            let current_server_config = ClusterServerConfig {
                server_id: config.id,
                server_type: self.server_group_name.clone(),
                back_host: config.back_host.clone(),
                back_tcp_port: config.back_tcp_port as u32,
            };
            
            // 创建节点连接请求，包含当前服务器的配置信息和token
            let connect_request = NodeConnectBRequest {
                msg_unique_id: chrono::Utc::now().timestamp_millis() as u32,  // 使用时间戳作为请求ID
                client_token: self.client_token.clone(),
                server_config: Some(current_server_config),
            };
            
            // 获取BackSessionManager并发送消息
            if let Some(back_session_mgr) = self.back_session_manager {
                unsafe {
                    let mgr = &mut *back_session_mgr;
                    if let Some(session) = mgr.get_unauthorized_session_mut(session_id) {
                        // 发送消息 - send_message会处理编码
                        if session.send_message(connect_request) {
                            // 发送成功，先设置session为已认证状态
                            session.set_authenticated(true);
                            info!("Successfully sent NodeConnectBRequest from session {}, set to authenticated", session_id);
                        } else {
                            error!("Failed to send node connect request");
                        }
                    }
                }
            }
        } else {
            error!("Cannot send node connect request: server_config is None");
        }
    }
    
    fn connect_to_master(&mut self) {
        if let Some(ref config) = self.master_config {
            unsafe {
                if let Some(manager) = self.back_session_manager.and_then(|p| p.as_mut()) {
                    // 创建一个新的客户端会话连接到 master
                    manager.create_client_session(
                        config.id, // server_id
                        &config.back_host,
                        config.back_tcp_port,
                    );
                } else {
                    error!("BackSessionManager pointer is null");
                }
            }
        }
    }
}

impl NetworkEventHandler for ClusterManager {
    fn handle_event(&mut self, event: &mut NetworkEventData) {
        // 只处理 BackTcp 的 ServerOpen 事件
        if event.server_type != ServerType::BackTcp{
            return;
        }
        match event.event_type {
            NetworkEventType::ServerOpen => {
                if let Some(ref config) = self.master_config {
                        debug!("ClusterManager: Server opened, connecting to master at {}:{}", 
                                config.back_host, config.back_tcp_port);
                        // 直接调用connect_to_master
                        self.connect_to_master();
                    }
            }
            NetworkEventType::ClientConnectSuccess => {
                self.handle_client_connect_success(event);
            }
            _ => {}
        }
    }
}