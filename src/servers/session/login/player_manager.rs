use std::collections::HashMap;
use tracing::{debug, info, warn, error};
use super::player::Player;

/// 已登录玩家管理器
/// 管理所有已完成登录验证的玩家
/// 单例，被SessionServer持有
pub struct PlayerManager {
    /// 玩家映射表 (player_id -> Player)
    players_by_id: HashMap<u64, Player>,
    /// 会话ID到玩家ID的映射 (session_id -> player_id)
    session_to_player: HashMap<u64, u64>,
    /// 玩家名称到玩家ID的映射 (name -> player_id)
    name_to_player: HashMap<String, u64>,
    /// 活动超时时间（秒）
    activity_timeout: u64,
}

impl PlayerManager {
    /// 创建新的玩家管理器
    pub fn new() -> Self {
        Self {
            players_by_id: HashMap::new(),
            session_to_player: HashMap::new(),
            name_to_player: HashMap::new(),
            activity_timeout: 300, // 默认5分钟超时
        }
    }

    /// 初始化管理器
    pub fn init(&mut self) -> bool {
        debug!("PlayerManager initializing");
        
        // 清空现有数据
        self.players_by_id.clear();
        self.session_to_player.clear();
        self.name_to_player.clear();
        
        // 设置活动超时时间（可以从配置读取）
        self.activity_timeout = 300;
        
        true
    }

    /// 添加已登录玩家
    pub fn add_player(&mut self, player_id: u64, name: String, session_id: u64) -> bool {
        // 检查玩家ID是否已存在
        if self.players_by_id.contains_key(&player_id) {
            warn!("Player with id {} already exists", player_id);
            return false;
        }

        // 检查会话ID是否已被使用
        if self.session_to_player.contains_key(&session_id) {
            warn!("Session {} is already associated with another player", session_id);
            return false;
        }

        // 检查名称是否已被使用
        if self.name_to_player.contains_key(&name) {
            warn!("Player name '{}' is already taken", name);
            return false;
        }

        let player = Player::new(player_id, name.clone(), session_id);

        // 添加到所有映射表
        self.players_by_id.insert(player_id, player);
        self.session_to_player.insert(session_id, player_id);
        self.name_to_player.insert(name.clone(), player_id);

        info!("Added player: id={}, name={}, session_id={}", player_id, name, session_id);
        true
    }

    /// 移除玩家
    pub fn remove_player(&mut self, player_id: u64) -> Option<Player> {
        if let Some(player) = self.players_by_id.remove(&player_id) {
            // 从其他映射表中移除
            self.session_to_player.remove(&player.session_id);
            self.name_to_player.remove(&player.name);
            
            info!("Removed player: id={}, name={}, session_id={}", 
                  player_id, player.name, player.session_id);
            return Some(player);
        }
        None
    }

    /// 通过会话ID移除玩家
    pub fn remove_player_by_session(&mut self, session_id: u64) -> Option<Player> {
        if let Some(&player_id) = self.session_to_player.get(&session_id) {
            return self.remove_player(player_id);
        }
        None
    }

    /// 获取玩家
    pub fn get_player(&self, player_id: u64) -> Option<&Player> {
        self.players_by_id.get(&player_id)
    }

    /// 获取玩家（可变引用）
    pub fn get_player_mut(&mut self, player_id: u64) -> Option<&mut Player> {
        self.players_by_id.get_mut(&player_id)
    }

    /// 通过会话ID获取玩家
    pub fn get_player_by_session(&self, session_id: u64) -> Option<&Player> {
        if let Some(&player_id) = self.session_to_player.get(&session_id) {
            return self.players_by_id.get(&player_id);
        }
        None
    }

    /// 通过会话ID获取玩家（可变引用）
    pub fn get_player_by_session_mut(&mut self, session_id: u64) -> Option<&mut Player> {
        if let Some(&player_id) = self.session_to_player.get(&session_id) {
            return self.players_by_id.get_mut(&player_id);
        }
        None
    }

    /// 通过名称获取玩家
    pub fn get_player_by_name(&self, name: &str) -> Option<&Player> {
        if let Some(&player_id) = self.name_to_player.get(name) {
            return self.players_by_id.get(&player_id);
        }
        None
    }

