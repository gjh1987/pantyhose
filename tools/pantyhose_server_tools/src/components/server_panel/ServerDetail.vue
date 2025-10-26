<template>
  <div class="server-info-wrapper">
    <div class="server-detail-content">
      <div class="server-info-container" :class="{ 'collapsed': serverInfoCollapsed }">
        <div class="detail-header">
          <div class="header-left">
            <el-button 
              @click="serverInfoCollapsed = !serverInfoCollapsed" 
              :icon="serverInfoCollapsed ? ArrowDown : ArrowUp"
              type="text"
              size="small"
              class="collapse-btn"
              round
            >
              {{ serverInfoCollapsed ? '展开' : '折叠' }}
            </el-button>
            <h3>{{ server.name || `服务器-${server.id}` }}</h3>
            <el-tag 
              :type="server.isRunning ? 'success' : 'danger'"
              size="large"
              effect="dark"
            >
              {{ server.isRunning ? '运行中' : '已停止' }}
            </el-tag>
          </div>
        </div>
      
        <el-descriptions :column="1" border class="server-info">
          <el-descriptions-item label="服务器ID">{{ server.id }}</el-descriptions-item>
          <el-descriptions-item label="后端TCP端口" v-if="server.back_tcp_port">
            <el-tag type="info">{{ server.back_tcp_port }}</el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="前端TCP端口" v-if="server.front_tcp_port">
            <el-tag type="primary">{{ server.front_tcp_port }}</el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="前端WebSocket端口" v-if="server.front_ws_port">
            <el-tag type="warning">{{ server.front_ws_port }}</el-tag>
          </el-descriptions-item>
        </el-descriptions>
      
        <div class="server-actions">
          <el-button 
            :type="server.isRunning ? 'danger' : 'success'"
            :icon="server.isRunning ? VideoPause : VideoPlay"
            @click="toggleServerStatus"
            size="large"
          >
            {{ server.isRunning ? '停止服务器' : '启动服务器' }}
          </el-button>
          
          <el-button 
            type="info" 
            plain 
            size="large"
            @click="restartServer"
            :disabled="!server.isRunning"
          >
            重启服务器
          </el-button>
        </div>
      </div>
      
      <!-- 服务器详细面板选项卡 -->
      <div class="server-tabs-section">
        <el-tabs v-model="activeTab" type="card" class="server-tabs">
          <el-tab-pane label="服务器日志" name="logs">
            <ServerLogs 
              :selected-server="server"
              @logs-refreshed="handleLogsRefreshed"
            />
          </el-tab-pane>
          <el-tab-pane label="GM面板" name="gm">
            <GM 
              :selected-server="server"
              @command-executed="handleGMCommandExecuted"
            />
          </el-tab-pane>
          <el-tab-pane 
            v-if="server.front_tcp_port || server.front_ws_port"
            label="测试客户端" 
            name="clients"
          >
            <ClientList 
              :server-info="{
                id: Number(server.id),
                name: server.name || `服务器-${server.id}`,
                type: server.type || 'unknown',
                host: '127.0.0.1',
                port: server.front_tcp_port || server.front_ws_port || 3001,
                tcpPort: server.front_tcp_port,
                wsPort: server.front_ws_port
              }"
            />
          </el-tab-pane>
        </el-tabs>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { VideoPlay, VideoPause, ArrowDown, ArrowUp } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { serverStore, type ServerInfo } from '../../stores/serverStore'
import ServerLogs from './ServerLogs.vue'
import GM from './GM.vue'
import ClientList from './ClientList.vue'

// Props
const props = defineProps<{
  server: ServerInfo
  clearLogsOnStart?: boolean
}>()

// Emits
const emit = defineEmits<{
  'server-status-changed': [serverId: string, isRunning: boolean]
  'close-tab': []
}>()

// 响应式数据
const serverInfoCollapsed = ref(false)
const activeTab = ref('logs')

// 定期更新服务器状态
let statusUpdateInterval: number | null = null

// 更新服务器状态
const updateServerStatus = async () => {
  try {
    const isRunning = await invoke('get_server_status', { serverId: props.server.id })
    if (props.server.isRunning !== isRunning) {
      serverStore.updateServerStatus(props.server.id, isRunning as boolean)
      emit('server-status-changed', props.server.id, isRunning as boolean)
    }
  } catch (error) {
    console.error('获取服务器状态失败:', error)
  }
}

