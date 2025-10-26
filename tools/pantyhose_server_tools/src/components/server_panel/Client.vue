<template>
  <div class="client-container">
    <!-- 客户端控制面板 -->
    <div class="client-header">
      <div class="header-left">
        <el-switch
          v-model="useWebSocket"
          :disabled="!canSwitchProtocol || isConnected"
          active-text="WS"
          inactive-text="TCP"
          style="margin-right: 10px;"
          @change="onProtocolChange"
        />
        
        <el-button 
          :type="isConnected ? 'danger' : 'success'"
          :icon="isConnected ? Link : Unlink"
          @click="toggleConnection"
          size="default"
        >
          {{ isConnected ? '断开连接' : '连接服务器' }}
        </el-button>
        
        <el-button 
          plain
          :icon="Edit"
          @click="() => $emit('rename', clientId)"
          size="default"
        >
          重命名
        </el-button>
        
        <el-tag :type="isConnected ? 'success' : 'info'" effect="dark">
          {{ isConnected ? '已连接' : '未连接' }}
        </el-tag>
        <el-tag type="info" v-if="currentHost && currentPort">
          {{ currentHost }}:{{ currentPort }}
        </el-tag>
      </div>
      
      <div class="header-right">
        <!-- 可以放其他内容，或者留空 -->
      </div>
    </div>

    <!-- 主内容区域 -->
    <el-splitter class="client-content" >
      <!-- 左侧消息发送面板 -->
      <el-splitter-panel>
        <div class="message-panel">
          <div class="message-panel-inner">
            <h4>消息发送</h4>
          
          <!-- 消息类型选择 -->
          <div class="message-type-container">
            <el-select 
              v-model="selectedMessageType" 
              placeholder="选择消息类型"
              class="message-type-select"
              filterable
              allow-create
            >
              <el-option-group
                v-for="group in messageTypes"
                :key="group.label"
                :label="group.label"
              >
                <el-option
                  v-for="item in group.options"
                  :key="item.value"
                  :label="item.label"
                  :value="item.value"
                />
              </el-option-group>
            </el-select>
            
            <el-checkbox v-model="isRpc">rpc</el-checkbox>
            
            <el-button 
              :icon="Plus"
              @click="addToTemplate"
              :disabled="!selectedMessageType || !messageContentJson || Object.keys(messageContentJson || {}).length === 0"
              title="添加到模板"
            >
              添加到模板
            </el-button>
          </div>
          
          <!-- 消息内容编辑器 -->
          <div class="message-editor">
            <Vue3JsonEditor
              :modelValue="messageContentJson"
              @update:modelValue="(val) => { 
                console.log('JSON Editor updated:', val)
                messageContentJson = val 
              }"
              :show-btns="false"
              mode="code"
              lang="zh"
              :expandedOnStart="true"
              @json-change="(val) => { 
                console.log('JSON changed:', val)
                messageContentJson = val 
              }"
              @has-error="(error) => { console.error('JSON Editor error:', error) }"
              style="height: 250px"
            />
            
            <!-- 发送按钮 -->
            <div class="send-actions">
              <el-button 
                type="primary" 
                :icon="Promotion"
                @click="sendMessage"
                :disabled="!isConnected || !selectedMessageType || !messageContentJson || Object.keys(messageContentJson || {}).length === 0"
              >
                发送消息
              </el-button>
              
              <el-button 
                plain
                :icon="Delete"
                @click="clearMessage"
              >
                清空
              </el-button>
            </div>
            
            <!-- 自定义模板 -->
            <div class="template-section">
              <div class="template-header">
                <span>自定义模板</span>
                <el-button size="small" text @click="refreshTemplates" :icon="Refresh">
                  刷新
                </el-button>
              </div>
              
              <!-- 自定义模板列表 -->
              <div v-if="customTemplates.length > 0" class="custom-templates">
                <div class="template-list">
                  <div v-for="template in customTemplates" :key="template.name" class="template-item">
                    <el-button 
                      size="small" 
                      @click="loadCustomTemplate(template)"
                      class="template-btn"
                    >
                      {{ template.name }}
                    </el-button>
                    <el-button 
                      size="small" 
                      :icon="Delete" 
                      text 
                      type="danger"
                      @click="deleteTemplate(template.name)"
                      title="删除模板"
                    />
                  </div>
                </div>
              </div>
              
              <!-- 空状态提示 -->
              <div v-else class="empty-templates">
                <el-empty description="暂无自定义模板" :image-size="60" />
              </div>
            </div>
          </div>
          </div>
        </div>
      </el-splitter-panel>
      
      <!-- 右侧消息日志面板 -->
      <el-splitter-panel>
        <div class="log-panel">
          <div class="log-header">
            <h4>消息日志</h4>
            <div class="log-actions">
              <el-checkbox v-model="autoScroll">自动滚动</el-checkbox>
              <el-button size="small" @click="clearLogs" :icon="Delete">
                清空日志
              </el-button>
            </div>
          </div>
          
          <!-- 日志内容 -->
          <div class="log-content" ref="logContainer">
            <div 
              v-for="(log, index) in messageLogs" 
              :key="index"
              class="log-item"
              :class="log.type"
            >
              <div class="log-time">{{ formatTime(log.timestamp) }}</div>
              <div class="log-type">
                <el-tag :type="getLogTagType(log.type)" size="small">
                  {{ log.type }}
                </el-tag>
              </div>
              <div class="log-message">
                <pre>{{ log.message }}</pre>
              </div>
            </div>
            
            <div v-if="messageLogs.length === 0" class="empty-logs">
              <el-empty description="暂无消息日志" />
            </div>
          </div>
        </div>
      </el-splitter-panel>
    </el-splitter>

  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, computed, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useTemplateStore } from '../../stores/templateStore'
