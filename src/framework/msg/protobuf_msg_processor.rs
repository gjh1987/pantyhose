use super::msg_processor::MsgProcessor;
use crate::framework::data::dynamic_buffer::DynamicBuffer;
use crate::proto::messages::protobuf::message::protobuf_message_id::MessageFactory;
use tracing::{info, error, debug, warn};
use serde::{Serialize, Deserialize};

/// Protobuf消息处理器，专门处理protobuf格式的消息
/// 这是一个无状态的实现，不存在竞争问题
pub struct ProtobufMsgProcessor;

impl ProtobufMsgProcessor {
    /// 创建新的Protobuf消息处理器
    pub fn new() -> Self {
        Self
    }
    
    /// 根据消息类型编码消息
    /// 这个方法用于发送消息时的编码
    pub fn encode_typed_message<T: Serialize>(&self, message_type: u16, message: &T) -> Option<Vec<u8>> {
        // 序列化消息体
        match bincode::serialize(message) {
            Ok(body) => {
                let body_len = body.len() as u16;
                let mut result = Vec::with_capacity(4 + body.len());
                
                // 添加消息长度（u16，小端序）
                result.extend_from_slice(&body_len.to_le_bytes());
                // 添加消息ID（u16，小端序）
                result.extend_from_slice(&message_type.to_le_bytes());
                // 添加消息体
                result.extend_from_slice(&body);
                
                debug!("Encoded message: id={}, body_len={}, total_len={}", 
                       message_type, body_len, result.len());
                Some(result)
            }
            Err(e) => {
                error!("Failed to serialize message: {}", e);
                None
            }
        }
    }

    /// 从消息数据中获取消息头信息
    /// 消息格式：
    /// - 前2个字节：消息长度（u16，小端序）
    /// - 接下来2个字节：消息ID（u16，小端序）
    /// 
    /// # 参数
    /// * `data` - 消息数据
    /// 
    /// # 返回值
    /// 成功返回(消息长度, 消息ID)，失败返回None
    pub fn get_message_header(&self, data: &[u8]) -> Option<(u16, u16)> {
        if data.len() < 4 {
            error!("ProtobufMsgProcessor: message data too short, need at least 4 bytes for header");
            return None;
        }
        
        let message_length = u16::from_le_bytes([data[0], data[1]]);
        let message_id = u16::from_le_bytes([data[2], data[3]]);
        
        debug!("ProtobufMsgProcessor: extracted header - length: {}, id: {}", message_length, message_id);
        Some((message_length, message_id))
    }

    /// 从消息数据中获取消息ID
    /// 
    /// # 参数
    /// * `data` - 消息数据
    /// 
    /// # 返回值
    /// 成功返回消息ID，失败返回None
    pub fn get_message_id(&self, data: &[u8]) -> Option<u16> {
        if let Some((_, message_id)) = self.get_message_header(data) {
            Some(message_id)
        } else {
            None
        }
    }

    /// 解码消息数据
    /// 从完整的消息数据中提取消息体
    /// 
    /// # 参数
    /// * `data` - 完整的消息数据（包含长度、消息ID和消息体）
    /// 
    /// # 返回值
    /// 成功返回(消息ID, 消息体)，失败返回None
    pub fn decode_message<'a>(&self, data: &'a [u8]) -> Option<(u16, &'a [u8])> {
        // 获取消息头
        let (message_length, message_id) = self.get_message_header(data)?;
        
        // 验证消息长度
        if data.len() < 4 + message_length as usize {
            error!("ProtobufMsgProcessor: message data too short, expected {} bytes, got {}", 
                   4 + message_length, data.len());
            return None;
        }
        
        // 提取消息体（跳过前4个字节的头部）
        let message_body = &data[4..4 + message_length as usize];
        
        debug!("ProtobufMsgProcessor: decoded message id: {}, body length: {}", 
               message_id, message_body.len());
        
        Some((message_id, message_body))
    }

    /// 编码消息
    /// 将消息ID和消息体组合成完整的消息数据
    /// 
    /// # 参数
    /// * `message_id` - 消息ID (u16)
    /// * `message_body` - 消息体数据
    /// 
    /// # 返回值
    /// 返回编码后的完整消息数据
    pub fn encode_message(&self, message_id: u16, message_body: &[u8]) -> Vec<u8> {
        let message_length = message_body.len() as u16;
        let mut result = Vec::with_capacity(4 + message_body.len());
        
        // 前2个字节：消息长度（u16，小端序）
        result.extend_from_slice(&message_length.to_le_bytes());
        // 接下来2个字节：消息ID（u16，小端序）
        result.extend_from_slice(&message_id.to_le_bytes());
        // 消息体
        result.extend_from_slice(message_body);
        
        debug!("ProtobufMsgProcessor: encoded message id: {}, body length: {}, total length: {}", 
               message_id, message_length, result.len());
        
        result
    }

    /// 序列化消息对象并编码
    /// 将消息对象序列化并添加消息ID头
    /// 
    /// # 参数
    /// * `message_id` - 消息ID (u16)
    /// * `message` - 要序列化的消息对象
    /// 
    /// # 返回值
    /// 成功返回编码后的消息数据，失败返回None
    pub fn serialize_and_encode<T: Serialize>(&self, message_id: u16, message: &T) -> Option<Vec<u8>> {
        match bincode::serialize(message) {
            Ok(message_body) => {
                let encoded = self.encode_message(message_id, &message_body);
                Some(encoded)
            }
            Err(e) => {
                error!("ProtobufMsgProcessor: failed to serialize message: {}", e);
                None
            }
        }
    }

    /// 解码并反序列化消息
    /// 从完整的消息数据中解码并反序列化为具体的消息对象
    /// 
    /// # 参数
    /// * `data` - 完整的消息数据
    /// 
    /// # 返回值
    /// 成功返回(消息ID, 消息对象)，失败返回None
    pub fn decode_and_deserialize<T: for<'de> Deserialize<'de>>(&self, data: &[u8]) -> Option<(u16, T)> {
        if let Some((message_id, message_body)) = self.decode_message(data) {
            match bincode::deserialize::<T>(message_body) {
                Ok(message) => {
                    debug!("ProtobufMsgProcessor: successfully deserialized message id: {}", message_id);
                    Some((message_id, message))
                }
                Err(e) => {
                    error!("ProtobufMsgProcessor: failed to deserialize message body: {}", e);
                    None
                }
            }
        } else {
            None
        }
    }
}

impl MsgProcessor for ProtobufMsgProcessor {
    fn decode_message(&self, message_id: u16, buffer: &mut DynamicBuffer, length: usize) -> Option<Box<dyn std::any::Any + Send>> {
        // 调用 protobuf_message_id 的 decode_message 方法
        MessageFactory::decode_message(message_id, buffer, length)
    }
}