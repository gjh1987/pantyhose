use crate::framework::data::dynamic_buffer::DynamicBuffer;

/// 消息处理器trait，定义消息处理的基本接口
/// 这是一个无状态的接口，所有方法都是只读的，不存在竞争问题
pub trait MsgProcessor: Send + Sync {
    /// 解码消息
    /// 
    /// # 参数
    /// * `message_id` - 消息ID
    /// * `buffer` - 包含消息数据的动态缓冲区
    /// * `length` - 消息长度
    /// 
    /// # 返回值
    /// 成功返回解码后的消息对象，失败返回None
    fn decode_message(&self, message_id: u16, buffer: &mut DynamicBuffer, length: usize) -> Option<Box<dyn std::any::Any + Send>>;
}
