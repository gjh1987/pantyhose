/// 数据库玩家信息
///
/// 用于存储从数据库加载的玩家基础信息
pub struct DBPlayer {
    /// 玩家ID
    pub player_id: u64,
    /// 玩家昵称
    pub nick_name: String,
    /// 玩家等级
    pub level: u32,
}

impl DBPlayer {
    /// 创建新的数据库玩家信息
    pub fn new(player_id: u64, nick_name: String, level: u32) -> Self {
        Self {
            player_id,
            nick_name,
            level,
        }
    }
}