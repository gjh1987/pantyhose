<template>
  <div class="gm-panel">
    <div class="gm-content" v-if="selectedServer">
      <!-- GM命令区域 -->
      <div class="gm-command-section" :class="{ 'collapsed': gmCommandCollapsed }">
        <div class="section-title">
          <div class="title-left">
            <el-button 
              @click="gmCommandCollapsed = !gmCommandCollapsed" 
              :icon="gmCommandCollapsed ? ArrowDown : ArrowUp"
              type="text"
              size="small"
              class="collapse-btn"
            >
              {{ gmCommandCollapsed ? '展开' : '折叠' }}
            </el-button>
            <span>GM命令</span>
          </div>
        </div>
        <div class="section-content" v-show="!gmCommandCollapsed">
          <div class="command-input-group">
            <el-input
              v-model="gmCommand"
              placeholder="输入GM命令"
              size="small"
              @keyup.enter="sendGMCommand"
              class="command-input"
            />
            <el-button 
              type="primary" 
              size="small"
              @click="sendGMCommand"
              :disabled="!gmCommand.trim()"
            >
              发送
            </el-button>
          </div>
          
          <!-- 常用GM命令快捷按钮 -->
          <div class="quick-commands">
            <div class="quick-commands-title">常用命令:</div>
            <div class="command-buttons">
              <el-button
                v-for="cmd in quickCommands"
                :key="cmd.command"
                size="small"
                type="info"
                plain
                @click="executeQuickCommand(cmd.command)"
                :title="cmd.description"
              >
                {{ cmd.label }}
              </el-button>
            </div>
          </div>
        </div>
      </div>
      
      <!-- 服务器状态信息 -->
      <div class="gm-status-section" :class="{ 'collapsed': serverStatusCollapsed }">
        <div class="section-title">
          <div class="title-left">
            <el-button 
              @click="serverStatusCollapsed = !serverStatusCollapsed" 
              :icon="serverStatusCollapsed ? ArrowDown : ArrowUp"
              type="text"
              size="small"
              class="collapse-btn"
            >
              {{ serverStatusCollapsed ? '展开' : '折叠' }}
            </el-button>
            <span>服务器状态</span>
          </div>
        </div>
        <el-descriptions v-show="!serverStatusCollapsed" :column="2" border size="small">
          <el-descriptions-item label="在线玩家数">
            <el-tag type="success">{{ serverStatus.onlineCount || 0 }}</el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="运行时间">
            <el-tag type="info">{{ serverStatus.uptime || '未知' }}</el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="内存使用">
            <el-tag type="warning">{{ serverStatus.memoryUsage || '未知' }}</el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="CPU使用">
            <el-tag type="primary">{{ serverStatus.cpuUsage || '未知' }}</el-tag>
          </el-descriptions-item>
        </el-descriptions>
      </div>
      
      <!-- GM命令历史 -->
      <div class="gm-history-section" :class="{ 'collapsed': commandHistoryCollapsed }">
        <div class="section-title">
          <div class="title-left">
            <el-button 
              @click="commandHistoryCollapsed = !commandHistoryCollapsed" 
              :icon="commandHistoryCollapsed ? ArrowDown : ArrowUp"
              type="text"
              size="small"
              class="collapse-btn"
            >
              {{ commandHistoryCollapsed ? '展开' : '折叠' }}
            </el-button>
            <span>命令历史</span>
          </div>
          <el-button v-show="!commandHistoryCollapsed" type="text" size="small" @click="clearHistory" class="clear-history-btn">
            清空
          </el-button>
        </div>
        <div class="command-history" v-show="!commandHistoryCollapsed">
          <div 
            v-for="(item, index) in commandHistory" 
            :key="index"
            class="history-item"
            :class="{ 'success': item.success, 'error': !item.success }"
          >
            <div class="history-command">
              <el-icon class="history-icon">
                <component :is="item.success ? 'Check' : 'Close'" />
              </el-icon>
              <span class="command-text">{{ item.command }}</span>
              <span class="command-time">{{ item.timestamp }}</span>
            </div>
            <div v-if="item.response" class="history-response">
              {{ item.response }}
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <div v-else class="no-server-selected">
      <el-empty description="请选择一个服务器使用GM面板" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from 'vue';
import { ElMessage } from 'element-plus';
import { ArrowDown, ArrowUp } from '@element-plus/icons-vue';
import type { ServerInfo } from '../../stores/serverStore';