import { serverStore } from '../../stores/serverStore'
import { invoke } from '@tauri-apps/api/core'
import netClientManager, { type ClientEventData } from '../../utils/NetClientManager'
import { Vue3JsonEditor } from 'vue3-json-editor'
import messageTransform from '../../utils/message_transform'
import { 
  Link, 
  CircleClose as Unlink,
  Edit, 
  Delete, 
  Promotion,
  Plus,
  Refresh 
} from '@element-plus/icons-vue'

// Props
interface Props {
  clientId: string
  serverInfo?: {
    id: number
    name: string
    type: string
    host: string
    port: number
    tcpPort?: number
    wsPort?: number
  }
}

const props = defineProps<Props>()
const emit = defineEmits(['update-status', 'rename'])

// 连接状态
const isConnected = ref(false)
const useWebSocket = ref(true)
const canSwitchProtocol = ref(true)
const currentHost = ref('')
const currentPort = ref<number>(0)
const autoReconnect = ref(false)
const reconnectInterval = ref(5)

// 消息相关
const selectedMessageType = ref('')
const messageContent = ref('')
const messageContentJson = ref<any>({})  // JSON 编辑器的数据
const isRpc = ref(false)  // RPC checkbox状态
const messageLogs = ref<Array<{
  type: 'sent' | 'received' | 'error' | 'info'
  message: string
  timestamp: number
}>>([])
const autoScroll = ref(true)
const logContainer = ref<HTMLElement>()

// 使用模板 store
const templateStore = useTemplateStore()

// 根据消息结构生成默认JSON
const generateDefaultJson = (messageType: string) => {
  // 从 templateStore 中查找消息定义
  const message = templateStore.getProtoMessage(messageType)
  
  if (!message || !message.fields) {
    return {}
  }
  
  const defaultJson: any = {}
  
  // 根据字段类型生成默认值
  message.fields.forEach((field: any) => {
    let defaultValue: any
    
    // 根据字段类型设置默认值
    switch (field.type) {
      case 'string':
        defaultValue = ''
        break
      case 'int32':
      case 'uint32':
      case 'int64':
      case 'uint64':
      case 'sint32':
      case 'sint64':
      case 'fixed32':
      case 'fixed64':
      case 'sfixed32':
      case 'sfixed64':
        defaultValue = 0
        break
      case 'float':
      case 'double':
        defaultValue = 0.0
        break
      case 'bool':
        defaultValue = false
        break
      case 'bytes':
        defaultValue = ''
        break
      default:
        // 可能是另一个消息类型或枚举
        defaultValue = {}
        break
    }
    
    // 如果是repeated字段，返回数组
    if (field.repeated) {
      defaultValue = []
    }
    
    defaultJson[field.name] = defaultValue
  })
  
  return defaultJson
}

// 标记是否正在加载模板，避免watcher覆盖
const isLoadingTemplate = ref(false)

