/// 未登录玩家信息
pub struct UnloginPlayer {
    /// 会话ID
    pub session_id: u64,
    /// 登录令牌
    pub token: String,
    /// 登录时间
    pub login_time: std::time::Instant,
}

impl UnloginPlayer {
    /// 创建新的未登录玩家
    pub fn new(session_id: u64, token: String) -> Self {
        Self {
            session_id,
            token,
            login_time: std::time::Instant::now(),
        }
    }

    /// 检查是否超时
    pub fn is_timeout(&self, timeout_duration: std::time::Duration) -> bool {
        std::time::Instant::now().duration_since(self.login_time) > timeout_duration
    }

    /// 获取已等待时间（秒）
    pub fn get_wait_time(&self) -> u64 {
        std::time::Instant::now().duration_since(self.login_time).as_secs()
    }

    /// 更新令牌
    pub fn update_token(&mut self, new_token: String) {
        self.token = new_token;
    }

    /// 验证令牌
    pub fn verify_token(&self, token: &str) -> bool {
        self.token == token
    }
}