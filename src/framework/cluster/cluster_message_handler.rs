use crate::framework::session::{BackSessionMessageDispatcher, BackSession, SessionTrait};
use crate::proto::messages::protobuf::message::cluster::{NodeRegisterBRequest, NodeRegisterBResponse, NodeConnectBRequest, NodeConnectBResponse, NodeRegisterBNotify, ServerConfig};
use crate::proto::messages::protobuf::message::protobuf_message_id::{MSG_ID_NODE_REGISTER_B_REQUEST, MSG_ID_NODE_REGISTER_B_RESPONSE, MSG_ID_NODE_CONNECT_B_REQUEST, MSG_ID_NODE_CONNECT_B_RESPONSE, MSG_ID_NODE_REGISTER_B_NOTIFY};
use crate::framework::cluster::{ClusterManager, ServerManager};
use crate::framework::author::server_token::server_token_authentication;
use super::server_info::ServerInfo;
use tracing::{debug, info, error};
use std::any::Any;

/// 集群消息处理器
/// 由Server持有，负责处理集群相关的消息
pub struct ClusterMessageHandler {
    /// ServerManager 指针
    server_manager: *mut ServerManager,
    /// ClusterManager 指针
    cluster_manager: *mut ClusterManager,
    /// BackSessionManager 指针
    back_session_manager: *mut crate::framework::session::BackSessionManager,
    /// 当前服务器配置指针
    current_server_config: *const crate::framework::config::config::ServerConfig,
    /// Master Server ID
    master_server_id: u32,
    /// 当前服务器类型
    server_type: String,
    /// Author key for token validation
    author_key: String,
}

impl ClusterMessageHandler {
    /// 创建新的集群消息处理器
    pub fn new() -> Self {
        Self {
            server_manager: std::ptr::null_mut(),
            cluster_manager: std::ptr::null_mut(),
            back_session_manager: std::ptr::null_mut(),
            current_server_config: std::ptr::null(),
            master_server_id: 0,
            server_type: String::new(),
            author_key: String::new(),
        }
    }
    
    /// 初始化集群消息处理器
    pub fn init(&mut self, 
        dispatcher: &mut BackSessionMessageDispatcher, 
        server_manager: *mut ServerManager, 
        cluster_manager: *mut ClusterManager, 
        back_session_manager: *mut crate::framework::session::BackSessionManager, 
        server_config: &crate::framework::config::config::ServerConfig, 
        server_type: &str,
        master_server_id: Option<u32>,
        author_key: String) {
        // 保存配置和类型
        self.server_manager = server_manager;
        self.cluster_manager = cluster_manager;
        self.back_session_manager = back_session_manager;
        self.current_server_config = server_config as *const crate::framework::config::config::ServerConfig;
        self.master_server_id = master_server_id.unwrap_or(0);
        self.server_type = server_type.to_string();
        self.author_key = author_key;
        
        if server_type == "master" {
            // Master服务器：只注册NodeRegisterBRequest处理器
            
            // 注册 NodeRegisterBRequest 处理器（master 模式）
            let cluster_handler_addr = self as *mut ClusterMessageHandler as usize;
            let handler_request = Box::new(move |session: &mut BackSession, message: &dyn Any| {
                let cluster_handler_ptr = cluster_handler_addr as *mut ClusterMessageHandler;
                Self::handle_node_register_request(session, message, cluster_handler_ptr);
            });
            dispatcher.register_handler(MSG_ID_NODE_REGISTER_B_REQUEST, handler_request);
        } else {
            // 非master服务器：注册所有其他处理器
            
            // 注册 NodeRegisterBResponse 处理器
            let cluster_handler_addr2 = self as *mut ClusterMessageHandler as usize;
            let handler_response = Box::new(move |session: &mut BackSession, message: &dyn Any| {
                let cluster_handler_ptr = cluster_handler_addr2 as *mut ClusterMessageHandler;
                Self::handle_node_register_response(session, message, cluster_handler_ptr);
            });
            dispatcher.register_handler(MSG_ID_NODE_REGISTER_B_RESPONSE, handler_response);
            
            // 注册 NodeConnectBRequest 处理器
            let cluster_handler_addr3 = self as *mut ClusterMessageHandler as usize;
            let handler_connect_request = Box::new(move |session: &mut BackSession, message: &dyn Any| {
                let cluster_handler_ptr = cluster_handler_addr3 as *mut ClusterMessageHandler;
                Self::handle_node_connect_request(session, message, cluster_handler_ptr);
            });
            dispatcher.register_handler(MSG_ID_NODE_CONNECT_B_REQUEST, handler_connect_request);
            
            // 注册 NodeConnectBResponse 处理器
            let cluster_handler_addr4 = self as *mut ClusterMessageHandler as usize;
            let handler_connect_response = Box::new(move |session: &mut BackSession, message: &dyn Any| {
                let cluster_handler_ptr = cluster_handler_addr4 as *mut ClusterMessageHandler;
                Self::handle_node_connect_response(session, message, cluster_handler_ptr);
            });
            dispatcher.register_handler(MSG_ID_NODE_CONNECT_B_RESPONSE, handler_connect_response);
            
            // 注册 NodeRegisterBNotify 处理器
            let cluster_handler_addr5 = self as *mut ClusterMessageHandler as usize;
            let handler_register_notify = Box::new(move |session: &mut BackSession, message: &dyn Any| {
                let cluster_handler_ptr = cluster_handler_addr5 as *mut ClusterMessageHandler;
                Self::handle_node_register_notify(session, message, cluster_handler_ptr);
            });
            dispatcher.register_handler(MSG_ID_NODE_REGISTER_B_NOTIFY, handler_register_notify);
        }
        
    }