// 监听消息类型选择变化
watch(selectedMessageType, (newType) => {
  // 如果正在加载模板，不自动填充
  if (isLoadingTemplate.value) {
    isLoadingTemplate.value = false
    return
  }
  
  if (newType) {
    // 自动填充JSON结构
    const defaultJson = generateDefaultJson(newType)
    messageContentJson.value = defaultJson
    
    console.log('自动填充消息结构:', newType, defaultJson)
  } else {
    messageContentJson.value = {}
  }
})

// 获取当前服务器类型（使用服务器所在的组名）
const currentServerType = computed(() => {
  // 需要从 serverStore 中根据服务器 ID 找到它所属的组
  const serverId = props.serverInfo?.id
  if (!serverId) return 'default'
  
  const serverGroups = serverStore.getServerGroups()
  for (const group of serverGroups) {
    const server = group.children.find(s => s.id === String(serverId))
    if (server) {
      // 返回组名（通常对应proto包名，如 session、chat 等）
      return group.name || group.id || 'default'
    }
  }
  return 'default'
})

// 获取当前服务器的模板列表
const customTemplates = computed(() => {
  return templateStore.getTemplatesByServerType(currentServerType.value)
})

// 消息类型定义（动态从proto文件加载）
const messageTypes = computed(() => {
  let protoMessages = templateStore.protoMessages
  
  console.log('=== messageTypes 计算开始 ===')
  console.log('原始 protoMessages 数量:', protoMessages.length)
  console.log('原始 protoMessages:', protoMessages)
  
  // 1. 首先根据rpc状态过滤消息
  if (isRpc.value) {
    // 勾选rpc时，只显示BRequest和BNotify（后端消息），且不限制服务器类型
    protoMessages = protoMessages.filter(msg => {
      const msgName = msg.name
      // 排除特定的RPC框架内部消息
      const rpcFrameworkMessages = [
        'RpcMessageFRequest',
        'RpcMessageFNotify', 
        'RpcForwardMessageBRequest',
        'RpcForwardMessageBNotify'
      ]
      if (rpcFrameworkMessages.includes(msgName)) {
        console.log(`  过滤掉(RPC框架消息): ${msgName}`)
        return false
      }
      // 检查是否包含 BRequest 或 BNotify 模式
      const matches = /B(?:Request|Notify)/i.test(msgName)
      if (!matches) {
        console.log(`  过滤掉(不是后端消息): ${msgName}`)
      }
      return matches
    })
    console.log('RPC模式 - 显示所有服务器的BRequest和BNotify消息（排除RPC框架消息）')
    // RPC模式下跳过服务器类型过滤
  } else {
    // 未勾选rpc时，显示当前服务器的所有Request和Notify类型的消息
    protoMessages = protoMessages.filter(msg => {
      const msgName = msg.name
      // 检查消息名是否包含Request或Notify
      // 例如：LoginFRequest, LoginBRequest, ChatFNotify, ChatBNotify等
      const matches = /Request|Notify/i.test(msgName)
      if (!matches) {
        console.log(`  过滤掉(不是Request/Notify): ${msgName}`)
      }
      return matches
    })
    console.log('步骤1后 - Request/Notify过滤后数量:', protoMessages.length)
    console.log('步骤1后 - 消息列表:', protoMessages.map(m => m.name))
    
    // 2. 非RPC模式下，根据当前服务器类型过滤消息
    // 服务器类型对应proto文件名（包名），例如：session.proto -> package session
    const serverType = currentServerType.value.toLowerCase()
    console.log('当前服务器类型:', serverType)
    console.log('服务器信息:', props.serverInfo)
    
    if (serverType && serverType !== 'default') {
      const beforeCount = protoMessages.length
      protoMessages = protoMessages.filter(msg => {
        const msgPackage = (msg.package || '').toLowerCase()
        // 服务器类型应该完全匹配包名
        // 例如：session服务器只显示 package 为 "session" 的消息
        const matches = msgPackage === serverType
        if (!matches) {
          console.log(`  过滤掉(包名不匹配): ${msg.name}, 包名: "${msgPackage}", 服务器类型: "${serverType}"`)
        }
        return matches
      })
      console.log(`步骤2后 - 服务器类型过滤(${serverType})后数量: ${beforeCount} -> ${protoMessages.length}`)
      console.log('步骤2后 - 保留的消息:', protoMessages.map(m => `${m.package}.${m.name}`))
    } else {
      console.log('服务器类型为default或空，跳过服务器类型过滤')
    }
  }
  
  console.log('最终过滤后的消息:', protoMessages.map(m => `${m.package}.${m.name}`))
  
  // 按包名分组
  const groupedMessages: { [key: string]: any[] } = {}
  
  protoMessages.forEach(msg => {
    const packageName = msg.package || '默认'
    if (!groupedMessages[packageName]) {
      groupedMessages[packageName] = []
    }
    
    groupedMessages[packageName].push({
      label: msg.name,  // 只显示消息名，不显示注释
      value: msg.package ? `${msg.package}.${msg.name}` : msg.name
    })
  })
  
  // 转换为el-select需要的格式
  return Object.entries(groupedMessages).map(([packageName, options]) => ({
    label: packageName,
    options
  }))
})

