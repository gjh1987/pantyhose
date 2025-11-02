use std::collections::HashMap;
use tracing::error;
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
        
        true
    }

    /// 清理管理器
    pub fn dispose(&mut self) {
        let player_count = self.players_by_id.len();
        self.players_by_id.clear();
        self.session_to_player.clear();
        self.name_to_player.clear();
    }
}