    /// 清理集群消息处理器
    pub fn dispose(&mut self, dispatcher: &mut BackSessionMessageDispatcher) {
        // 注销消息处理器
        dispatcher.unregister_handler(MSG_ID_NODE_REGISTER_B_REQUEST);
        dispatcher.unregister_handler(MSG_ID_NODE_REGISTER_B_RESPONSE);
        dispatcher.unregister_handler(MSG_ID_NODE_CONNECT_B_REQUEST);
        dispatcher.unregister_handler(MSG_ID_NODE_CONNECT_B_RESPONSE);
        dispatcher.unregister_handler(MSG_ID_NODE_REGISTER_B_NOTIFY);
        
        // 清空指针
        self.server_manager = std::ptr::null_mut();
        self.cluster_manager = std::ptr::null_mut();
        self.back_session_manager = std::ptr::null_mut();
        self.current_server_config = std::ptr::null();
        self.master_server_id = 0;
        
        debug!("ClusterMessageHandler disposed");
    }

    /// 处理节点注册请求（静态函数，用于注册到消息分发器）
    pub fn handle_node_register_request(
        session: &mut BackSession, 
        message: &dyn Any,
        cluster_handler_ptr: *mut ClusterMessageHandler
    ) {
        debug!("Handling NodeRegisterBRequest from session {}", session.get_session_id());

        // 尝试将消息转换为 NodeRegisterBRequest
        if let Some(request) = message.downcast_ref::<NodeRegisterBRequest>() {
            // 如果 server_config 是 None，返回错误
            let server_config = match request.server_config.as_ref() {
                Some(config) => config,
                None => {
                    error!("NodeRegisterBRequest missing server_config");
                    return;
                }
            };
            info!("Received NodeRegisterBRequest from server_id: {}, server_type: {}", 
                    server_config.server_id, server_config.server_type);
            
            debug!("Processing node registration for server_id: {}", server_config.server_id);
            
            // 检查 ClusterMessageHandler 指针是否有效
            if cluster_handler_ptr.is_null() {
                error!("ClusterMessageHandler pointer is null");
                return;
            }
            
            // 安全访问 ClusterMessageHandler
            let cluster_handler = unsafe { &mut *cluster_handler_ptr };
            
            // 验证token
            if !server_token_authentication(&request.client_token, &cluster_handler.author_key) {
                error!("Invalid token from server_id: {}, closing session", server_config.server_id);
                
                // 检查 BackSessionManager 指针是否有效
                if !cluster_handler.back_session_manager.is_null() {
                    let back_session_mgr = unsafe { &mut *cluster_handler.back_session_manager };
                    let session_id = session.get_session_id();
                    back_session_mgr.remove_bad_token_session(session_id, server_config.server_id);
                }
                return;
            }
            
            info!("Token validation successful for server_id: {}", server_config.server_id);
            
            // 验证通过，将session设置为可信任（已认证）
            if !cluster_handler.back_session_manager.is_null() {
                let back_session_mgr = unsafe { &mut *cluster_handler.back_session_manager };
                let session_id = session.get_session_id();
                if back_session_mgr.authorize_session(session_id, server_config.server_id, server_config.server_type.clone()) {
                    info!("Session {} moved to authorized list after successful token validation in NodeRegisterBRequest, server_id: {}, server_type: {}", 
                          session_id, server_config.server_id, server_config.server_type);
                } else {
                    error!("Failed to authorize session {} after token validation in NodeRegisterBRequest", session_id);
                }
            }
            
            // 检查 ServerManager 指针是否有效
            if cluster_handler.server_manager.is_null() {
                error!("ServerManager pointer is null");
                return;
            }
            
            // 安全访问 ServerManager
            let server_manager = unsafe { &mut *cluster_handler.server_manager };
            let master_server_id = cluster_handler.master_server_id;
            
            // 1. 将 server_config 添加到 server_manager
            let new_server = ServerInfo::new(
                server_config.server_id,
                server_config.server_type.clone(),
                server_config.back_host.clone(),
                server_config.back_tcp_port,
            );
            info!("Adding server to cluster: {}", new_server.get_info());
            server_manager.add_server(new_server);
            
            // 2. 遍历 server_manager，获取 server_id 小于当前请求服务器 ID 的服务器
            let mut server_list = Vec::new();
            let request_server_id = server_config.server_id;
            
            for server in server_manager.get_all_servers() {
                let server_id = server.get_server_id();
                // 只返回 server_id 小于请求服务器的，且排除 master server 自己
                if server_id < request_server_id && server_id != master_server_id {
                    server_list.push(ServerConfig {
                        server_id: server.get_server_id(),
                        server_type: server.get_server_type().clone(),
                        back_host: server.get_back_host().clone(),
                        back_tcp_port: server.get_back_tcp_port(),
                    });
                }
            }
            
            debug!("Found {} servers with ID < {} (excluding master server ID {})", 
                   server_list.len(), request_server_id, master_server_id);
            
            // 3. 创建响应消息
            let response = NodeRegisterBResponse {
                msg_unique_id: request.msg_unique_id,
                server_list,
            };
            
            // 4. 发送响应
            if session.send_message(response.clone()) {
                info!("Sent NodeRegisterBResponse to server_id: {} successfully with {} servers", 
                      server_config.server_id, response.server_list.len());
            } else {
                error!("Failed to send NodeRegisterBResponse to server_id: {}", 
                       server_config.server_id);
            }
            
            // 5. 给 server_id > request_server_id 的服务器发送 NodeRegisterBNotify
            let notify_message = NodeRegisterBNotify {
                server_config: Some(ServerConfig {
                    server_id: server_config.server_id,
                    server_type: server_config.server_type.clone(),
                    back_host: server_config.back_host.clone(),
                    back_tcp_port: server_config.back_tcp_port,
                }),
            };
            
            // 获取 BackSessionManager 并查找相应的会话
            if cluster_handler.back_session_manager.is_null() {
                error!("BackSessionManager pointer is null");
                return;
            }
            
            let back_session_mgr = unsafe { &mut *cluster_handler.back_session_manager };
            
            // 遍历所有服务器，找到 server_id > request_server_id 的服务器并发送通知
            for server in server_manager.get_all_servers() {
                let server_id = server.get_server_id();
                if server_id > request_server_id && server_id != master_server_id {
                    // 查找对应的 BackSession
                    if let Some(target_session) = back_session_mgr.find_session_by_server_id(server_id) {
                        let target_session_id = target_session.get_session_id();
                        if let Some(target_session_mut) = back_session_mgr.get_session_mut(target_session_id) {
                            if target_session_mut.send_message(notify_message.clone()) {
                                info!("Sent NodeRegisterBNotify to server_id: {} about new server_id: {}", 
                                      server_id, server_config.server_id);
                            } else {
                                error!("Failed to send NodeRegisterBNotify to server_id: {}", server_id);
                            }
                        }
                    } else {
                        debug!("No BackSession found for server_id: {} (may not be connected yet)", server_id);
                    }
                }
            }
        } else {
            error!("Failed to downcast message to NodeRegisterBRequest");
        }
    }