// 连接实例和状态
let reconnectTimer: number | null = null
let rustClientId: number | null = null  // Rust端客户端ID（所有连接都通过Rust处理）
let eventHandler: ((event: ClientEventData) => void) | null = null  // 事件处理器

// 切换连接状态
const toggleConnection = () => {
  if (isConnected.value) {
    disconnect()
  } else {
    connect()
  }
}

// 连接服务器（统一通过 Rust 后端处理）
const connect = async () => {
  if (!rustClientId) {
    ElMessage.error('客户端未初始化')
    return
  }
  
  // 根据协议类型设置端口
  if (useWebSocket.value) {
    currentHost.value = props.serverInfo?.host || '127.0.0.1'
    currentPort.value = props.serverInfo?.wsPort || props.serverInfo?.port || 3001
  } else {
    currentHost.value = props.serverInfo?.host || '127.0.0.1'
    currentPort.value = props.serverInfo?.tcpPort || props.serverInfo?.port || 3001
  }
  
  try {
    // 统一通过 Rust 端连接（TCP 或 WebSocket）
    await invoke('connect_client', {
      clientId: rustClientId
    })
    
    // 连接状态会通过事件通知，不在这里直接设置
    const protocol = useWebSocket.value ? 'WebSocket' : 'TCP'
    addLog('info', `正在连接到${protocol}服务器 ${currentHost.value}:${currentPort.value}...`)
    
  } catch (error) {
    const protocol = useWebSocket.value ? 'WebSocket' : 'TCP'
    addLog('error', `${protocol}连接失败: ${error}`)
    ElMessage.error(`${protocol}连接失败: ${error}`)
    isConnected.value = false
    emit('update-status', props.clientId, false)
  }
}

// 反序列化protobuf消息（使用messageTransform工具）
const deserializeMessage = async (msgId: number, bytes: Uint8Array): Promise<any> => {
  return await messageTransform.decodeMessage(msgId, bytes)
}

// 处理来自 Rust 的事件
const handleClientEvent = async (event: ClientEventData) => {
  console.log('Client received event:', event)
  
  switch (event.eventType) {
    case 'connected':
      isConnected.value = true
      emit('update-status', props.clientId, true)
      addLog('info', `已连接到服务器`)
      ElMessage.success('连接成功')
      break
      
    case 'disconnected':
      isConnected.value = false
      emit('update-status', props.clientId, false)
      addLog('info', '连接已断开')
      
      // 自动重连
      if (autoReconnect.value && !reconnectTimer) {
        startReconnect()
      }
      break
      
    case 'message':
      // 处理二进制消息
      if (event.msgId !== undefined && event.bytes) {
        const bytes = new Uint8Array(event.bytes)
        const decodedMessage = await deserializeMessage(event.msgId, bytes)
        
        if (decodedMessage) {
          // 使用当前时间戳
          addLog('received', `${decodedMessage.type} (ID:${event.msgId}, Size:${event.byteLen}): ${JSON.stringify(decodedMessage.data, null, 2)}`)
        } else {
          addLog('received', `未知消息 (ID:${event.msgId}, Size:${event.byteLen})`)
        }
      } else if (event.data) {
        // 处理文本消息（兼容旧格式）
        addLog('received', event.data)
      }
      break
      
    case 'error':
      addLog('error', event.error || '未知错误')
      ElMessage.error(event.error || '连接错误')
      break
  }
}

