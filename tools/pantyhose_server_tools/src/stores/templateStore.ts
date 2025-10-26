import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import protocolManager, { type ProtoMessage } from '../utils/protocolManager'
import { serverStore } from './serverStore'

export interface MessageTemplate {
  name: string
  type: string
  content: string
}

interface TemplateState {
  templates: Map<string, MessageTemplate[]> // serverType -> templates
  isLoading: boolean
  protoMessages: ProtoMessage[] // 从proto文件解析的消息列表
  isProtoLoaded: boolean
}

export const useTemplateStore = defineStore('template', {
  state: (): TemplateState => ({
    templates: new Map(),
    isLoading: false,
    protoMessages: [],
    isProtoLoaded: false
  }),

  getters: {
    // 获取指定服务器类型的模板
    getTemplatesByServerType: (state) => {
      return (serverType: string): MessageTemplate[] => {
        return state.templates.get(serverType) || []
      }
    },
    
    // 获取所有proto消息名称列表
    getProtoMessageNames: (state): string[] => {
      return state.protoMessages.map(msg => {
        const fullName = msg.package ? `${msg.package}.${msg.name}` : msg.name
        return fullName
      })
    },
    
    // 根据名称获取proto消息
    getProtoMessage: (state) => {
      return (name: string): ProtoMessage | undefined => {
        return state.protoMessages.find(msg => {
          const fullName = msg.package ? `${msg.package}.${msg.name}` : msg.name
          return fullName === name || msg.name === name
        })
      }
    }
  },

  actions: {
    // 从 Rust 加载模板
    async loadTemplates(serverType: string) {
      this.isLoading = true
      try {
        const templates = await invoke('load_message_templates', { serverType })
        if (Array.isArray(templates)) {
          this.templates.set(serverType, templates)
        }
      } catch (error) {
        console.error('加载模板失败:', error)
        ElMessage.error(`加载模板失败: ${error}`)
      } finally {
        this.isLoading = false
      }
    },

    // 保存模板到 Rust
    async saveTemplates(serverType: string) {
      try {
        const templates = this.templates.get(serverType) || []
        await invoke('save_message_templates', {
          serverType,
          templates
        })
      } catch (error) {
        console.error('保存模板失败:', error)
        ElMessage.error(`保存模板失败: ${error}`)
        throw error
      }
    },

    // 添加模板
    async addTemplate(serverType: string, template: MessageTemplate) {
      const templates = this.templates.get(serverType) || []
      
      // 检查是否已存在同名模板
      const existingIndex = templates.findIndex(t => t.name === template.name)
      if (existingIndex >= 0) {
        // 覆盖现有模板
        templates[existingIndex] = template
      } else {
        // 添加新模板
        templates.push(template)
      }
      
      this.templates.set(serverType, templates)
      
      // 保存到 Rust
      await this.saveTemplates(serverType)
    },

    // 删除模板
    async deleteTemplate(serverType: string, templateName: string) {
      const templates = this.templates.get(serverType) || []
      const index = templates.findIndex(t => t.name === templateName)
      
      if (index >= 0) {
        templates.splice(index, 1)
        this.templates.set(serverType, templates)
        
        // 保存到 Rust
        await this.saveTemplates(serverType)
        ElMessage.success('模板已删除')
      }
    },

    // 更新模板
    async updateTemplate(serverType: string, oldName: string, newTemplate: MessageTemplate) {
      const templates = this.templates.get(serverType) || []
      const index = templates.findIndex(t => t.name === oldName)
      
      if (index >= 0) {
        templates[index] = newTemplate
        this.templates.set(serverType, templates)
        
        // 保存到 Rust
        await this.saveTemplates(serverType)
      }
    },

    // 初始化时加载所有模板
    async initializeTemplates() {
      // 这里可以加载常用的服务器类型的模板
      // 或者在需要时懒加载
    },
    
    // 加载proto文件中的消息定义
    async loadProtoMessages() {
      if (this.isProtoLoaded) {
        return // 已经加载过了
      }
      
      try {
        this.isLoading = true
        
        // 获取proto目录路径
        const protoPath = serverStore.protoPath
        console.log('Proto路径:', protoPath)
        
        if (!protoPath) {
          console.warn('Proto路径未配置')
          return
        }
        
        // 加载proto文件
        console.log('开始加载proto文件...')
        await protocolManager.loadProtoDirectory(protoPath)
        
        // 获取所有消息
        this.protoMessages = protocolManager.getAllMessages()
        this.isProtoLoaded = true
        
        console.log(`加载了 ${this.protoMessages.length} 个协议消息`)
      } catch (error) {
        console.error('加载Proto消息失败:', error)
        ElMessage.error(`加载协议失败: ${error}`)
      } finally {
        this.isLoading = false
      }
    },
    
    // 清除proto消息缓存
    clearProtoMessages() {
      protocolManager.clear()
      this.protoMessages = []
      this.isProtoLoaded = false
    },
    
    // 搜索proto消息
    searchProtoMessages(keyword: string): ProtoMessage[] {
      return protocolManager.searchMessages(keyword)
    },
    
    // 根据包名获取消息
    getMessagesByPackage(packageName: string): ProtoMessage[] {
      return protocolManager.getMessagesByPackage(packageName)
    }
  }
})