    /// 处理节点注册响应（静态函数，用于注册到消息分发器）
    pub fn handle_node_register_response(
        session: &mut BackSession, 
        message: &dyn Any,
        cluster_handler_ptr: *mut ClusterMessageHandler
    ) {
        debug!("Handling NodeRegisterBResponse from session {}", session.get_session_id());

        // 尝试将消息转换为 NodeRegisterBResponse
        if let Some(response) = message.downcast_ref::<NodeRegisterBResponse>() {
            info!("Received NodeRegisterBResponse with {} servers", 
                    response.server_list.len());
            
            debug!("Processing node register response with msg_unique_id: {}", response.msg_unique_id);
            
            // 检查 ClusterMessageHandler 指针是否有效
            if cluster_handler_ptr.is_null() {
                error!("ClusterMessageHandler pointer is null");
                return;
            }
            
            // 安全访问 ClusterMessageHandler
            let cluster_handler = unsafe { &mut *cluster_handler_ptr };
            
            // 检查指针是否有效
            if cluster_handler.cluster_manager.is_null() {
                error!("ClusterManager pointer is null");
                return;
            }
            
            if cluster_handler.back_session_manager.is_null() {
                error!("BackSessionManager pointer is null");
                return;
            }
            
            // 安全访问管理器
            let _cluster_manager = unsafe { &mut *cluster_handler.cluster_manager };
            let back_session_manager = unsafe { &mut *cluster_handler.back_session_manager };
            
            // 处理服务器列表，创建客户端连接
            for server_config in &response.server_list {
                info!("Received server info - ID: {}, Type: {}, Host: {}:{}", 
                      server_config.server_id, 
                      server_config.server_type,
                      server_config.back_host,
                      server_config.back_tcp_port);
                
                // 创建与其他服务器的客户端连接
                info!("ClusterMessageHandler: Connecting to CLUSTER NODE (from register response) - server_id={}, back_host={}, back_tcp_port={}", 
                      server_config.server_id, 
                      server_config.back_host, 
                      server_config.back_tcp_port);
                
                back_session_manager.create_client_session(
                    server_config.server_id,
                    &server_config.back_host,
                    server_config.back_tcp_port as u16
                );
                
                debug!("Client session created for server {}", server_config.server_id);
            }
            
            info!("Successfully processed NodeRegisterBResponse with {} servers and created {} client connections", 
                  response.server_list.len(), response.server_list.len());
        } else {
            error!("Failed to downcast message to NodeRegisterBResponse");
        }
    }

