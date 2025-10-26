import * as protobuf from 'protobufjs'
import { useTemplateStore } from '../stores/templateStore'
import protocolManager from './protocolManager'

export interface EncodeResult {
  msgId: number
  buffer: Uint8Array
  byteArray: number[]
  messageType: string
  messageData: any
}

export interface DecodeResult {
  type: string
  data: any
}

class MessageTransform {
  private static msgUniqueId = 1 // 静态消息唯一ID，每次递增
  
  // 获取下一个消息唯一ID
  private getNextMsgUniqueId(): number {
    return MessageTransform.msgUniqueId++
  }
  
  // 编码普通消息
  async encodeMessage(messageType: string, messageData: any): Promise<EncodeResult> {
    const templateStore = useTemplateStore()
    
    // 获取消息ID
    const msgId = protocolManager.getMessageId(messageType)
    if (!msgId) {
      throw new Error(`未找到消息类型 ${messageType} 的ID`)
    }
    
    // 获取消息定义
    const messageProto = templateStore.getProtoMessage(messageType)
    if (!messageProto) {
      throw new Error(`未找到消息定义: ${messageType}`)
    }
    
    // 动态构建protobuf消息类型
    const root = new protobuf.Root()
    const MessageType = new protobuf.Type(messageProto.name)
    
    // 添加字段到消息类型
    if (messageProto.fields) {
      messageProto.fields.forEach((field: any) => {
        const rule = field.repeated ? 'repeated' : (field.optional ? 'optional' : 'required')
        MessageType.add(new protobuf.Field(field.name, field.tag, field.type, rule))
      })
    }
    
    root.add(MessageType)
    
    // 验证消息格式
    const errMsg = MessageType.verify(messageData)
    if (errMsg) {
      throw new Error(`消息格式错误: ${errMsg}`)
    }
    
    // 序列化消息为字节数组
    const message = MessageType.create(messageData)
    const buffer = MessageType.encode(message).finish()
    
    return {
      msgId,
      buffer,
      byteArray: Array.from(buffer),
      messageType,
      messageData
    }
  }
  
  // 编码RPC请求消息
  async encodeRpcRequest(originalMessageType: string, messageData: any, serverType: string): Promise<EncodeResult> {
    // 首先编码原始消息
    const originalEncoded = await this.encodeMessage(originalMessageType, messageData)
    
    // 检查是否为Request类型
    if (!/Request$/i.test(originalMessageType)) {
      throw new Error(`消息类型 ${originalMessageType} 不是Request类型，无法创建RPC请求`)
    }
    
    // 创建RpcMessageFRequest数据
    const rpcMessageData = {
      msg_unique_id: this.getNextMsgUniqueId(), // 使用递增的唯一ID
      server_type: serverType,
      msg_id: originalEncoded.msgId,
      message: originalEncoded.byteArray
    }
    
    // 编码RPC消息
    const rpcEncoded = await this.encodeMessage('cluster.RpcMessageFRequest', rpcMessageData)
    
    return {
      ...rpcEncoded,
      messageData: {
        rpcMessage: rpcMessageData,
        originalMessage: {
          type: originalMessageType,
          data: messageData
        }
      }
    }
  }
  
  // 编码RPC通知消息
  async encodeRpcNotify(originalMessageType: string, messageData: any, serverType: string): Promise<EncodeResult> {
    // 首先编码原始消息
    const originalEncoded = await this.encodeMessage(originalMessageType, messageData)
    
    // 检查是否为Notify类型
    if (!/Notify$/i.test(originalMessageType)) {
      throw new Error(`消息类型 ${originalMessageType} 不是Notify类型，无法创建RPC通知`)
    }
    
    // 创建RpcMessageFNotify数据
    const rpcMessageData = {
      server_type: serverType,
      msg_id: originalEncoded.msgId,
      message: originalEncoded.byteArray
    }
    
    // 编码RPC消息
    const rpcEncoded = await this.encodeMessage('common.RpcMessageFNotify', rpcMessageData)
    
    return {
      ...rpcEncoded,
      messageData: {
        rpcMessage: rpcMessageData,
        originalMessage: {
          type: originalMessageType,
          data: messageData
        }
      }
    }
  }
  
  // 根据消息ID和字节数据解码消息
  async decodeMessage(msgId: number, bytes: Uint8Array): Promise<DecodeResult | null> {
    const templateStore = useTemplateStore()
    
    try {
      // 根据msgId获取消息类型
      const messageType = this.getMessageTypeByMsgId(msgId)
      if (!messageType) {
        console.error(`未找到msgId=${msgId}对应的消息类型`)
        return null
      }
      
      // 获取消息定义
      const messageProto = templateStore.getProtoMessage(messageType)
      if (!messageProto) {
        console.error(`未找到消息定义: ${messageType}`)
        return null
      }
      
      // 动态构建protobuf消息类型
      const root = new protobuf.Root()
      const MessageType = new protobuf.Type(messageProto.name)
      
      // 添加字段到消息类型
      if (messageProto.fields) {
        messageProto.fields.forEach((field: any) => {
          const rule = field.repeated ? 'repeated' : (field.optional ? 'optional' : 'required')
          MessageType.add(new protobuf.Field(field.name, field.tag, field.type, rule))
        })
      }
      
      root.add(MessageType)
      
      // 反序列化消息
      const message = MessageType.decode(bytes)
      const decodedData = MessageType.toObject(message)
      
      // 如果是RpcMessageFResponse，则继续解析内部message并直接返回
      if (messageType === 'cluster.RpcMessageFResponse' && decodedData.message && decodedData.msg_id) {
        try {
          const innerMessageBytes = new Uint8Array(decodedData.message)
          return await this.decodeMessage(decodedData.msg_id, innerMessageBytes)
        } catch (error) {
          console.warn('解析RpcMessageFResponse内部消息失败:', error)
          // 即使内部消息解析失败，也返回外层消息
        }
      }
      
      return {
        type: messageType,
        data: decodedData
      }
    } catch (error) {
      console.error('反序列化消息失败:', error)
      return null
    }
  }
  
  // 根据消息ID获取消息类型名称
  private getMessageTypeByMsgId(msgId: number): string | undefined {
    const templateStore = useTemplateStore()
    
    // 遍历所有消息查找匹配的msgId
    for (const msg of templateStore.protoMessages) {
      if (msg.msgId === msgId) {
        return msg.package ? `${msg.package}.${msg.name}` : msg.name
      }
    }
    return undefined
  }
  
  // 判断消息类型
  isRequestMessage(messageType: string): boolean {
    return /Request$/i.test(messageType)
  }
  
  isNotifyMessage(messageType: string): boolean {
    return /Notify$/i.test(messageType)
  }
  
  isRpcMessage(messageType: string): boolean {
    return messageType === 'cluster.RpcMessageFRequest' || 
           messageType === 'cluster.RpcMessageFNotify' ||
           messageType === 'cluster.RpcMessageFResponse' ||
           messageType === 'cluster.RpcForwardMessageBRequest' ||
           messageType === 'cluster.RpcForwardMessageBResponse' ||
           messageType === 'cluster.RpcForwardMessageBNotify'
  }
}

// 导出单例实例
const messageTransform = new MessageTransform()
export default messageTransform