// 注册事件处理器
const registerEventHandler = () => {
  if (!rustClientId) return
  
  // 创建事件处理器
  eventHandler = handleClientEvent
  
  // 注册到 NetClientManager
  netClientManager.addListener(rustClientId, eventHandler)
  console.log(`Registered event handler for client ${rustClientId}`)
}

// 注销事件处理器
const unregisterEventHandler = () => {
  if (!rustClientId || !eventHandler) return
  
  netClientManager.removeListener(rustClientId, eventHandler)
  eventHandler = null
  console.log(`Unregistered event handler for client ${rustClientId}`)
}

// 断开连接（统一通过 Rust 后端处理）
const disconnect = async () => {
  if (reconnectTimer) {
    clearTimeout(reconnectTimer)
    reconnectTimer = null
  }
  
  // 统一通过 Rust 端断开连接
  if (rustClientId) {
    try {
      await invoke('disconnect_client', {
        clientId: rustClientId
      })
    } catch (error) {
      console.error('断开连接失败:', error)
    }
  }
  
  isConnected.value = false
  emit('update-status', props.clientId, false)
  addLog('info', '已主动断开连接')
}

// 自动重连
const startReconnect = () => {
  reconnectTimer = window.setTimeout(() => {
    reconnectTimer = null
    if (!isConnected.value && autoReconnect.value) {
      addLog('info', '正在尝试重新连接...')
      connect()
    }
  }, reconnectInterval.value * 1000)
}

// 发送消息（统一通过 Rust 后端处理）
const sendMessage = async () => {
  if (!isConnected.value) {
    ElMessage.warning('请先连接服务器')
    return
  }
  
  if (!rustClientId) {
    ElMessage.error('客户端未初始化')
    return
  }
  
  try {
    let encodeResult
    
    if (isRpc.value) {
      // RPC模式：根据消息类型创建对应的RPC包装消息
      // 从消息类型中提取包名作为server_type (例如: session.LoginFRequest -> session)
      const serverType = selectedMessageType.value.includes('.') 
        ? selectedMessageType.value.split('.')[0] 
        : 'default'
        
      if (messageTransform.isRequestMessage(selectedMessageType.value)) {
        // 创建RPC请求消息
        encodeResult = await messageTransform.encodeRpcRequest(
          selectedMessageType.value,
          messageContentJson.value,
          serverType
        )
      } else if (messageTransform.isNotifyMessage(selectedMessageType.value)) {
        // 创建RPC通知消息
        encodeResult = await messageTransform.encodeRpcNotify(
          selectedMessageType.value,
          messageContentJson.value,
          serverType
        )
      } else {
        ElMessage.error(`消息类型 ${selectedMessageType.value} 不支持RPC模式（必须是Request或Notify类型）`)
        return
      }
    } else {
      // 普通模式：直接编码原始消息
      encodeResult = await messageTransform.encodeMessage(selectedMessageType.value, messageContentJson.value)
    }
    
    const { msgId, byteArray, messageType, messageData } = encodeResult
    const byteLen = byteArray.length
    
    console.log('发送消息 - 详细信息:', {
      isRpc: isRpc.value,
      msgId,
      byteLen,
      messageType,
      originalMessageType: selectedMessageType.value,
      data: messageContentJson.value,
      encodedData: messageData
    })
    
    // 先记录日志（使用当前时间戳）
    const logPrefix = isRpc.value ? `[RPC] ${selectedMessageType.value}` : messageType
    const logData = isRpc.value ? messageContentJson.value : messageContentJson.value
    addLog('sent', `${logPrefix} (ID:${msgId}, Size:${byteLen}): ${JSON.stringify(logData, null, 2)}`)
    
    // 然后调用Rust发送消息（传递msgId、byteLen和字节数组）
    await invoke('send_client_message', {
      clientId: rustClientId,
      msgId: msgId,
      byteLen: byteLen,
      data: byteArray
    })
    
    ElMessage.success('消息已发送')
    
  } catch (error) {
    addLog('error', `发送失败: ${error}`)
    ElMessage.error(`发送失败: ${error}`)
  }
}

// 清空消息
const clearMessage = () => {
  messageContent.value = ''
  messageContentJson.value = {}  // 清空 JSON 编辑器
  selectedMessageType.value = ''
}