    /// 处理节点连接请求（静态函数，用于注册到消息分发器）
    pub fn handle_node_connect_request(
        session: &mut BackSession, 
        message: &dyn Any,
        cluster_handler_ptr: *mut ClusterMessageHandler
    ) {
        debug!("Handling NodeConnectBRequest from session {}", session.get_session_id());

        // 尝试将消息转换为 NodeConnectBRequest
        if let Some(request) = message.downcast_ref::<NodeConnectBRequest>() {
            // 如果 server_config 是 None，返回错误
            let server_config = match request.server_config.as_ref() {
                Some(config) => config,
                None => {
                    error!("NodeConnectBRequest missing server_config");
                    return;
                }
            };
            info!("Received NodeConnectBRequest from server_id: {}, server_type: {}", 
                    server_config.server_id, server_config.server_type);
            
            debug!("Processing node connect request with msg_unique_id: {}", request.msg_unique_id);
            
            // 检查 ClusterMessageHandler 指针是否有效
            if cluster_handler_ptr.is_null() {
                error!("ClusterMessageHandler pointer is null");
                return;
            }
            
            // 安全访问 ClusterMessageHandler
            let cluster_handler = unsafe { &mut *cluster_handler_ptr };
            
            // 验证token
            if !server_token_authentication(&request.client_token, &cluster_handler.author_key) {
                error!("Invalid token from server_id: {} in NodeConnectBRequest, closing session", server_config.server_id);
                
                // 检查 BackSessionManager 指针是否有效
                if !cluster_handler.back_session_manager.is_null() {
                    let back_session_mgr = unsafe { &mut *cluster_handler.back_session_manager };
                    let session_id = session.get_session_id();
                    back_session_mgr.remove_bad_token_session(session_id, server_config.server_id);
                }
                return;
            }
            
            info!("Token validation successful for server_id: {} in NodeConnectBRequest", server_config.server_id);
            
            // 验证通过，将session设置为可信任（已认证）
            if !cluster_handler.back_session_manager.is_null() {
                let back_session_mgr = unsafe { &mut *cluster_handler.back_session_manager };
                let session_id = session.get_session_id();
                if back_session_mgr.authorize_session(session_id, server_config.server_id, server_config.server_type.clone()) {
                    info!("Session {} moved to authorized list after successful token validation in NodeConnectBRequest, server_id: {}, server_type: {}", 
                          session_id, server_config.server_id, server_config.server_type);
                } else {
                    error!("Failed to authorize session {} after token validation in NodeConnectBRequest", session_id);
                }
            }
            
            // 检查 ServerManager 和当前服务器配置指针是否有效
            if cluster_handler.server_manager.is_null() {
                error!("ServerManager pointer is null");
                return;
            }
            
            if cluster_handler.current_server_config.is_null() {
                error!("Current server config pointer is null");
                return;
            }
            
            // 安全访问 ServerManager 和当前服务器配置
            let server_manager = unsafe { &mut *cluster_handler.server_manager };
            let current_config = unsafe { &*cluster_handler.current_server_config };
            
            // 将请求中的服务器配置添加到 server_manager
            let new_server = ServerInfo::new(
                server_config.server_id,
                server_config.server_type.clone(),
                server_config.back_host.clone(),
                server_config.back_tcp_port,
            );
            info!("Adding requesting server to cluster: {}", new_server.get_info());
            server_manager.add_server(new_server);
            
            // 创建响应消息（使用当前服务器的真实配置信息）
            let response = NodeConnectBResponse {
                msg_unique_id: request.msg_unique_id,
                server_config: Some(ServerConfig {
                    server_id: current_config.id,
                    server_type: unsafe { &*cluster_handler_ptr }.server_type.clone(),
                    back_host: current_config.back_host.clone(),
                    back_tcp_port: current_config.back_tcp_port as u32,
                }),
            };
            
            // 发送响应
            if session.send_message(response.clone()) {
                info!("Sent NodeConnectBResponse to server_id: {} successfully", 
                      server_config.server_id);
                debug!("Response details: msgid={}, response_server_id={}", 
                       response.msg_unique_id, response.server_config.as_ref().unwrap().server_id);
            } else {
                error!("Failed to send NodeConnectBResponse to server_id: {}", 
                       server_config.server_id);
            }
        } else {
            error!("Failed to downcast message to NodeConnectBRequest");
        }
    }

