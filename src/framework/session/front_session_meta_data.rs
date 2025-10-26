use std::collections::HashMap;
use tracing::{info, debug};

/// 前端会话元数据
#[derive(Debug, Clone)]
pub struct FrontSessionMetaData {
    /// 服务器元数据映射 <服务器类型, 服务器ID>
    pub server_meta: HashMap<String, u32>,
}

impl FrontSessionMetaData {
    /// 创建新的前端会话元数据
    pub fn new() -> Self {
        Self {
            server_meta: HashMap::new(),
        }
    }

    /// 添加服务器元数据
    /// 
    /// # 参数
    /// * `server_type` - 服务器类型
    /// * `server_id` - 服务器ID
    /// 
    /// # 返回值
    /// 成功返回true，失败返回false
    pub fn add_server_meta(&mut self, server_type: String, server_id: u32) -> bool {
        let old_value = self.server_meta.insert(server_type.clone(), server_id);
        if let Some(old_id) = old_value {
            if old_id != server_id {
                info!("Updated server meta for type '{}': {} -> {}", server_type, old_id, server_id);
            }
        } else {
            info!("Added server meta for type '{}': {}", server_type, server_id);
        }
        true
    }

    /// 移除服务器元数据
    /// 
    /// # 参数
    /// * `server_type` - 服务器类型
    /// 
    /// # 返回值
    /// 成功返回true，失败返回false
    pub fn remove_server_meta(&mut self, server_type: &str) -> bool {
        if let Some(server_id) = self.server_meta.remove(server_type) {
            info!("Removed server meta for type '{}': {}", server_type, server_id);
            true
        } else {
            debug!("Server meta not found for type '{}'", server_type);
            false
        }
    }

    /// 获取服务器ID
    /// 
    /// # 参数
    /// * `server_type` - 服务器类型
    /// 
    /// # 返回值
    /// 如果存在返回服务器ID，否则返回None
    pub fn get_server_id(&self, server_type: &str) -> Option<u32> {
        self.server_meta.get(server_type).copied()
    }

    /// 检查是否包含服务器类型
    /// 
    /// # 参数
    /// * `server_type` - 服务器类型
    /// 
    /// # 返回值
    /// 存在返回true，不存在返回false
    pub fn has_server_type(&self, server_type: &str) -> bool {
        self.server_meta.contains_key(server_type)
    }

    /// 获取所有服务器类型
    pub fn get_server_types(&self) -> Vec<String> {
        self.server_meta.keys().cloned().collect()
    }

    /// 获取服务器元数据数量
    pub fn get_server_meta_count(&self) -> usize {
        self.server_meta.len()
    }

    /// 清空所有服务器元数据
    pub fn clear(&mut self) {
        let count = self.server_meta.len();
        self.server_meta.clear();
        if count > 0 {
            info!("Cleared {} server meta entries", count);
        }
    }

    /// 获取服务器元数据的引用
    pub fn get_server_meta(&self) -> &HashMap<String, u32> {
        &self.server_meta
    }

    /// 获取服务器元数据的可变引用
    pub fn get_server_meta_mut(&mut self) -> &mut HashMap<String, u32> {
        &mut self.server_meta
    }
}

impl Default for FrontSessionMetaData {
    fn default() -> Self {
        Self::new()
    }
}