// Props
const props = defineProps<{
  selectedServer: ServerInfo | null;
}>();

// Emits
const emit = defineEmits<{
  commandExecuted: [command: string];
}>();

// GM命令输入
const gmCommand = ref('');

// 各区域的折叠状态
const gmCommandCollapsed = ref(false);
const serverStatusCollapsed = ref(false);
const commandHistoryCollapsed = ref(false);

// 服务器状态信息
const serverStatus = ref({
  onlineCount: 0,
  uptime: '',
  memoryUsage: '',
  cpuUsage: ''
});

// 命令历史记录
interface CommandHistoryItem {
  command: string;
  timestamp: string;
  success: boolean;
  response?: string;
}

const commandHistory = ref<CommandHistoryItem[]>([]);

// 常用GM命令
const quickCommands = [
  { label: '重载配置', command: '/reload', description: '重新加载服务器配置' },
  { label: '保存数据', command: '/save', description: '保存所有数据到数据库' },
  { label: '踢出所有玩家', command: '/kickall', description: '踢出所有在线玩家' },
  { label: '关闭服务器', command: '/shutdown', description: '安全关闭服务器' },
  { label: '查看状态', command: '/status', description: '查看服务器详细状态' },
  { label: '清理缓存', command: '/cleancache', description: '清理服务器缓存' }
];

// 发送GM命令
const sendGMCommand = async () => {
  if (!props.selectedServer || !gmCommand.value.trim()) return;
  
  const command = gmCommand.value.trim();
  const timestamp = new Date().toLocaleTimeString();
  
  try {
    // 这里应该调用实际的GM命令API
    // const result = await invoke('send_gm_command', { 
    //   serverId: props.selectedServer.id,
    //   command: command
    // });
    
    // 模拟API调用
    console.log(`Sending GM command to server ${props.selectedServer.id}: ${command}`);
    
    // 添加到历史记录
    commandHistory.value.unshift({
      command,
      timestamp,
      success: true,
      response: `命令 "${command}" 执行成功`
    });
    
    // 限制历史记录数量
    if (commandHistory.value.length > 50) {
      commandHistory.value = commandHistory.value.slice(0, 50);
    }
    
    // 清空输入框
    gmCommand.value = '';
    
    // 触发事件
    emit('commandExecuted', command);
    
    ElMessage.success('GM命令已发送');
  } catch (error) {
    console.error('发送GM命令失败:', error);
    
    // 添加失败记录
    commandHistory.value.unshift({
      command,
      timestamp,
      success: false,
      response: `命令执行失败: ${error}`
    });
    
    ElMessage.error(`GM命令发送失败: ${error}`);
  }
};

// 执行快捷命令
const executeQuickCommand = (command: string) => {
  gmCommand.value = command;
  sendGMCommand();
};

// 刷新GM数据
const refreshGMData = async () => {
  if (!props.selectedServer) return;
  
  try {
    // 这里应该调用实际的API获取服务器状态
    // const status = await invoke('get_server_gm_status', { 
    //   serverId: props.selectedServer.id 
    // });
    
    // 模拟数据
    serverStatus.value = {
      onlineCount: Math.floor(Math.random() * 100),
      uptime: '2小时30分钟',
      memoryUsage: `${Math.floor(Math.random() * 1000)}MB`,
      cpuUsage: `${Math.floor(Math.random() * 100)}%`
    };
    
    ElMessage.success('GM数据已刷新');
  } catch (error) {
    console.error('刷新GM数据失败:', error);
    ElMessage.error(`刷新失败: ${error}`);
  }
};

// 清空命令历史
const clearHistory = () => {
  commandHistory.value = [];
  ElMessage.success('命令历史已清空');
};

