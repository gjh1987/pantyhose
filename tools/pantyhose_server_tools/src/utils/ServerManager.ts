export interface ServerStatusEvent {
  serverId: string
  isRunning: boolean
  timestamp: number
}

class ServerManager {
  private listeners: Map<string, Set<(event: ServerStatusEvent) => void>> = new Map()
  private globalListeners: Set<(statusMap: Record<string, boolean>) => void> = new Set()
  private statusCache: Map<string, boolean> = new Map()
  
  constructor() {
    console.log('ServerManager initialized')
  }
  
  addListener(serverId: string, callback: (event: ServerStatusEvent) => void) {
    if (!this.listeners.has(serverId)) {
      this.listeners.set(serverId, new Set())
    }
    this.listeners.get(serverId)!.add(callback)
    console.log(`Added listener for server ${serverId}`)
  }
  
  removeListener(serverId: string, callback: (event: ServerStatusEvent) => void) {
    const callbacks = this.listeners.get(serverId)
    if (callbacks) {
      callbacks.delete(callback)
      if (callbacks.size === 0) {
        this.listeners.delete(serverId)
      }
    }
  }
  
  clearListeners(serverId: string) {
    this.listeners.delete(serverId)
    console.log(`Cleared all listeners for server ${serverId}`)
  }
  
  addGlobalListener(callback: (statusMap: Record<string, boolean>) => void) {
    this.globalListeners.add(callback)
    console.log('Added global status listener')
  }
  
  removeGlobalListener(callback: (statusMap: Record<string, boolean>) => void) {
    this.globalListeners.delete(callback)
  }
  
  handleStatusChange(serverId: string, isRunning: boolean) {
    const previousStatus = this.statusCache.get(serverId)
    
    if (previousStatus !== isRunning) {
      this.statusCache.set(serverId, isRunning)
      
      const event: ServerStatusEvent = {
        serverId,
        isRunning,
        timestamp: Date.now()
      }
      
      console.log(`Server status changed: ${serverId} is now ${isRunning ? 'running' : 'stopped'}`)
      
      const callbacks = this.listeners.get(serverId)
      if (callbacks) {
        callbacks.forEach(callback => {
          try {
            callback(event)
          } catch (error) {
            console.error(`Error in event listener for server ${serverId}:`, error)
          }
        })
      }
    }
  }
  
  updateAllStatus(statusMap: Record<string, boolean>) {
    console.log('Received status update from Rust:', statusMap)
    
    for (const [serverId, isRunning] of Object.entries(statusMap)) {
      this.handleStatusChange(serverId, isRunning)
    }
    
    this.globalListeners.forEach(callback => {
      try {
        callback(statusMap)
      } catch (error) {
        console.error('Error in global status listener:', error)
      }
    })
  }
  
  onServerStart(serverId: string) {
    this.handleStatusChange(serverId, true)
  }
  
  onServerStop(serverId: string) {
    this.handleStatusChange(serverId, false)
  }
  
  getStatus(serverId: string): boolean | undefined {
    return this.statusCache.get(serverId)
  }
  
  getAllStatus(): Record<string, boolean> {
    const statusMap: Record<string, boolean> = {}
    this.statusCache.forEach((isRunning, serverId) => {
      statusMap[serverId] = isRunning
    })
    return statusMap
  }
}

const serverManager = new ServerManager()

if (typeof window !== 'undefined') {
  (window as any).ServerManager = serverManager
}

export default serverManager