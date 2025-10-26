use tracing::{debug, info};

/// 服务器信息
#[derive(Debug, Clone)]
pub struct ServerInfo {
    /// 服务器ID
    pub server_id: u32,
    /// 服务器类型
    pub server_type: String,
    /// 后端主机地址
    pub back_host: String,
    /// 后端TCP端口
    pub back_tcp_port: u32,
}

impl ServerInfo {
    /// 创建新的服务器实例
    pub fn new(server_id: u32, server_type: String, back_host: String, back_tcp_port: u32) -> Self {
        Self {
            server_id,
            server_type,
            back_host,
            back_tcp_port,
        }
    }

    /// 获取服务器ID
    pub fn get_server_id(&self) -> u32 {
        self.server_id
    }

    /// 获取服务器类型
    pub fn get_server_type(&self) -> &String {
        &self.server_type
    }

    /// 获取后端主机地址
    pub fn get_back_host(&self) -> &String {
        &self.back_host
    }

    /// 获取后端TCP端口
    pub fn get_back_tcp_port(&self) -> u32 {
        self.back_tcp_port
    }

    /// 获取服务器地址字符串
    pub fn get_address(&self) -> String {
        format!("{}:{}", self.back_host, self.back_tcp_port)
    }

    /// 获取服务器信息字符串
    pub fn get_info(&self) -> String {
        format!("Server[id={}, type={}, addr={}]", 
                self.server_id, self.server_type, self.get_address())
    }
}