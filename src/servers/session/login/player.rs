/// 已登录玩家信息
pub struct Player {
    /// 玩家ID
    pub player_id: u64,
    /// 玩家名称
    pub name: String,
    /// 会话ID
    pub session_id: u64,
    /// 登录时间
    pub login_time: std::time::Instant,
    /// 最后活动时间
    pub last_active_time: std::time::Instant,
}

impl Player {
    /// 创建新的玩家
    pub fn new(player_id: u64, name: String, session_id: u64) -> Self {
        let now = std::time::Instant::now();
        Self {
            player_id,
            name,
            session_id,
            login_time: now,
            last_active_time: now,
        }
    }

    /// 更新玩家活动时间
    pub fn update_activity(&mut self) {
        self.last_active_time = std::time::Instant::now();
    }

    /// 检查玩家是否超时
    pub fn is_timeout(&self, timeout_duration: std::time::Duration) -> bool {
        std::time::Instant::now().duration_since(self.last_active_time) > timeout_duration
    }

    /// 获取在线时长（秒）
    pub fn get_online_duration(&self) -> u64 {
        std::time::Instant::now().duration_since(self.login_time).as_secs()
    }

    /// 获取空闲时长（秒）
    pub fn get_idle_duration(&self) -> u64 {
        std::time::Instant::now().duration_since(self.last_active_time).as_secs()
    }

    /// 检查是否刚登录（指定秒数内）
    pub fn is_recently_logged_in(&self, seconds: u64) -> bool {
        self.get_online_duration() <= seconds
    }

    /// 检查是否活跃（指定秒数内有活动）
    pub fn is_active(&self, seconds: u64) -> bool {
        self.get_idle_duration() <= seconds
    }
}