// 添加到模板
const addToTemplate = async () => {
  if (!selectedMessageType.value || !messageContentJson.value || Object.keys(messageContentJson.value).length === 0) {
    ElMessage.warning('请先选择消息类型并填写消息内容')
    return
  }
  
  try {
    const { value } = await ElMessageBox.prompt('请输入模板名称', '添加到模板', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      inputPattern: /^.{1,20}$/,
      inputErrorMessage: '模板名称长度应为 1-20 个字符',
      inputValue: `${selectedMessageType.value}_模板`
    })
    
    if (value) {
      // 检查是否已存在同名模板
      const existingTemplate = customTemplates.value.find(t => t.name === value)
      if (existingTemplate) {
        // 如果存在，询问是否覆盖
        await ElMessageBox.confirm(
          `模板 "${value}" 已存在，是否覆盖？`,
          '提示',
          {
            confirmButtonText: '覆盖',
            cancelButtonText: '取消',
            type: 'warning',
          }
        )
      }
      
      // 使用 store 添加模板（保存 JSON 格式的内容）
      await templateStore.addTemplate(currentServerType.value, {
        name: value,
        type: selectedMessageType.value,
        content: JSON.stringify(messageContentJson.value, null, 2)  // 格式化 JSON
      })
      
      ElMessage.success('模板已保存')
    }
  } catch (error) {
    // 用户取消或错误
    if (error !== 'cancel') {
      console.error('添加模板失败:', error)
    }
  }
}

// 加载自定义模板
const loadCustomTemplate = (template: any) => {
  // 标记正在加载模板
  isLoadingTemplate.value = true
  
  // 先设置JSON内容
  try {
    messageContentJson.value = JSON.parse(template.content)
  } catch {
    messageContentJson.value = { message: template.content }
  }
  
  // 然后设置消息类型（watcher会检查isLoadingTemplate标记）
  selectedMessageType.value = template.type
  messageContent.value = template.content
  
  ElMessage.success(`已加载模板: ${template.name}`)
}

// 删除模板
const deleteTemplate = async (templateName: string) => {
  try {
    await ElMessageBox.confirm(
      `确定要删除模板 "${templateName}" 吗？`,
      '删除模板',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning',
      }
    )
    
    await templateStore.deleteTemplate(currentServerType.value, templateName)
  } catch {
    // 用户取消
  }
}

// 刷新模板列表
const refreshTemplates = async () => {
  await templateStore.loadTemplates(currentServerType.value)
  ElMessage.success('模板列表已刷新')
}

// 协议切换处理
const onProtocolChange = async () => {
  // 如果正在连接中，不允许切换
  if (isConnected.value) {
    // 恢复原来的值
    useWebSocket.value = !useWebSocket.value
    ElMessage.warning('请先断开连接再切换协议')
    return
  }
  
  // 切换Rust端客户端类型
  if (rustClientId) {
    try {
      const protocol = useWebSocket.value ? 'websocket' : 'tcp'
      await invoke('switch_client_type', {
        clientId: rustClientId,
        clientType: protocol
      })
      
      // 更新端口
      updatePortByProtocol()
      
      // 添加切换日志
      const protocolName = useWebSocket.value ? 'WebSocket' : 'TCP'
      addLog('info', `已切换到 ${protocolName} 协议，端口: ${currentPort.value}`)
      
    } catch (error) {
      console.error('切换客户端类型失败:', error)
      // 恢复原来的值
      useWebSocket.value = !useWebSocket.value
      ElMessage.error(`切换协议失败: ${error}`)
      return
    }
  } else {
    // 没有 Rust 客户端时也更新端口
    updatePortByProtocol()
    const protocolName = useWebSocket.value ? 'WebSocket' : 'TCP'
    addLog('info', `已切换到 ${protocolName} 协议，端口: ${currentPort.value}`)
  }
}

// 根据协议类型更新端口
const updatePortByProtocol = () => {
  if (!props.serverInfo) return
  
  if (useWebSocket.value) {
    // 使用 WebSocket
    if (props.serverInfo.wsPort) {
      currentPort.value = props.serverInfo.wsPort
    } else {
      currentPort.value = props.serverInfo.port || 3001
    }
  } else {
    // 使用 TCP
    if (props.serverInfo.tcpPort) {
      currentPort.value = props.serverInfo.tcpPort
    } else {
      currentPort.value = props.serverInfo.port || 3001
    }
  }
}