    /// 处理节点连接响应（静态函数，用于注册到消息分发器）
    pub fn handle_node_connect_response(
        session: &mut BackSession, 
        message: &dyn Any,
        cluster_handler_ptr: *mut ClusterMessageHandler
    ) {
        debug!("Handling NodeConnectBResponse from session {}", session.get_session_id());

        // 检查session是否已认证，未认证的session可能是恶意连接
        if !session.is_authenticated() {
            error!("Security: Received NodeConnectBResponse from unauthenticated session {}, removing session", session.get_session_id());
            // 检查 ClusterMessageHandler 指针是否有效
            if !cluster_handler_ptr.is_null() {
                let cluster_handler = unsafe { &mut *cluster_handler_ptr };
                if !cluster_handler.back_session_manager.is_null() {
                    let back_session_mgr = unsafe { &mut *cluster_handler.back_session_manager };
                    let session_id = session.get_session_id();
                    back_session_mgr.remove_bad_token_session(session_id, 0);
                }
            }
            return;
        }

        // 尝试将消息转换为 NodeConnectBResponse
        if let Some(response) = message.downcast_ref::<NodeConnectBResponse>() {
            // 如果 server_config 是 None，返回错误
            let server_config = match response.server_config.as_ref() {
                Some(config) => config,
                None => {
                    error!("NodeConnectBResponse missing server_config");
                    return;
                }
            };
            info!("Received NodeConnectBResponse from server_id: {}, server_type: {}", 
                    server_config.server_id, server_config.server_type);
            
            debug!("Processing node connect response with msg_unique_id: {}", response.msg_unique_id);
            
            // 检查 ClusterMessageHandler 指针是否有效
            if cluster_handler_ptr.is_null() {
                error!("ClusterMessageHandler pointer is null");
                return;
            }
            
            // 安全访问 ClusterMessageHandler
            let cluster_handler = unsafe { &mut *cluster_handler_ptr };
            
            // 检查 ServerManager 指针是否有效
            if cluster_handler.server_manager.is_null() {
                error!("ServerManager pointer is null");
                return;
            }
            
            // 安全访问 ServerManager
            let server_manager = unsafe { &mut *cluster_handler.server_manager };
            
            // 将响应中的服务器配置添加到 server_manager
            let responding_server = ServerInfo::new(
                server_config.server_id,
                server_config.server_type.clone(),
                server_config.back_host.clone(),
                server_config.back_tcp_port,
            );
            info!("Adding responding server to cluster: {}", responding_server.get_info());
            server_manager.add_server(responding_server);
            
            // 处理连接确认
            info!("Node connection established with server_id: {}, server_type: {}", 
                  server_config.server_id, server_config.server_type);
            
            // 收到响应确认连接，调用authorize_session更新服务器信息
            if !cluster_handler.back_session_manager.is_null() {
                let back_session_mgr = unsafe { &mut *cluster_handler.back_session_manager };
                let session_id = session.get_session_id();
                if back_session_mgr.authorize_session(session_id, server_config.server_id, server_config.server_type.clone()) {
                    info!("Session {} authorized with server info after NodeConnectBResponse, server_id: {}, server_type: {}", 
                          session_id, server_config.server_id, server_config.server_type);
                } else {
                    // 如果authorize_session失败，可能session已经在authorized列表中，直接更新服务器信息
                    if let Some(session) = back_session_mgr.get_session_mut(session_id) {
                        session.set_server_id(server_config.server_id);
                        session.set_server_type(Some(server_config.server_type.clone()));
                        info!("Updated existing session {} server info after NodeConnectBResponse, server_id: {}, server_type: {}", 
                              session_id, server_config.server_id, server_config.server_type);
                    }
                }
            }
            
            debug!("Successfully processed NodeConnectBResponse from server {}", 
                   server_config.server_id);
        } else {
            error!("Failed to downcast message to NodeConnectBResponse");
        }
    }

