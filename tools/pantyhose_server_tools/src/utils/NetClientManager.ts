// 网络客户端管理器 - 处理 Rust 端的事件通知
export interface ClientEventData {
  clientId: number
  eventType: 'connected' | 'disconnected' | 'message' | 'error'
  data?: any
  msgId?: number  // 添加消息ID
  byteLen?: number  // 添加字节长度
  bytes?: number[]  // 添加字节数组
  error?: string
}

class NetClientManager {
  private listeners: Map<number, Set<(event: ClientEventData) => void>> = new Map()
  private nestMsgId = 0;
  public get NestMsgId(){ this.nestMsgId++; return this.nestMsgId;}
  
  constructor() {
    console.log('NetClientManager initialized')
  }
  
  // 注册客户端事件监听器
  addListener(clientId: number, callback: (event: ClientEventData) => void) {
    if (!this.listeners.has(clientId)) {
      this.listeners.set(clientId, new Set())
    }
    this.listeners.get(clientId)!.add(callback)
    console.log(`Added listener for client ${clientId}`)
  }
  
  // 移除客户端事件监听器
  removeListener(clientId: number, callback: (event: ClientEventData) => void) {
    const callbacks = this.listeners.get(clientId)
    if (callbacks) {
      callbacks.delete(callback)
      if (callbacks.size === 0) {
        this.listeners.delete(clientId)
      }
    }
  }
  
  // 清除客户端的所有监听器
  clearListeners(clientId: number) {
    this.listeners.delete(clientId)
    console.log(`Cleared all listeners for client ${clientId}`)
  }
  
  // 处理来自 Rust 端的事件（由 Rust 通过 eval 调用）
  handleEvent(event: ClientEventData) {
    console.log('Received event from Rust:', event)
    
    const callbacks = this.listeners.get(event.clientId)
    if (callbacks) {
      callbacks.forEach(callback => {
        try {
          callback(event)
        } catch (error) {
          console.error(`Error in event listener for client ${event.clientId}:`, error)
        }
      })
    } else {
      console.warn(`No listeners registered for client ${event.clientId}`)
    }
  }
  
  // Rust 端调用的便捷方法
  onConnect(clientId: number) {
    this.handleEvent({
      clientId,
      eventType: 'connected'
    })
  }
  
  onDisconnect(clientId: number) {
    this.handleEvent({
      clientId,
      eventType: 'disconnected'
    })
  }
  
  onMessage(clientId: number, msgId: number, byteLen: number, bytes: number[]) {
    this.handleEvent({
      clientId,
      eventType: 'message',
      msgId,
      byteLen,
      bytes
    })
  }
  
  // 保留旧的文本消息方法以兼容
  onTextMessage(clientId: number, message: string) {
    this.handleEvent({
      clientId,
      eventType: 'message',
      data: message
    })
  }
  
  onError(clientId: number, error: string) {
    this.handleEvent({
      clientId,
      eventType: 'error',
      error
    })
  }
}

// 创建全局实例
const netClientManager = new NetClientManager()

// 注册到 window 对象，供 Rust 端调用
if (typeof window !== 'undefined') {
  (window as any).NetClientManager = netClientManager
}

export default netClientManager