// 添加日志
const addLog = (type: 'sent' | 'received' | 'error' | 'info', message: string, timestamp?: number) => {
  messageLogs.value.push({
    type,
    message,
    timestamp: timestamp || Date.now()
  })
  
  // 限制日志数量
  if (messageLogs.value.length > 1000) {
    messageLogs.value.shift()
  }
  
  // 自动滚动到底部
  if (autoScroll.value) {
    nextTick(() => {
      if (logContainer.value) {
        logContainer.value.scrollTop = logContainer.value.scrollHeight
      }
    })
  }
}

// 清空日志
const clearLogs = () => {
  messageLogs.value = []
  ElMessage.success('日志已清空')
}

// 格式化时间
const formatTime = (timestamp: number) => {
  const date = new Date(timestamp)
  const time = date.toLocaleTimeString('zh-CN', {
    hour12: false,
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit'
  })
  const ms = date.getMilliseconds().toString().padStart(3, '0')
  return `${time}.${ms}`
}

// 获取日志标签类型
const getLogTagType = (type: string) => {
  switch (type) {
    case 'sent': return 'primary'
    case 'received': return 'success'
    case 'error': return 'danger'
    case 'info': return 'info'
    default: return 'info'
  }
}

// 生命周期
onMounted(async () => {
  // 初始化连接设置
  if (props.serverInfo) {
    currentHost.value = props.serverInfo.host || '127.0.0.1'
    
    // 判断支持的协议类型
    const hasTCP = !!props.serverInfo.tcpPort
    const hasWebSocket = !!props.serverInfo.wsPort
    
    // 如果只支持一种协议，禁用切换
    if (hasTCP && !hasWebSocket) {
      // 只支持 TCP
      useWebSocket.value = false
      canSwitchProtocol.value = false
      currentPort.value = props.serverInfo.tcpPort!
    } else if (!hasTCP && hasWebSocket) {
      // 只支持 WebSocket
      useWebSocket.value = true
      canSwitchProtocol.value = false
      currentPort.value = props.serverInfo.wsPort!
    } else if (hasTCP && hasWebSocket) {
      // 两种都支持，默认使用 WebSocket
      useWebSocket.value = true
      canSwitchProtocol.value = true
      currentPort.value = props.serverInfo.wsPort!
    } else {
      // 都没有明确指定，使用默认端口
      useWebSocket.value = true
      canSwitchProtocol.value = true
      currentPort.value = props.serverInfo.port || 3001
    }
    
    // 创建Rust端客户端（一对一映射）
    try {
      rustClientId = await invoke('create_client', {
        serverId: String(props.serverInfo.id),
        serverName: props.serverInfo.name,
        host: props.serverInfo.host,
        tcpPort: props.serverInfo.tcpPort,
        wsPort: props.serverInfo.wsPort
      }) as number
      
      addLog('info', `客户端已创建，ID: ${rustClientId}`)
      
      // 注册事件处理器（生命周期内一直保持）
      registerEventHandler()
    } catch (error) {
      console.error('创建客户端失败:', error)
      ElMessage.error(`创建客户端失败: ${error}`)
    }
    
    // 从 store 加载消息模板
    await templateStore.loadTemplates(currentServerType.value)
    
    // 加载proto文件中的协议定义
    await templateStore.loadProtoMessages()
    
    // 初始化完成，显示当前配置
    const protocol = useWebSocket.value ? 'WebSocket' : 'TCP'
    addLog('info', `客户端已初始化，当前协议: ${protocol}，服务器地址: ${currentHost.value}:${currentPort.value}`)
  } else {
    // 没有服务器信息时的默认值
    currentHost.value = '127.0.0.1'
    currentPort.value = 3001
    useWebSocket.value = true
    canSwitchProtocol.value = true
    
    // 从 store 加载默认模板
    await templateStore.loadTemplates('default')
  }
})

onUnmounted(async () => {
  // 清理资源
  await disconnect()
  
  // 注销事件处理器
  unregisterEventHandler()
  
  // 删除Rust端客户端（一对一映射）
  if (rustClientId) {
    try {
      await invoke('delete_client', {
        clientId: rustClientId
      })
      console.log(`客户端 ${rustClientId} 已删除`)
    } catch (error) {
      console.error('删除客户端失败:', error)
    }
  }
})
</script>