    /// 处理节点注册通知（静态函数，用于注册到消息分发器）
    pub fn handle_node_register_notify(
        _session: &mut BackSession, 
        message: &dyn Any,
        cluster_handler_ptr: *mut ClusterMessageHandler
    ) {
        debug!("Handling NodeRegisterBNotify");

        // 尝试将消息转换为 NodeRegisterBNotify
        if let Some(notify) = message.downcast_ref::<NodeRegisterBNotify>() {
            // 如果 server_config 是 None，返回错误
            let server_config = match notify.server_config.as_ref() {
                Some(config) => config,
                None => {
                    error!("NodeRegisterBNotify missing server_config");
                    return;
                }
            };
            info!("Received NodeRegisterBNotify from server_id: {}, server_type: {}", 
                  server_config.server_id, server_config.server_type);
            
            // 检查 ClusterMessageHandler 指针是否有效
            if cluster_handler_ptr.is_null() {
                error!("ClusterMessageHandler pointer is null");
                return;
            }
            
            // 安全访问 ClusterMessageHandler
            let cluster_handler = unsafe { &mut *cluster_handler_ptr };
            
            // 检查 ServerManager 和 BackSessionManager 指针是否有效
            if cluster_handler.server_manager.is_null() {
                error!("ServerManager pointer is null");
                return;
            }
            
            if cluster_handler.back_session_manager.is_null() {
                error!("BackSessionManager pointer is null");
                return;
            }
            
            // 安全访问 ServerManager 和 BackSessionManager
            let server_manager = unsafe { &mut *cluster_handler.server_manager };
            let back_session_mgr = unsafe { &mut *cluster_handler.back_session_manager };
            
            // 1. 将通知中的服务器配置添加到 server_manager
            let new_server = ServerInfo::new(
                server_config.server_id,
                server_config.server_type.clone(),
                server_config.back_host.clone(),
                server_config.back_tcp_port,
            );
            info!("Adding notified server to cluster: {}", new_server.get_info());
            server_manager.add_server(new_server);
            
            // 2. 调用 back_session_manager.create_client_session 去连接新服务器
            info!("ClusterMessageHandler: Connecting to CLUSTER NODE (from register notify) - server_id={}, back_host={}, back_tcp_port={}", 
                  server_config.server_id, 
                  server_config.back_host, 
                  server_config.back_tcp_port);
            
            back_session_mgr.create_client_session(
                server_config.server_id,
                &server_config.back_host,
                server_config.back_tcp_port as u16
            );
            
            info!("Created client session to connect to notified server_id: {} at {}:{}", 
                  server_config.server_id, 
                  server_config.back_host, 
                  server_config.back_tcp_port);
            
        } else {
            error!("Failed to downcast message to NodeRegisterBNotify");
        }
    }
}