// 启动/停止服务器
const toggleServerStatus = async () => {
  if (props.server.isRunning) {
    await stopServer()
  } else {
    await startServer()
  }
}

// 启动服务器
const startServer = async () => {
  try {
    // 如果设置了启动前清理日志
    if (props.clearLogsOnStart) {
      try {
        await invoke('clear_server_logs', { serverId: props.server.id })
        console.log(`清理服务器 ${props.server.id} 的日志文件`)
      } catch (error) {
        console.error('清理日志失败:', error)
      }
    }
    
    await invoke('start_server', { serverId: props.server.id })
    
    serverStore.updateServerStatus(props.server.id, true)
    emit('server-status-changed', props.server.id, true)
    
    ElMessage.success(`服务器 ${props.server.name || props.server.id} 已启动`)
  } catch (error) {
    ElMessage.error(`启动服务器失败: ${error}`)
  }
}

// 停止服务器
const stopServer = async () => {
  try {
    await ElMessageBox.confirm(
      `确定要停止服务器 ${props.server.name || props.server.id} 吗？`,
      '停止服务器',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning',
      }
    )
    
    await invoke('stop_server', { serverId: props.server.id })
    
    serverStore.updateServerStatus(props.server.id, false)
    emit('server-status-changed', props.server.id, false)
    
    ElMessage.success(`服务器 ${props.server.name || props.server.id} 已停止`)
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error(`停止服务器失败: ${error}`)
    }
  }
}

// 重启服务器
const restartServer = async () => {
  try {
    await ElMessageBox.confirm(
      `确定要重启服务器 ${props.server.name || props.server.id} 吗？`,
      '重启服务器',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning',
      }
    )
    
    await invoke('restart_server', { serverId: props.server.id })
    ElMessage.success(`服务器 ${props.server.name || props.server.id} 正在重启`)
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error(`重启服务器失败: ${error}`)
    }
  }
}

// 处理日志刷新事件
const handleLogsRefreshed = () => {
  // 可以在这里处理日志刷新后的逻辑
}

// 处理GM命令执行事件
const handleGMCommandExecuted = () => {
  // 可以在这里处理GM命令执行后的逻辑
}

// 组件挂载时启动定期更新
onMounted(() => {
  // 立即更新一次状态
  updateServerStatus()
  
  // 每5秒更新一次状态
  statusUpdateInterval = window.setInterval(() => {
    updateServerStatus()
  }, 5000)
})

// 组件卸载时清理定时器
onUnmounted(() => {
  if (statusUpdateInterval) {
    clearInterval(statusUpdateInterval)
  }
})
</script>

<style scoped>
.server-info-wrapper {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.server-detail-content {
  height: 100%;
  display: flex;
  flex-direction: column;
  padding: 20px;
  background-color: #f5f7fa;
}

.server-info-container {
  background: white;
  border-radius: 8px;
  padding: 20px;
  margin-bottom: 20px;
  box-shadow: 0 2px 4px rgba(0,0,0,0.1);
  transition: all 0.3s ease;
}

.server-info-container.collapsed {
  padding: 15px 20px;
}

.server-info-container.collapsed .server-info,
.server-info-container.collapsed .server-actions {
  display: none;
}

.detail-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.server-info-container.collapsed .detail-header {
  margin-bottom: 0;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 15px;
}

.collapse-btn {
  color: #606266;
}

.collapse-btn:hover {
  color: #409eff;
}

.header-left h3 {
  margin: 0;
  font-size: 20px;
  color: #303133;
}

.server-info {
  margin-bottom: 20px;
}

.server-actions {
  display: flex;
  gap: 10px;
}

.server-tabs-section {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: white;
  border-radius: 8px;
  box-shadow: 0 2px 4px rgba(0,0,0,0.1);
  overflow: hidden;
}

.server-tabs {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.server-tabs :deep(.el-tabs__header) {
  margin: 0;
  padding: 10px 10px 0;
  background-color: #f5f7fa;
}

.server-tabs :deep(.el-tabs__content) {
  flex: 1;
  padding: 20px;
  overflow: auto;
}

.server-tabs :deep(.el-tab-pane) {
  height: 100%;
}
</style>