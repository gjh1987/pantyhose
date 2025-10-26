use crate::framework::session::{FrontSession, BackSessionManager};
use crate::framework::session::session_trait::SessionTrait;
use std::collections::HashMap;
use tracing::{info, error, debug, warn};

/// 路由函数类型定义
/// 返回 session_id 而不是 session 引用，避免生命周期问题
/// 
/// # 参数
/// * target_server_type - 目标服务器类型
/// * front_session - 可选的前端会话（如果是前端发起的请求，可变引用以便设置metadata）
/// * back_session_manager - 后端会话管理器
pub type RouterFunction = Box<dyn Fn(
    &str,  // target_server_type
    Option<&mut FrontSession>,
    &BackSessionManager,
) -> Option<u64> + Send + Sync>;

/// 路由管理器，管理多个路由函数
pub struct RouterManager {
    /// 路由函数映射表 <服务器类型, 路由函数>
    routers: HashMap<String, RouterFunction>,
    /// 默认路由函数，用于未配置的server_type
    default_router: RouterFunction,
}

impl RouterManager {
    /// 创建新的路由管理器
    pub fn new() -> Self {
        // 默认路由函数：优先检查 front session metadata，否则随机选择
        let default_router: RouterFunction = Box::new(|target_server_type, mut front_session, back_session_manager| {
            debug!("Default router handling: {}", target_server_type);
            
            // 首先尝试从 front session metadata 获取
            if let Some(ref front) = front_session {
                let metadata = front.get_metadata();
                if let Some(server_id) = metadata.get_server_id(target_server_type) {
                    info!("Found server ID {} from metadata for type: {}", server_id, target_server_type);
                    
                    // 检查该 server_id 对应的会话是否存在
                    if back_session_manager.get_session(server_id as u64).is_some() {
                        debug!("Found back session with ID: {}", server_id);
                        return Some(server_id as u64);
                    } else {
                        warn!("Server ID {} from metadata not found in active sessions", server_id);
                    }
                }
            }
            
            // 如果没有 front session 或获取 server_id 失败，随机选择一个
            debug!("Attempting to randomly select a back session for type: {}", target_server_type);
            
            // 获取指定类型的所有活跃会话
            let active_sessions = back_session_manager.get_active_sessions(target_server_type);
            
            if !active_sessions.is_empty() {
                // 随机选择一个会话
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let index = rng.gen_range(0..active_sessions.len());
                let selected_session = &active_sessions[index];
                let session_id = selected_session.get_session_id();
                
                info!("Selected back session {} for type: {} (from {} available)", 
                      session_id, target_server_type, active_sessions.len());
                
                // 如果有front session，将选中的server_id设置到metadata中
                if let Some(ref mut front) = front_session {
                    let metadata = front.get_metadata_mut();
                    metadata.add_server_meta(target_server_type.to_string(), session_id as u32);
                    debug!("Set server_id {} for type {} in front session metadata", session_id, target_server_type);
                }
                
                return Some(session_id);
            }
            
            warn!("No active back sessions found for type: {}", target_server_type);
            None
        });
        
        Self {
            routers: HashMap::new(),
            default_router,
        }
    }

    /// 初始化路由管理器
    pub fn init(&mut self) -> bool {
        true
    }

    /// 添加路由函数
    /// 
    /// # 参数
    /// * `server_type` - 服务器类型
    /// * `router_fn` - 路由函数
    /// 
    /// # 返回值
    /// 成功返回true，失败返回false
    pub fn add_router(&mut self, server_type: String, router_fn: RouterFunction) -> bool {
        if self.routers.contains_key(&server_type) {
            warn!("Router for server type '{}' already exists", server_type);
            return false;
        }

        self.routers.insert(server_type.clone(), router_fn);
        info!("Added router function for server type: {}", server_type);
        true
    }

    /// 移除路由函数
    /// 
    /// # 参数
    /// * `server_type` - 服务器类型
    /// 
    /// # 返回值
    /// 成功返回true，失败返回false
    pub fn remove_router(&mut self, server_type: &str) -> bool {
        if self.routers.remove(server_type).is_some() {
            info!("Removed router function for server type: {}", server_type);
            true
        } else {
            warn!("Router function for server type '{}' not found", server_type);
            false
        }
    }

    /// 获取路由函数
    /// 
    /// # 参数
    /// * `server_type` - 服务器类型
    /// 
    /// # 返回值
    /// 路由函数的引用，如果不存在返回默认路由函数
    pub fn get_router(&self, server_type: &str) -> &RouterFunction {
        self.routers.get(server_type).unwrap_or(&self.default_router)
    }

    /// 检查是否存在指定类型的路由器
    /// 
    /// # 参数
    /// * `server_type` - 服务器类型
    /// 
    /// # 返回值
    /// 存在返回true，不存在返回false
    pub fn has_router(&self, server_type: &str) -> bool {
        self.routers.contains_key(server_type)
    }

    /// 获取路由器数量
    pub fn get_router_count(&self) -> usize {
        self.routers.len()
    }

    /// 获取所有服务器类型
    pub fn get_server_types(&self) -> Vec<String> {
        self.routers.keys().cloned().collect()
    }


    /// 销毁所有路由函数
    pub fn dispose(&mut self) {
        info!("Disposing RouterManager with {} routers", self.routers.len());
        
        // 清空所有路由函数
        self.routers.clear();
        
        info!("RouterManager disposed");
    }
}

impl Drop for RouterManager {
    fn drop(&mut self) {
        if !self.routers.is_empty() {
            self.dispose();
        }
    }
}