    /// 检查玩家是否存在
    pub fn has_player(&self, player_id: u64) -> bool {
        self.players_by_id.contains_key(&player_id)
    }

    /// 检查会话是否已登录
    pub fn is_session_logged_in(&self, session_id: u64) -> bool {
        self.session_to_player.contains_key(&session_id)
    }

    /// 检查名称是否已被使用
    pub fn is_name_taken(&self, name: &str) -> bool {
        self.name_to_player.contains_key(name)
    }

    /// 更新玩家活动时间
    pub fn update_player_activity(&mut self, player_id: u64) -> bool {
        if let Some(player) = self.players_by_id.get_mut(&player_id) {
            player.update_activity();
            return true;
        }
        false
    }

    /// 通过会话ID更新玩家活动时间
    pub fn update_player_activity_by_session(&mut self, session_id: u64) -> bool {
        if let Some(&player_id) = self.session_to_player.get(&session_id) {
            return self.update_player_activity(player_id);
        }
        false
    }

    /// 清理超时的玩家
    pub fn cleanup_timeout_players(&mut self) -> Vec<u64> {
        let timeout_duration = std::time::Duration::from_secs(self.activity_timeout);
        let mut timeout_players = Vec::new();
        
        // 找出所有超时的玩家
        for (player_id, player) in self.players_by_id.iter() {
            if player.is_timeout(timeout_duration) {
                timeout_players.push(*player_id);
            }
        }
        
        // 移除超时的玩家
        for player_id in &timeout_players {
            if let Some(player) = self.remove_player(*player_id) {
                warn!("Removed timeout player: id={}, name={}", player_id, player.name);
            }
        }
        
        if !timeout_players.is_empty() {
            info!("Cleaned up {} timeout players", timeout_players.len());
        }
        
        timeout_players
    }

    /// 获取当前在线玩家数量
    pub fn get_player_count(&self) -> usize {
        self.players_by_id.len()
    }

    /// 获取所有玩家ID
    pub fn get_all_player_ids(&self) -> Vec<u64> {
        self.players_by_id.keys().cloned().collect()
    }

    /// 获取所有玩家名称
    pub fn get_all_player_names(&self) -> Vec<String> {
        self.name_to_player.keys().cloned().collect()
    }

    /// 获取所有会话ID
    pub fn get_all_session_ids(&self) -> Vec<u64> {
        self.session_to_player.keys().cloned().collect()
    }

    /// 设置活动超时时间
    pub fn set_activity_timeout(&mut self, timeout: u64) {
        self.activity_timeout = timeout;
        info!("Activity timeout set to {} seconds", timeout);
    }

    /// 获取活动超时时间
    pub fn get_activity_timeout(&self) -> u64 {
        self.activity_timeout
    }

    /// 获取玩家统计信息
    pub fn get_statistics(&self) -> PlayerStatistics {
        let mut total_online_time = 0u64;
        let mut longest_online_time = 0u64;
        let mut newest_player_id = 0u64;
        let mut newest_login_time = None;

        for (player_id, player) in self.players_by_id.iter() {
            let online_time = player.get_online_duration();
            total_online_time += online_time;
            
            if online_time > longest_online_time {
                longest_online_time = online_time;
            }
            
            if newest_login_time.is_none() || player.login_time > newest_login_time.unwrap() {
                newest_login_time = Some(player.login_time);
                newest_player_id = *player_id;
            }
        }

        let player_count = self.players_by_id.len();
        let average_online_time = if player_count > 0 {
            total_online_time / player_count as u64
        } else {
            0
        };

        PlayerStatistics {
            total_players: player_count,
            average_online_time,
            longest_online_time,
            newest_player_id,
        }
    }

    /// 清理管理器
    pub fn dispose(&mut self) {
        debug!("PlayerManager disposing");
        
        let player_count = self.players_by_id.len();
        self.players_by_id.clear();
        self.session_to_player.clear();
        self.name_to_player.clear();
        
        info!("PlayerManager disposed, cleared {} players", player_count);
    }
}

/// 玩家统计信息
#[derive(Debug, Clone)]
pub struct PlayerStatistics {
    pub total_players: usize,
    pub average_online_time: u64,  // 秒
    pub longest_online_time: u64,  // 秒
    pub newest_player_id: u64,
}