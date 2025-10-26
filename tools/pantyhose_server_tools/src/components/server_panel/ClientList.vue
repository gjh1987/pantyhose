<template>
  <div class="client-list-container">
    <!-- 顶部工具栏 -->
    <div class="client-toolbar">
      <div class="toolbar-left">
        <el-button type="primary" size="small" @click="addNewClient" :icon="Plus">
          新建客户端
        </el-button>
        <el-button size="small" @click="refreshClients" :icon="Refresh">
          刷新
        </el-button>
        <el-tag type="info" style="margin-left: 10px;">
          客户端总数: {{ clients.length }}
        </el-tag>
        <el-tag type="success" style="margin-left: 10px;">
          已连接: {{ connectedCount }}
        </el-tag>
      </div>
      <div class="toolbar-right">
        <!-- 可以放其他内容，或者留空 -->
      </div>
    </div>

    <!-- 空状态 -->
    <div v-if="clients.length === 0" class="empty-state">
      <el-empty description="暂无客户端连接">
        <el-button type="primary" @click="addNewClient">
          创建测试客户端
        </el-button>
      </el-empty>
    </div>
    
    <!-- Tabs 标签页 -->
    <el-tabs 
      v-else
      v-model="activeTab" 
      type="card" 
      closable 
      @tab-remove="removeClient"
      class="client-tabs"
    >
      <el-tab-pane 
        v-for="client in clients" 
        :key="client.id"
        :label="client.name"
        :name="client.id"
      >
        <!-- 标签页标题自定义 -->
        <template #label>
          <span class="tab-label">
            <el-icon v-if="client.connected" class="status-icon connected">
              <CircleCheckFilled />
            </el-icon>
            <el-icon v-else class="status-icon disconnected">
              <CircleCloseFilled />
            </el-icon>
            {{ client.name }}
          </span>
        </template>
        
        <!-- 客户端内容组件 -->
        <Client 
          :client-id="client.id"
          :server-info="serverInfo"
          @update-status="updateClientStatus"
          @rename="renameClient"
        />
      </el-tab-pane>

    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Refresh, CircleCheckFilled, CircleCloseFilled } from '@element-plus/icons-vue'
import Client from './Client.vue'

// Props
interface Props {
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

// 客户端数据结构
interface ClientData {
  id: string
  name: string
  connected: boolean
  createTime: number
  lastActiveTime: number
}

// 响应式数据
const clients = ref<ClientData[]>([])
const activeTab = ref<string>('')
let clientCounter = 0

// 计算属性
const connectedCount = computed(() => {
  return clients.value.filter(c => c.connected).length
})

// 添加新客户端
const addNewClient = () => {
  clientCounter++
  const newClient: ClientData = {
    id: `client_${Date.now()}_${clientCounter}`,
    name: `客户端 ${clientCounter}`,
    connected: false,
    createTime: Date.now(),
    lastActiveTime: Date.now()
  }
  
  clients.value.push(newClient)
  activeTab.value = newClient.id
  
  ElMessage.success(`已创建客户端: ${newClient.name}`)
}

// 移除客户端
const removeClient = async (clientId: string) => {
  const client = clients.value.find(c => c.id === clientId)
  if (!client) return
  
  try {
    await ElMessageBox.confirm(
      `确定要关闭客户端 "${client.name}" 吗？`,
      '关闭客户端',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning',
      }
    )
    
    const index = clients.value.findIndex(c => c.id === clientId)
    if (index > -1) {
      clients.value.splice(index, 1)
      
      // 如果关闭的是当前标签，切换到其他标签
      if (activeTab.value === clientId && clients.value.length > 0) {
        activeTab.value = clients.value[0].id
      } else if (clients.value.length === 0) {
        activeTab.value = ''
      }
      
      ElMessage.success('客户端已关闭')
    }
  } catch {
    // 用户取消
  }
}

// 刷新客户端列表
const refreshClients = () => {
  // 这里可以添加刷新逻辑，比如重新获取客户端状态
  ElMessage.info('正在刷新客户端列表...')
  
  // 模拟刷新
  clients.value.forEach(client => {
    client.lastActiveTime = Date.now()
  })
}

// 更新客户端状态
const updateClientStatus = (clientId: string, connected: boolean) => {
  const client = clients.value.find(c => c.id === clientId)
  if (client) {
    client.connected = connected
    client.lastActiveTime = Date.now()
  }
}

// 重命名客户端
const renameClient = async (clientId: string) => {
  const client = clients.value.find(c => c.id === clientId)
  if (!client) return
  
  try {
    const { value } = await ElMessageBox.prompt('请输入新的客户端名称', '重命名客户端', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      inputValue: client.name,
      inputPattern: /^.{1,20}$/,
      inputErrorMessage: '客户端名称长度应为 1-20 个字符'
    })
    
    if (value) {
      client.name = value
      ElMessage.success('客户端已重命名')
    }
  } catch {
    // 用户取消
  }
}

// 生命周期
onMounted(() => {
  // 如果有服务器信息，可以自动创建一个客户端
  if (props.serverInfo) {
    console.log('Server info:', props.serverInfo)
  }
})

onUnmounted(() => {
  // 清理资源
  clients.value = []
})
</script>

<style scoped>
.client-list-container {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.client-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px;
  background: #f5f7fa;
  border-radius: 4px;
}

.toolbar-left {
  display: flex;
  gap: 10px;
}

.toolbar-right {
  display: flex;
  align-items: center;
}

.client-tabs {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.client-tabs :deep(.el-tabs__content) {
  flex: 1;
  padding: 0;
  overflow: hidden;
}

.client-tabs :deep(.el-tab-pane) {
  height: 100%;
}

.tab-label {
  display: flex;
  align-items: center;
  gap: 5px;
}

.status-icon {
  font-size: 14px;
}

.status-icon.connected {
  color: #67c23a;
}

.status-icon.disconnected {
  color: #909399;
}

.empty-state {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>