// 生成测试数据
const generateTestData = () => {
  const testCommands = [
    '/reload', '/save', '/kickall', '/shutdown', '/status', '/cleancache',
    '/give player item 64', '/teleport player x y z', '/ban player reason',
    '/unban player', '/kick player reason', '/mute player time',
    '/unmute player', '/heal player', '/feed player', '/fly player',
    '/gamemode creative player', '/weather clear', '/time set day',
    '/setspawn', '/home', '/spawn', '/back', '/warp location',
    '/setwarp location', '/delwarp location', '/money give player amount',
    '/money take player amount', '/rank set player rank', '/permissions add player permission'
  ];
  
  const responses = [
    '命令执行成功', '操作完成', '配置已重载', '数据已保存',
    '玩家已被踢出', '服务器已关闭', '缓存已清理', '权限已更新',
    '传送成功', '物品已给予', '玩家已被封禁', '玩家已解封',
    '玩家已静音', '玩家已治愈', '天气已更改', '时间已设置'
  ];

  for (let i = 1; i <= 200; i++) {
    const randomCommand = testCommands[Math.floor(Math.random() * testCommands.length)];
    const randomResponse = responses[Math.floor(Math.random() * responses.length)];
    const isSuccess = Math.random() > 0.1; // 90%成功率
    
    commandHistory.value.push({
      command: `${randomCommand} ${i}`,
      timestamp: new Date(Date.now() - (200 - i) * 30000).toLocaleTimeString(), // 每30秒一条
      success: isSuccess,
      response: isSuccess ? randomResponse : `命令执行失败: 权限不足或参数错误`
    });
  }
};

// 组件挂载时生成测试数据
onMounted(() => {
  generateTestData();
});

// 监听选中服务器变化
watch(() => props.selectedServer, (newServer) => {
  if (newServer) {
    refreshGMData();
  }
});
</script>

<style scoped>
.gm-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  min-height: 0;
  overflow: hidden;
}

.gm-content {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 16px;   
  padding: 0px 0;
}

.no-server-selected {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

.section-title {
  font-size: 14px;
  font-weight: 600;
  color: #303133;
  margin-bottom: 12px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.title-left {
  display: flex;
  align-items: center;
}

.collapse-btn {
  margin-right: 8px;
}

.gm-command-section {
  background: #f8f9fa;
  padding: 16px;
  border-radius: 8px;
  border: 1px solid #e5e7eb;
  transition: all 0.3s ease;
}

.gm-command-section.collapsed {
  padding-bottom: 8px;
}

.section-content {
  animation: fadeIn 0.3s ease;
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(-10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.command-input-group {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}

.command-input {
  flex: 1;
}

.quick-commands-title {
  font-size: 12px;
  color: #666;
  margin-bottom: 8px;
}

.command-buttons {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.gm-status-section {
  background: #fff;
  padding: 16px;
  border-radius: 8px;
  border: 1px solid #e5e7eb;
  transition: all 0.3s ease;
}

.gm-status-section.collapsed {
  padding-bottom: 8px;
}

.gm-history-section {
  background: #fff;
  padding: 16px;
  border-radius: 8px;
  border: 1px solid #e5e7eb;
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
  position: relative;
  transition: all 0.3s ease;
}

.gm-history-section.collapsed {
  flex: initial;
  min-height: auto;
  padding-bottom: 8px;
}

.clear-history-btn {
  margin-left: auto;
  color: #f56c6c;
}

.command-history {
  position: absolute;
  top: 60px;
  left: 16px;
  right: 16px;
  bottom: 16px;
  overflow-y: auto;
}

.history-item {
  margin-bottom: 12px;
  padding: 8px;
  border-radius: 4px;
  border: 1px solid #e5e7eb;
  background: #fafafa;
}

.history-item.success {
  border-color: #67c23a;
  background: #f0f9ff;
}

.history-item.error {
  border-color: #f56c6c;
  background: #fef2f2;
}

.history-command {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}

.history-icon {
  font-size: 14px;
}

.history-item.success .history-icon {
  color: #67c23a;
}

.history-item.error .history-icon {
  color: #f56c6c;
}

.command-text {
  flex: 1;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 13px;
  color: #303133;
}

.command-time {
  font-size: 11px;
  color: #909399;
}

.history-response {
  font-size: 12px;
  color: #666;
  padding-left: 22px;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
}

/* 滚动条样式 */
.gm-content::-webkit-scrollbar,
.command-history::-webkit-scrollbar {
  width: 6px;
}

.gm-content::-webkit-scrollbar-track,
.command-history::-webkit-scrollbar-track {
  background: #f1f1f1;
  border-radius: 3px;
}

.gm-content::-webkit-scrollbar-thumb,
.command-history::-webkit-scrollbar-thumb {
  background: #c1c1c1;
  border-radius: 3px;
}

.gm-content::-webkit-scrollbar-thumb:hover,
.command-history::-webkit-scrollbar-thumb:hover {
  background: #a8a8a8;
}
</style>