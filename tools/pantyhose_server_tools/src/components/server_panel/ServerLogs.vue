<template>
  <!-- 服务器日志区域 -->
  <div class="server-logs-section">
    <div class="logs-header">
      <div class="logs-header-left">
        <h4>服务器日志</h4>
        <el-select
          v-model="logFilters"
          multiple
          placeholder="筛选日志"
          size="small"
          style="width: 200px; margin-left: 12px"
          :teleported="false"
          :disabled="showAllLogs"
        >
          <el-option
            v-for="item in logFilterOptions"
            :key="item.value"
            :label="item.label"
            :value="item.value"
          />
        </el-select>
        <el-checkbox
          v-model="showAllLogs"
          size="small"
          style="margin-left: 12px"
        >
          ALL
        </el-checkbox>
      </div>
      <div class="logs-actions">
        <el-button @click="refreshLogs" type="primary" size="small">刷新</el-button>
        <el-button @click="clearLogs" type="danger" size="small">清空</el-button>
      </div>
    </div>
    
    <div class="logs-content">
      <div class="log-lines">
        <div 
          v-for="(log, index) in filteredLogs" 
          :key="index" 
          class="log-line"
          :class="{
            'log-error': log.includes('[ERROR]'),
            'log-system': log.includes('[SYSTEM]')
          }"
          v-html="convertAnsiToHtml(log)"
        >
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { ElMessage } from 'element-plus';
import { serverStore, type ServerInfo } from '../../stores/serverStore';
import AnsiToHtml from 'ansi-to-html';

// Props
const props = defineProps<{
  selectedServer: ServerInfo | null;
}>();

// Emits
const emit = defineEmits<{
  logsRefreshed: [];
}>();

// 创建 ANSI 到 HTML 转换器
const ansiConverter = new AnsiToHtml({
  fg: '#FFF',
  bg: '#000',
  newline: true,
  escapeXML: true,
  stream: false
});

// 转换 ANSI 颜色代码为 HTML
const convertAnsiToHtml = (text: string): string => {
  return ansiConverter.toHtml(text);
};

// 日志筛选相关
const logFilters = ref<string[]>([]);
const showAllLogs = ref(true); // 默认显示全部日志
const logFilterOptions = [
  { label: 'DEBUG', value: 'DEBUG' },
  { label: 'INFO', value: 'INFO' },
  { label: 'NET', value: 'NET' },
  { label: 'WARN', value: 'WARN' },
  { label: 'ERROR', value: 'ERROR' },
];

// 当前选中服务器的日志（从 store 获取）
const currentServerLogs = computed(() => {
  if (!props.selectedServer) return [];
  return serverStore.getServerLogs(props.selectedServer.id);
});

// 根据筛选条件过滤日志
const filteredLogs = computed(() => {
  if (!currentServerLogs.value || currentServerLogs.value.length === 0) {
    return [];
  }
  
  // 如果勾选了ALL，显示所有日志
  if (showAllLogs.value) {
    return currentServerLogs.value;
  }
  
  // 如果没有选择任何筛选条件，不显示日志
  if (logFilters.value.length === 0) {
    return [];
  }
  
  // 根据筛选条件过滤日志
  return currentServerLogs.value.filter(log => {
    const logUpper = log.toUpperCase();
    
    // 检查每个筛选条件
    for (const filter of logFilters.value) {
      switch(filter) {
        case 'DEBUG':
          if (logUpper.includes('[DEBUG]') || logUpper.includes('DEBUG')) {
            return true;
          }
          break;
        case 'INFO':
          if (logUpper.includes('[INFO]') || logUpper.includes('INFO')) {
            return true;
          }
          break;
        case 'NET':
          if (logUpper.includes('[NET]') || logUpper.includes('NET') || 
              logUpper.includes('NETWORK') || logUpper.includes('TCP') || 
              logUpper.includes('SOCKET') || logUpper.includes('连接') ||
              logUpper.includes('网络')) {
            return true;
          }
          break;
        case 'WARN':
          if (logUpper.includes('[WARN]') || logUpper.includes('WARN') || 
              logUpper.includes('WARNING')) {
            return true;
          }
          break;
        case 'ERROR':
          if (logUpper.includes('[ERROR]') || logUpper.includes('ERROR') || 
              logUpper.includes('FAIL') || logUpper.includes('失败')) {
            return true;
          }
          break;
      }
    }
    
    return false;
  });
});

