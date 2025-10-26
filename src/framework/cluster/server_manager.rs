use super::server_info::ServerInfo;
use std::collections::HashMap;
use tracing::{debug, info, warn, error};

/// 服务器管理器
pub struct ServerManager {
    /// 服务器列表 (server_id -> ServerInfo)
    servers: HashMap<u32, ServerInfo>,
    /// 按类型分组的服务器 (server_type -> Vec<server_id>)
    servers_by_type: HashMap<String, Vec<u32>>,
}

impl ServerManager {
    /// 创建新的服务器管理器
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
            servers_by_type: HashMap::new(),
        }
    }
    
    /// 初始化服务器管理器
    pub fn init(&mut self) -> bool {
        
        // 清空现有数据
        self.servers.clear();
        self.servers_by_type.clear();
        
        true
    }
    
    /// 清理服务器管理器
    pub fn dispose(&mut self) {
        debug!("ServerManager disposing...");
        
        let count = self.servers.len();
        self.servers.clear();
        self.servers_by_type.clear();
        
        if count > 0 {
            info!("ServerManager disposed, cleared {} servers", count);
        } else {
            debug!("ServerManager disposed");
        }
    }

    /// 添加服务器
    pub fn add_server(&mut self, server: ServerInfo) -> bool {
        let server_id = server.server_id;
        let server_type = server.server_type.clone();
        
        // 检查是否已存在
        if self.servers.contains_key(&server_id) {
            warn!("Server {} already exists, updating", server_id);
            self.remove_server(server_id);
        }
        
        // 添加到主列表
        self.servers.insert(server_id, server.clone());
        
        // 添加到类型分组
        self.servers_by_type
            .entry(server_type.clone())
            .or_insert_with(Vec::new)
            .push(server_id);
        
        info!("Added server: {}", server.get_info());
        true
    }

    /// 移除服务器
    pub fn remove_server(&mut self, server_id: u32) -> Option<ServerInfo> {
        if let Some(server) = self.servers.remove(&server_id) {
            // 从类型分组中移除
            if let Some(servers) = self.servers_by_type.get_mut(&server.server_type) {
                servers.retain(|&id| id != server_id);
                if servers.is_empty() {
                    self.servers_by_type.remove(&server.server_type);
                }
            }
            
            info!("Removed server: {}", server.get_info());
            Some(server)
        } else {
            None
        }
    }

    /// 获取服务器
    pub fn get_server(&self, server_id: u32) -> Option<&ServerInfo> {
        self.servers.get(&server_id)
    }

    /// 获取可变服务器引用
    pub fn get_server_mut(&mut self, server_id: u32) -> Option<&mut ServerInfo> {
        self.servers.get_mut(&server_id)
    }

    /// 获取所有服务器
    pub fn get_all_servers(&self) -> Vec<&ServerInfo> {
        self.servers.values().collect()
    }

    /// 获取指定类型的所有服务器
    pub fn get_servers_by_type(&self, server_type: &str) -> Vec<&ServerInfo> {
        if let Some(server_ids) = self.servers_by_type.get(server_type) {
            server_ids
                .iter()
                .filter_map(|id| self.servers.get(id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 检查服务器是否存在
    pub fn has_server(&self, server_id: u32) -> bool {
        self.servers.contains_key(&server_id)
    }

    /// 获取服务器数量
    pub fn server_count(&self) -> usize {
        self.servers.len()
    }

    /// 获取按类型分组的服务器数量
    pub fn server_count_by_type(&self, server_type: &str) -> usize {
        self.servers_by_type
            .get(server_type)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// 获取所有服务器类型
    pub fn get_all_server_types(&self) -> Vec<String> {
        self.servers_by_type.keys().cloned().collect()
    }

    /// 清空所有服务器
    pub fn clear(&mut self) {
        let count = self.servers.len();
        self.servers.clear();
        self.servers_by_type.clear();
        if count > 0 {
            info!("Cleared {} servers", count);
        }
    }

    /// 获取服务器列表（用于发送给新注册的节点）
    pub fn get_server_list(&self) -> Vec<ServerInfo> {
        self.servers.values().cloned().collect()
    }
}

impl Default for ServerManager {
    fn default() -> Self {
        Self::new()
    }
}