<style scoped>
.client-container {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.client-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px;
  background: #f5f7fa;
  border-bottom: 1px solid #e4e7ed;
}

.header-left {
  display: flex;
  gap: 10px;
}

.header-right {
  display: flex;
  gap: 10px;
  align-items: center;
}

.client-content {
  flex: 1;
  min-height: 0;
}

/* 消息面板样式 - 使用绝对定位自适应布局 */
.message-panel {
  height: 100%;
  overflow: hidden;
  position: relative;
}

.message-panel-inner {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  padding: 15px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
}

.message-panel h4 {
  margin: 0 0 15px 0;
  color: #303133;
  flex-shrink: 0;
}

.message-type-container {
  display: flex;
  gap: 10px;
  margin-bottom: 15px;
  flex-shrink: 0;
}

.message-type-select {
  flex: 1;
}

.message-editor {
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
}

.message-editor :deep(.jsoneditor) {
  border: 1px solid #dcdfe6;
  border-radius: 4px;
}

.message-editor :deep(.jsoneditor-menu) {
  background-color: #f5f7fa;
  border-bottom: 1px solid #dcdfe6;
}

.message-input {
  font-family: 'Consolas', 'Monaco', monospace;
}

.send-actions {
  display: flex;
  gap: 10px;
  margin-top: 10px;
}

.template-section {
  margin-top: 15px;
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.template-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
  font-size: 13px;
  color: #606266;
  font-weight: 500;
  flex-shrink: 0;
}

.custom-templates {
  margin-top: 10px;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  border: 1px solid #e4e7ed;
  border-radius: 4px;
  padding: 10px;
  background: #fafafa;
}

.empty-templates {
  margin-top: 10px;
  padding: 20px;
  border: 1px solid #e4e7ed;
  border-radius: 4px;
  background: #fafafa;
  text-align: center;
}

.template-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.template-item {
  display: flex;
  align-items: center;
  gap: 2px;
}

.template-btn {
  max-width: 150px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 自定义滚动条样式 */
.message-panel-inner::-webkit-scrollbar,
.custom-templates::-webkit-scrollbar,
.log-content::-webkit-scrollbar {
  width: 8px;
}

.message-panel-inner::-webkit-scrollbar-track,
.custom-templates::-webkit-scrollbar-track,
.log-content::-webkit-scrollbar-track {
  background: #f1f1f1;
  border-radius: 4px;
}

.message-panel-inner::-webkit-scrollbar-thumb,
.custom-templates::-webkit-scrollbar-thumb,
.log-content::-webkit-scrollbar-thumb {
  background: #c0c4cc;
  border-radius: 4px;
}

.message-panel-inner::-webkit-scrollbar-thumb:hover,
.custom-templates::-webkit-scrollbar-thumb:hover,
.log-content::-webkit-scrollbar-thumb:hover {
  background: #909399;
}

/* 日志面板样式 - 使用绝对定位自适应布局 */
.log-panel {
  height: 100%;
  overflow: hidden;
  position: relative;
  background: #fafafa;
}

.log-header {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 15px;
  background: white;
  border-bottom: 1px solid #e4e7ed;
  z-index: 1;
}

.log-header h4 {
  margin: 0;
  color: #303133;
}

.log-actions {
  display: flex;
  gap: 10px;
  align-items: center;
}

.log-content {
  position: absolute;
  top: 55px; /* header高度 + padding */
  left: 0;
  right: 0;
  bottom: 0;
  padding: 10px;
  overflow-y: auto;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 12px;
}

.log-item {
  margin-bottom: 10px;
  padding: 8px;
  background: white;
  border-radius: 4px;
  border-left: 3px solid #e4e7ed;
}

.log-item.sent {
  border-left-color: #409eff;
}

.log-item.received {
  border-left-color: #67c23a;
}

.log-item.error {
  border-left-color: #f56c6c;
}

.log-item.info {
  border-left-color: #909399;
}

.log-time {
  color: #909399;
  font-size: 11px;
  margin-bottom: 5px;
}

.log-type {
  margin-bottom: 5px;
}

.log-message pre {
  margin: 0;
  white-space: pre-wrap;
  word-wrap: break-word;
  color: #303133;
  line-height: 1.5;
}

.empty-logs {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* Element Plus 样式覆盖 */
:deep(.el-textarea__inner) {
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 13px;
}
</style>