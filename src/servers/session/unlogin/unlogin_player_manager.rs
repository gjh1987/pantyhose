use std::collections::HashMap;
use tracing::{debug, info, warn, error};
use super::unlogin_player::UnloginPlayer;

/// 未登录玩家管理器
/// 管理所有尚未完成登录验证的玩家会话
/// 单例，被SessionServer持有
pub struct UnloginPlayerManager {
    /// 未登录玩家映射表 (session_id -> UnloginPlayer)
    players: HashMap<u64, UnloginPlayer>,
    /// 登录超时时间（秒）
    login_timeout: u64,
}

impl UnloginPlayerManager {
    /// 创建新的未登录玩家管理器
    pub fn new() -> Self {
        Self {
            players: HashMap::new(),
            login_timeout: 60, // 默认60秒超时
        }
    }

    /// 初始化管理器
    pub fn init(&mut self) -> bool {
        debug!("UnloginPlayerManager initializing");
        
        // 清空现有数据
        self.players.clear();
        
        // 设置登录超时时间（可以从配置读取）
        self.login_timeout = 60;
        
        true
    }

    /// 添加未登录玩家
    pub fn add_player(&mut self, session_id: u64, token: String) -> bool {
        if self.players.contains_key(&session_id) {
            warn!("UnloginPlayer with session_id {} already exists", session_id);
            return false;
        }

        let player = UnloginPlayer::new(session_id, token.clone());

        self.players.insert(session_id, player);
        debug!("Added unlogin player: session_id={}, token={}", session_id, token);
        true
    }

    /// 移除未登录玩家
    pub fn remove_player(&mut self, session_id: u64) -> Option<UnloginPlayer> {
        let player = self.players.remove(&session_id);
        if player.is_some() {
            debug!("Removed unlogin player: session_id={}", session_id);
        }
        player
    }

    /// 获取未登录玩家
    pub fn get_player(&self, session_id: u64) -> Option<&UnloginPlayer> {
        self.players.get(&session_id)
    }

    /// 获取未登录玩家（可变引用）
    pub fn get_player_mut(&mut self, session_id: u64) -> Option<&mut UnloginPlayer> {
        self.players.get_mut(&session_id)
    }

    /// 检查玩家是否存在
    pub fn has_player(&self, session_id: u64) -> bool {
        self.players.contains_key(&session_id)
    }

    /// 清理超时的未登录玩家
    pub fn cleanup_timeout_players(&mut self) -> Vec<u64> {
        let now = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_secs(self.login_timeout);
        
        let mut timeout_sessions = Vec::new();
        
        // 找出所有超时的会话
        for (session_id, player) in self.players.iter() {
            if player.is_timeout(timeout_duration) {
                timeout_sessions.push(*session_id);
            }
        }
        
        // 移除超时的会话
        for session_id in &timeout_sessions {
            self.players.remove(session_id);
            warn!("Removed timeout unlogin player: session_id={}", session_id);
        }
        
        if !timeout_sessions.is_empty() {
            info!("Cleaned up {} timeout unlogin players", timeout_sessions.len());
        }
        
        timeout_sessions
    }

    /// 获取当前未登录玩家数量
    pub fn get_player_count(&self) -> usize {
        self.players.len()
    }

    /// 获取所有未登录玩家的会话ID
    pub fn get_all_session_ids(&self) -> Vec<u64> {
        self.players.keys().cloned().collect()
    }

    /// 验证玩家token
    pub fn verify_player_token(&self, session_id: u64, token: &str) -> bool {
        if let Some(player) = self.players.get(&session_id) {
            return player.verify_token(token);
        }
        false
    }

    /// 更新玩家token
    pub fn update_player_token(&mut self, session_id: u64, new_token: String) -> bool {
        if let Some(player) = self.players.get_mut(&session_id) {
            player.update_token(new_token);
            debug!("Updated token for unlogin player: session_id={}", session_id);
            return true;
        }
        false
    }

    /// 设置登录超时时间
    pub fn set_login_timeout(&mut self, timeout: u64) {
        self.login_timeout = timeout;
        info!("Login timeout set to {} seconds", timeout);
    }

    /// 获取登录超时时间
    pub fn get_login_timeout(&self) -> u64 {
        self.login_timeout
    }

    /// 清理管理器
    pub fn dispose(&mut self) {
        debug!("UnloginPlayerManager disposing");
        
        let player_count = self.players.len();
        self.players.clear();
        
        info!("UnloginPlayerManager disposed, cleared {} unlogin players", player_count);
    }
}