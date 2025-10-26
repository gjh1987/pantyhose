import { invoke } from '@tauri-apps/api/core'

export interface ProtoField {
  name: string
  type: string
  tag: number
  repeated?: boolean
  optional?: boolean
  comment?: string
}

export interface ProtoMessage {
  name: string
  fields: ProtoField[]
  comment?: string
  package?: string
  msgId?: number  // 添加消息ID
}

export interface ProtoEnum {
  name: string
  values: { name: string; value: number }[]
  comment?: string
  package?: string
}

export interface ProtoFile {
  fileName: string
  package?: string
  messages: ProtoMessage[]
  enums: ProtoEnum[]
  imports: string[]
}

class ProtocolManager {
  private protoFiles: Map<string, ProtoFile> = new Map()
  private allMessages: Map<string, ProtoMessage> = new Map()
  private allEnums: Map<string, ProtoEnum> = new Map()
  
  constructor() {
    console.log('ProtocolManager initialized')
  }
  
  async loadProtoDirectory(dirPath: string): Promise<void> {
    try {
      // 使用Rust命令读取所有proto文件
      const protoFiles = await invoke('read_proto_files', { protoPath: dirPath }) as Array<{
        file_name: string
        content: string
        path: string
      }>
      
      // 解析每个proto文件
      for (const file of protoFiles) {
        await this.parseProtoContent(file.file_name, file.content)
      }
      
      // 分配消息ID（按照proto-id-tool的逻辑）
      this.assignMessageIds()
      
      console.log(`Loaded ${this.protoFiles.size} proto files`)
      console.log(`Found ${this.allMessages.size} messages and ${this.allEnums.size} enums`)
    } catch (error) {
      console.error('Error loading proto directory:', error)
      throw error
    }
  }
  
  async parseProtoFile(filePath: string): Promise<ProtoFile> {
    try {
      // 使用Rust命令读取proto文件
      const file = await invoke('read_proto_file', { filePath }) as {
        file_name: string
        content: string
        path: string
      }
      
      return await this.parseProtoContent(file.file_name, file.content)
    } catch (error) {
      console.error(`Error parsing proto file ${filePath}:`, error)
      throw error
    }
  }
  
  async parseProtoContent(fileName: string, content: string): Promise<ProtoFile> {
    try {
      
      const protoFile: ProtoFile = {
        fileName,
        messages: [],
        enums: [],
        imports: []
      }
      
      const lines = content.split('\n')
      let currentPackage = ''
      let inMessage = false
      let inEnum = false
      let currentMessage: ProtoMessage | null = null
      let currentEnum: ProtoEnum | null = null
      let braceCount = 0
      
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i].trim()
        
        if (line.startsWith('//') || line === '') {
          continue
        }
        
        if (line.startsWith('syntax')) {
          continue
        }
        
        if (line.startsWith('package ')) {
          currentPackage = line.replace('package ', '').replace(';', '').trim()
          protoFile.package = currentPackage
          continue
        }
        
        if (line.startsWith('import ')) {
          const importPath = line.replace('import ', '').replace(/[";]/g, '').trim()
          protoFile.imports.push(importPath)
          continue
        }
        
        if (line.startsWith('message ') && !inMessage && !inEnum) {
          const messageName = line.replace('message ', '').replace('{', '').trim()
          currentMessage = {
            name: messageName,
            fields: [],
            package: currentPackage
          }
          
          const prevLine = i > 0 ? lines[i - 1].trim() : ''
          if (prevLine.startsWith('//')) {
            currentMessage.comment = prevLine.substring(2).trim()
          }
          
          inMessage = true
          braceCount = 1
          continue
        }
        
        if (line.startsWith('enum ') && !inMessage && !inEnum) {
          const enumName = line.replace('enum ', '').replace('{', '').trim()
          currentEnum = {
            name: enumName,
            values: [],
            package: currentPackage
          }
          
          const prevLine = i > 0 ? lines[i - 1].trim() : ''
          if (prevLine.startsWith('//')) {
            currentEnum.comment = prevLine.substring(2).trim()
          }
          
          inEnum = true
          braceCount = 1
          continue
        }
        
        if (line.includes('{')) {
          braceCount += (line.match(/{/g) || []).length
        }
        
        if (line.includes('}')) {
          braceCount -= (line.match(/}/g) || []).length
          
          if (braceCount === 0) {
            if (inMessage && currentMessage) {
              protoFile.messages.push(currentMessage)
              const fullName = currentPackage ? `${currentPackage}.${currentMessage.name}` : currentMessage.name
              this.allMessages.set(fullName, currentMessage)
              currentMessage = null
              inMessage = false
            } else if (inEnum && currentEnum) {
              protoFile.enums.push(currentEnum)
              const fullName = currentPackage ? `${currentPackage}.${currentEnum.name}` : currentEnum.name
              this.allEnums.set(fullName, currentEnum)
              currentEnum = null
              inEnum = false
            }
          }
          continue
        }
        
        if (inMessage && currentMessage && braceCount > 0) {
          const field = this.parseField(line)
          if (field) {
            const prevLine = i > 0 ? lines[i - 1].trim() : ''
            if (prevLine.startsWith('//')) {
              field.comment = prevLine.substring(2).trim()
            }
            currentMessage.fields.push(field)
          }
        }
        
        if (inEnum && currentEnum && braceCount > 0) {
          const enumValue = this.parseEnumValue(line)
          if (enumValue) {
            currentEnum.values.push(enumValue)
          }
        }
      }
      
      this.protoFiles.set(fileName, protoFile)
      return protoFile
    } catch (error) {
      console.error(`Error parsing proto content for ${fileName}:`, error)
      throw error
    }
  }
  