// 加载服务器日志
const loadServerLogs = async (server: ServerInfo) => {
  try {
    const logs = await invoke('get_server_logs', { 
      serverId: server.id
    }) as string[];
    
    serverStore.setServerLogs(server.id, logs);
  } catch (error) {
    console.error('获取服务器日志失败:', error);
    serverStore.setServerLogs(server.id, []);
  }
};

// 刷新当前服务器日志
const refreshLogs = async () => {
  if (!props.selectedServer) return;
  await loadServerLogs(props.selectedServer);
  emit('logsRefreshed');
};

// 清空当前服务器日志
const clearLogs = async () => {
  if (!props.selectedServer) return;
  
  try {
    // 调用后端清理命令
    await invoke('clear_server_memory_logs', { serverId: props.selectedServer.id });
    // 清空前端显示
    serverStore.clearServerLogs(props.selectedServer.id);
    ElMessage.success('日志已清空');
  } catch (error) {
    console.error('清空日志失败:', error);
    ElMessage.error(`清空日志失败: ${error}`);
  }
};

// 定时更新日志
let logUpdateInterval: number | null = null;

// 启动定时刷新
const startLogRefresh = () => {
  // 清除之前的定时器
  if (logUpdateInterval) {
    clearInterval(logUpdateInterval);
  }
  
  // 每500毫秒更新一次当前服务器日志（实时更新）
  logUpdateInterval = window.setInterval(() => {
    if (props.selectedServer) {
      loadServerLogs(props.selectedServer);
    }
  }, 500);
};

// 停止定时刷新
const stopLogRefresh = () => {
  if (logUpdateInterval) {
    clearInterval(logUpdateInterval);
    logUpdateInterval = null;
  }
};

// 组件挂载时启动定时刷新
onMounted(() => {
  if (props.selectedServer) {
    loadServerLogs(props.selectedServer);
    startLogRefresh();
  }
});

// 组件卸载时清理定时器
onUnmounted(() => {
  stopLogRefresh();
});


// 监听选中服务器变化，自动加载日志并重启定时器
watch(() => props.selectedServer, (newServer) => {
  if (newServer) {
    loadServerLogs(newServer);
    startLogRefresh();
  } else {
    stopLogRefresh();
  }
}, { immediate: true });
</script>

<style scoped>
/* 服务器日志区域样式 */
.server-logs-section {
  height: 100%;
  display: flex;
  flex-direction: column;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  overflow: hidden;
}

.logs-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
  flex-shrink: 0;
}

.logs-header-left {
  display: flex;
  align-items: center;
}

.logs-header h4 {
  margin: 0;
  color: #303133;
  font-size: 16px;
  font-weight: 600;
}

.logs-actions {
  gap: 8px;
}

.logs-content {
  flex: 1; 
  background-color: #1e1e1e;
  border-radius: 4px;
  border: 1px solid #e5e7eb;
  overflow: hidden;
  position: relative;
}

.log-lines {
  font-size: 12px;
  line-height: 1.4;
  padding: 12px;
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  overflow-y: auto;
}

.log-line {
  margin-bottom: 2px;
  padding: 2px 4px;
  border-radius: 2px;
  white-space: pre-wrap;
  word-break: break-word;
  color: #d4d4d4;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
}

/* 支持 ANSI 颜色样式 */
.log-line :deep(span) {
  font-family: inherit;
}

.log-line.log-error {
  background-color: rgba(244, 67, 54, 0.1);
  border-left: 3px solid #f44336;
  padding-left: 8px;
}

.log-line.log-system {
  background-color: rgba(33, 150, 243, 0.1);
  border-left: 3px solid #2196f3;
  padding-left: 8px;
}

.log-line:hover {
  background-color: rgba(255, 255, 255, 0.05);
}

/* 日志内容滚动条样式 */
.log-lines::-webkit-scrollbar {
  width: 8px;
}

.log-lines::-webkit-scrollbar-track {
  background: #2d2d2d;
  border-radius: 4px;
}

.log-lines::-webkit-scrollbar-thumb {
  background: #555;
  border-radius: 4px;
}

.log-lines::-webkit-scrollbar-thumb:hover {
  background: #666;
}
</style>