  private parseField(line: string): ProtoField | null {
    line = line.replace(/;.*$/, '').trim()
    
    if (!line || line.startsWith('//')) {
      return null
    }
    
    let repeated = false
    let optional = false
    
    if (line.startsWith('repeated ')) {
      repeated = true
      line = line.substring(9).trim()
    }
    
    if (line.startsWith('optional ')) {
      optional = true
      line = line.substring(9).trim()
    }
    
    const parts = line.split(/\s+/)
    if (parts.length < 3) {
      return null
    }
    
    const type = parts[0]
    const name = parts[1]
    const tag = parseInt(parts[parts.length - 1])
    
    if (isNaN(tag)) {
      return null
    }
    
    return {
      name,
      type,
      tag,
      repeated,
      optional
    }
  }
  
  private parseEnumValue(line: string): { name: string; value: number } | null {
    line = line.replace(/;.*$/, '').trim()
    
    if (!line || line.startsWith('//')) {
      return null
    }
    
    const parts = line.split('=')
    if (parts.length !== 2) {
      return null
    }
    
    const name = parts[0].trim()
    const value = parseInt(parts[1].trim())
    
    if (isNaN(value)) {
      return null
    }
    
    return { name, value }
  }
  
  getAllMessages(): ProtoMessage[] {
    return Array.from(this.allMessages.values())
  }
  
  getAllEnums(): ProtoEnum[] {
    return Array.from(this.allEnums.values())
  }
  
  getMessage(fullName: string): ProtoMessage | undefined {
    return this.allMessages.get(fullName)
  }
  
  getEnum(fullName: string): ProtoEnum | undefined {
    return this.allEnums.get(fullName)
  }
  
  getProtoFile(fileName: string): ProtoFile | undefined {
    return this.protoFiles.get(fileName)
  }
  
  getAllProtoFiles(): ProtoFile[] {
    return Array.from(this.protoFiles.values())
  }
  
  searchMessages(keyword: string): ProtoMessage[] {
    const results: ProtoMessage[] = []
    const lowerKeyword = keyword.toLowerCase()
    
    for (const message of this.allMessages.values()) {
      if (message.name.toLowerCase().includes(lowerKeyword) ||
          message.comment?.toLowerCase().includes(lowerKeyword)) {
        results.push(message)
      }
    }
    
    return results
  }
  
  getMessagesByPackage(packageName: string): ProtoMessage[] {
    const results: ProtoMessage[] = []
    
    for (const message of this.allMessages.values()) {
      if (message.package === packageName) {
        results.push(message)
      }
    }
    
    return results
  }
  
  // 根据消息类型获取消息ID
  getMessageId(messageType: string): number | undefined {
    const message = this.allMessages.get(messageType)
    return message?.msgId
  }
  
  // 分配消息ID（按照proto-id-tool的逻辑）
  private assignMessageIds(): void {
    // 收集所有消息并排序（按文件名和消息名）
    const allMessages: ProtoMessage[] = []
    
    // 按文件名排序（proto-id-tool的逻辑）
    const sortedFiles = Array.from(this.protoFiles.keys()).sort()
    
    for (const fileName of sortedFiles) {
      const protoFile = this.protoFiles.get(fileName)
      if (protoFile) {
        // proto-id-tool 会按照文件中出现的顺序处理消息
        // 并且会跳过 MetaEntry 消息
        const filteredMessages = protoFile.messages.filter(msg => msg.name !== 'MetaEntry')
        allMessages.push(...filteredMessages)
      }
    }
    
    // 从1开始分配消息ID（与proto-id-tool完全一致）
    let msgId = 1
    for (const message of allMessages) {
      message.msgId = msgId++
      
      console.log(`Assigned msgId ${message.msgId} to ${message.package ? message.package + '.' : ''}${message.name}`)
      
      // 更新allMessages Map中的消息
      const key = message.package ? `${message.package}.${message.name}` : message.name
      const storedMsg = this.allMessages.get(key)
      if (storedMsg) {
        storedMsg.msgId = message.msgId
      }
    }
    
    console.log('Assigned message IDs to', allMessages.length, 'messages')
  }
  
  clear(): void {
    this.protoFiles.clear()
    this.allMessages.clear()
    this.allEnums.clear()
  }
}

const protocolManager = new ProtocolManager()

export default protocolManager