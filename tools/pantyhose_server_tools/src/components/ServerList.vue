<template>
  <el-splitter class="server-list-container" default-size="25%">
    <!-- 左侧面板 -->
    <el-splitter-panel :size="300" :max="400">
      <div class="server-list-sidebar">
        <!-- 固定顶部 -->
        <div class="sidebar-header">
          <div class="title-with-checkbox">
            <el-checkbox 
              v-if="serverGroups.length > 0"
              v-model="allSelected" 
              :indeterminate="isIndeterminate"
              @change="handleSelectAll"
              class="select-all-checkbox"
            />
            <h2>服务器列表</h2>
            <el-button
              v-if="serverGroups.length > 0"
              type="primary"
              size="small"
              circle
              @click="refreshAllServerStatus"
              title="刷新状态"
              style="margin-left: auto;"
            >
              <el-icon><Refresh /></el-icon>
            </el-button>
          </div>
          <el-tag v-if="serverGroups.length > 0" type="info" size="small" class="server-count-tag">
            {{ getTotalRunningCount() }}/{{ getTotalServerCount() }}
          </el-tag>
        </div>
        
        <!-- 可滚动内容区 -->
        <div class="sidebar-content">
          <!-- 空状态提示 -->
          <div v-if="serverGroups.length === 0" class="empty-state">
            <el-empty description="暂无服务器数据">
              <el-button type="primary" @click="goToSettings">
                返回配置页面
              </el-button>
            </el-empty>
          </div>
          
          <!-- 服务器列表 -->
          <el-collapse v-else v-model="activeNames" @change="handleCollapseChange">
            <el-collapse-item 
              v-for="group in serverGroups" 
              :key="group.id" 
              :name="group.id"
            >
              <template #title>
                <div class="collapse-title">
                  <span class="group-name">{{ group.name }}</span>
                  <el-tag 
                    :type="getRunningCount(group) === group.children.length ? 'success' : getRunningCount(group) > 0 ? 'warning' : 'info'"
                    size="small"
                    class="status-tag"
                  >
                    {{ getRunningCount(group) }}/{{ group.children.length }}
                  </el-tag>
                </div>
              </template>
              <div class="server-list">
                <div 
                  v-for="server in group.children" 
                  :key="server.id" 
                  class="server-item"
                  :class="{ 'selected': selectedServer?.id === server.id }"
                  @click="handleServerClick(server)"
                >
                  <el-checkbox 
                    v-model="server.checked" 
                    @click.stop 
                    @change="handleServerCheck(server)"
                    class="server-checkbox"
                  />
                  <el-icon class="server-status-icon" :class="server.isRunning ? 'running' : 'stopped'">
                    <component :is="server.isRunning ? 'VideoPlay' : 'VideoPause'" />
                  </el-icon>
                  <span class="server-name">{{ server.name || `服务器-${server.id}` }}</span>
                  <el-tag 
                    :type="server.isRunning ? 'success' : 'danger'"
                    size="small"
                    effect="plain"
                    class="server-status-tag"
                  >
                    {{ server.isRunning ? '运行中' : '已停止' }}
                  </el-tag>
                </div>
              </div>
            </el-collapse-item>
          </el-collapse>
        </div>
        
        <!-- 固定底部 -->
        <div v-if="serverGroups.length > 0" class="batch-actions">
          <div class="clear-logs-option">
            <el-checkbox
              v-model="clearLogsOnStart"
              size="small"
            >
              启动前清理日志
            </el-checkbox>
          </div>
          <div class="batch-operations">
            <el-button 
              type="success" 
              size="small" 
              :disabled="getCheckedServers().length === 0"
              @click="batchStart"
            >
              批量启动
            </el-button>
            <el-button 
              type="danger" 
              size="small" 
              :disabled="getCheckedServers().length === 0"
              @click="batchStop"
            >
              批量关闭
            </el-button>
          </div>
        </div>
      </div>
    </el-splitter-panel>
    
    <!-- 右侧面板 -->
    <el-splitter-panel>
      <div class="server-details">
        <div class="placeholder" v-if="openedTabs.length === 0">
          请选择一个服务器查看详情
        </div>
        <el-tabs 
          v-else
          v-model="activeTabId" 
          type="card" 
          closable
          @tab-remove="removeTab"
          class="server-detail-tabs"
        >
          <el-tab-pane 
            v-for="tab in openedTabs" 
            :key="tab.serverId"
            :name="tab.serverId"
          >
            <template #label>
              <span class="tab-label">
                <el-icon v-if="tab.server.isRunning" class="status-icon running">
                  <VideoPlay />
                </el-icon>
                <el-icon v-else class="status-icon stopped">
                  <VideoPause />
                </el-icon>
                {{ tab.server.name || `服务器-${tab.server.id}` }}
              </span>
            </template>
            <ServerDetail 
              :server="tab.server"
              :clear-logs-on-start="clearLogsOnStart"
              @server-status-changed="handleServerStatusChanged"
            />
          </el-tab-pane>
        </el-tabs>
      </div>
    </el-splitter-panel>
  </el-splitter>
</template>

<script setup lang="ts">
import { ref, onUnmounted, computed, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import { VideoPlay, VideoPause, Refresh } from '@element-plus/icons-vue';
import { serverStore, type ServerInfo, type ServerGroup } from '../stores/serverStore';
import { invoke } from '@tauri-apps/api/core';
import { ElMessage } from 'element-plus';
import ServerDetail from './server_panel/ServerDetail.vue';
import serverManager from '../utils/ServerManager';


// 使用全局状态的服务器组数据
const serverGroups = computed(() => serverStore.getServerGroups());

// Collapse 组件的活跃面板
const activeNames = ref<string[]>([]);

// el-splitter 已经内置了拖拽功能，不再需要这些状态


// 初始化展开状态
const initExpandedState = () => {
  // 默认不展开任何组
  activeNames.value = [];
};

// 监听服务器数据变化
computed(() => {
  return serverGroups.value;
});

// 在组件挂载时初始化展开状态
initExpandedState();

// 处理服务器状态更新
const handleStatusUpdate = (statusMap: Record<string, boolean>) => {
  serverGroups.value.forEach(group => {
    group.children.forEach(server => {
      const isRunning = statusMap[server.id] || false;
      if (server.isRunning !== isRunning) {
        serverStore.updateServerStatus(server.id, isRunning);
      }
    });
  });
};

// 初始化状态
const initServerStatus = async () => {
  try {
    const statusMap = await invoke('get_all_server_status') as Record<string, boolean>;
    serverManager.updateAllStatus(statusMap);
    handleStatusUpdate(statusMap);
  } catch (error) {
    console.error('获取服务器状态失败:', error);
  }
};

// 刷新所有服务器状态（主动检查进程）
const refreshAllServerStatus = async () => {
  try {
    await invoke('refresh_all_server_status');
  } catch (error) {
    console.error('刷新服务器状态失败:', error);
  }
};

// 组件挂载时初始化
onMounted(() => {
  // 注册全局状态监听器（用于接收Rust端的实时更新）
  serverManager.addGlobalListener(handleStatusUpdate);
  
  // 立即获取一次初始状态
  initServerStatus();
});

// 组件卸载时清理
onUnmounted(() => {
  serverManager.removeGlobalListener(handleStatusUpdate);
});

// 路由器实例
const router = useRouter();

// 标签页管理
interface ServerTab {
  serverId: string;
  server: ServerInfo;
}

const openedTabs = ref<ServerTab[]>([]);
const activeTabId = ref<string>('');

// 选中的服务器
const selectedServer = computed(() => {
  const tab = openedTabs.value.find(t => t.serverId === activeTabId.value);
  return tab?.server || null;
});

// 全选状态
const allSelected = ref(false);
const isIndeterminate = ref(false);


// 启动前清理日志选项
const clearLogsOnStart = ref(false);


// 处理折叠面板变化
const handleCollapseChange = (activeNames: string[]) => {
  console.log('活跃面板:', activeNames);
};

// 处理服务器点击事件
const handleServerClick = (server: ServerInfo) => {
  // 检查是否已经打开
  const existingTab = openedTabs.value.find(t => t.serverId === server.id);
  
  if (existingTab) {
    // 如果已经打开，切换到该标签页
    activeTabId.value = server.id;
  } else {
    // 如果没有打开，添加新标签页
    openedTabs.value.push({
      serverId: server.id,
      server: server
    });
    activeTabId.value = server.id;
  }
};

// 移除标签页
const removeTab = (targetId: string) => {
  const tabs = openedTabs.value;
  let activeName = activeTabId.value;
  
  if (activeName === targetId) {
    tabs.forEach((tab, index) => {
      if (tab.serverId === targetId) {
        const nextTab = tabs[index + 1] || tabs[index - 1];
        if (nextTab) {
          activeName = nextTab.serverId;
        }
      }
    });
  }
  
  activeTabId.value = activeName;
  openedTabs.value = tabs.filter(tab => tab.serverId !== targetId);
};

// 处理服务器状态变化
const handleServerStatusChanged = (serverId: string, isRunning: boolean) => {
  // 更新对应标签页中的服务器状态
  const tab = openedTabs.value.find(t => t.serverId === serverId);
  if (tab) {
    tab.server.isRunning = isRunning;
  }
};


// 获取组内运行中的服务器数量
const getRunningCount = (group: ServerGroup) => {
  return serverStore.getRunningCount(group);
};

// 获取总的运行中服务器数量
const getTotalRunningCount = () => {
  return serverGroups.value.reduce((total, group) => {
    return total + getRunningCount(group);
  }, 0);
};

// 获取总的服务器数量
const getTotalServerCount = () => {
  return serverGroups.value.reduce((total, group) => {
    return total + group.children.length;
  }, 0);
};

// 获取当前配置的服务器路径
const getServerPath = async (): Promise<string> => {
  try {
    const savedPath = await invoke('load_server_path') as string | null;
    return savedPath || "";
  } catch (error) {
    console.error('获取服务器路径失败:', error);
    return '';
  }
};

// 切换服务器运行状态（真实实现）
// const toggleServerStatus = async (server: ServerInfo) => {
//   try {
//     const serverPath = await getServerPath();
    
//     if (server.isRunning) {
//       // 停止服务器
//       const result = await invoke('stop_server', { serverId: server.id }) as boolean;
//       if (result) {
//         serverStore.updateServerStatus(server.id, false);
//         ElMessage.success(`服务器 ${server.name || server.id} 已停止`);
//       } else {
//         ElMessage.error(`停止服务器 ${server.name || server.id} 失败`);
//       }
//     } else {
//       // 启动服务器
//       const config = serverStore.getConfig();
//       const result = await invoke('start_server', { 
//         serverPath: serverPath,
//         serverId: server.id,
//         executableName: config.executableName,
//         configFileName: config.configFileName
//       }) as boolean;
//       if (result) {
//         serverStore.updateServerStatus(server.id, true);
//         ElMessage.success(`服务器 ${server.name || server.id} 已启动`);
//       } else {
//         ElMessage.error(`启动服务器 ${server.name || server.id} 失败`);
//       }
//     }
//   } catch (error) {
//     console.error('切换服务器状态失败:', error);
//     ElMessage.error(`操作失败: ${error}`);
//   }
// };

// 处理服务器勾选状态变化
const handleServerCheck = (server: ServerInfo) => {
  console.log(`服务器 ${server.name} 勾选状态: ${server.checked}`);
  updateSelectAllState();
};

// 更新全选状态
const updateSelectAllState = () => {
  const allServers = getAllServers();
  const checkedServers = allServers.filter(server => server.checked);
  
  if (checkedServers.length === 0) {
    allSelected.value = false;
    isIndeterminate.value = false;
  } else if (checkedServers.length === allServers.length) {
    allSelected.value = true;
    isIndeterminate.value = false;
  } else {
    allSelected.value = false;
    isIndeterminate.value = true;
  }
};

// 获取所有服务器
const getAllServers = () => {
  const allServers: ServerInfo[] = [];
  serverGroups.value.forEach(group => {
    allServers.push(...group.children);
  });
  return allServers;
};

// 处理全选/取消全选
const handleSelectAll = (checked: boolean) => {
  serverGroups.value.forEach(group => {
    group.children.forEach(server => {
      server.checked = checked;
    });
  });
  updateSelectAllState();
};

// 获取所有勾选的服务器
const getCheckedServers = () => {
  const checkedServers: ServerInfo[] = [];
  serverGroups.value.forEach(group => {
    group.children.forEach(server => {
      if (server.checked) {
        checkedServers.push(server);
      }
    });
  });
  return checkedServers;
};


// 批量启动
const batchStart = async () => {
  const checkedServers = getCheckedServers();
  const serverPath = await getServerPath();
  let successCount = 0;
  let errorCount = 0;
  let logClearCount = 0;
  
  // 如果勾选了清理日志选项，先清理所有选中服务器的日志
  if (clearLogsOnStart.value) {
    for (const server of checkedServers) {
      try {
        await invoke('clear_server_memory_logs', { serverId: server.id });
        // 清理前端显示的日志
        serverStore.clearServerLogs(server.id);
        logClearCount++;
        console.log(`已清理服务器 ${server.id} 的日志`);
      } catch (error) {
        console.error(`清理服务器 ${server.id} 日志失败:`, error);
      }
    }
    if (logClearCount > 0) {
      ElMessage.success(`已清理 ${logClearCount} 个服务器的日志`);
    }
  }
  
  // 然后启动未运行的服务器
  for (const server of checkedServers) {
    if (!server.isRunning) {
      try {
        const config = serverStore.getConfig();
        const result = await invoke('start_server', { 
          serverPath: serverPath,
          serverId: server.id,
          executableName: config.executableName,
          configFileName: config.configFileName
        }) as boolean;
        if (result) {
          serverStore.updateServerStatus(server.id, true);
          successCount++;
        } else {
          errorCount++;
        }
      } catch (error) {
        console.error(`启动服务器 ${server.id} 失败:`, error);
        errorCount++;
      }
    }
  }
  
  if (successCount > 0) {
    ElMessage.success(`成功启动 ${successCount} 个服务器`);
  }
  if (errorCount > 0) {
    ElMessage.error(`${errorCount} 个服务器启动失败`);
  }
};

// 批量关闭
const batchStop = async () => {
  const checkedServers = getCheckedServers();
  let successCount = 0;
  let errorCount = 0;
  
  for (const server of checkedServers) {
    if (server.isRunning) {
      try {
        const result = await invoke('stop_server', { serverId: server.id }) as boolean;
        if (result) {
          serverStore.updateServerStatus(server.id, false);
          successCount++;
        } else {
          errorCount++;
        }
      } catch (error) {
        console.error(`停止服务器 ${server.id} 失败:`, error);
        errorCount++;
      }
    }
  }
  
  if (successCount > 0) {
    ElMessage.success(`成功停止 ${successCount} 个服务器`);
  }
  if (errorCount > 0) {
    ElMessage.error(`${errorCount} 个服务器停止失败`);
  }
};

// 重启服务器
// const restartServer = async (server: ServerInfo) => {
//   try {
//     const serverPath = await getServerPath();
//     ElMessage.info(`正在重启服务器 ${server.name || server.id}...`);
    
//     const config = serverStore.getConfig();
//     const result = await invoke('restart_server', { 
//       serverPath: serverPath,
//       serverId: server.id,
//       executableName: config.executableName,
//       configFileName: config.configFileName
//     }) as boolean;
    
//     if (result) {
//       serverStore.updateServerStatus(server.id, true);
//       ElMessage.success(`服务器 ${server.name || server.id} 重启成功`);
//     } else {
//       ElMessage.error(`重启服务器 ${server.name || server.id} 失败`);
//     }
//   } catch (error) {
//     console.error('重启服务器失败:', error);
//     ElMessage.error(`重启失败: ${error}`);
//   }
// };


// 返回设置页面
const goToSettings = () => {
  router.push('/');
};
</script>

<style scoped>
/* 左侧边栏 - 使用 flex 布局 */
.server-list-sidebar {
  display: flex;
  flex-direction: column;
  background: white;
  height: 100%;
  position: relative;
}

/* 固定顶部的头部 */
.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px;
  background: white;
  border-bottom: 1px solid #e5e7eb;
  flex-shrink: 0;
}

/* 可滚动的内容区 */
.sidebar-content {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden; /* 防止横向滚动 */
  padding: 20px;
}

.server-list-sidebar h2 {
  margin: 0;
  color: #303133;
  font-size: 20px;
  font-weight: 600;
}

.title-with-checkbox {
  display: flex;
  align-items: center;
  gap: 8px;
}

.server-count-tag {
  font-weight: 600;
}

.server-details {
  background: white;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden; /* 防止整体滚动 */
}

.placeholder {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #909399;
  font-size: 16px;
  padding: 20px;
}

/* 空状态样式 */
.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 200px;
  padding: 40px;
}

/* Collapse 组件相关样式 */
.collapse-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-right: 30px; /* 给折叠箭头留空间，防止内容超出 */
}

.group-name {
  font-weight: 600;
  color: #303133;
  font-size: 16px;
  margin-left: 10px;
}

.status-tag {
  margin-left: auto;
}

/* 服务器列表样式 */
.server-list {
  margin-top: 2px;
}

.server-item {
  display: flex;
  align-items: center;
  padding: 4px 6px;
  margin-bottom: 3px;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.2s;
  border: 1px solid #f0f0f0;
  background-color: #fafafa;
  overflow: hidden; /* 防止内容溢出 */
}

.server-item:hover {
  background-color: #f5f7fa;
  border-color: #c6e2ff;
  transform: translateY(-1px);
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
}

.server-item.selected {
  background-color: #e6f7ff;
  border-color: #409eff;
  box-shadow: 0 4px 12px rgba(64, 158, 255, 0.2);
}

.server-status-icon {
  margin-right: 4px;
  font-size: 12px;
}

.server-status-icon.running {
  color: #67c23a;
}

.server-status-icon.stopped {
  color: #f56c6c;
}

.server-name {
  flex: 1;
  color: #303133;
  font-size: 12px;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0; /* 允许 flex 项收缩 */
}

.server-status-tag {
  margin-left: auto;
}

.server-checkbox {
  margin-right: 6px;
}

/* 固定底部的批量操作 */
.batch-actions {
  padding: 12px 20px;
  background: #f8f9fa;
  border-top: 1px solid #e5e7eb;
  flex-shrink: 0;
}

.clear-logs-option {
  margin-bottom: 8px;
  padding: 6px;
  background-color: #fff;
  border-radius: 4px;
  border: 1px solid #e5e7eb;
  display: flex;
  align-items: center;
}

.batch-operations {
  display: flex;
  gap: 8px;
}

/* 服务器详情标签页样式 */
.server-detail-tabs {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.server-detail-tabs :deep(.el-tabs__header) {
  margin: 0;
  padding: 10px 10px 0;
  background-color: #f5f7fa;
  border-bottom: 1px solid #e4e7ed;
}

.server-detail-tabs :deep(.el-tabs__content) {
  flex: 1;
  padding: 0;
  overflow: hidden;
}

.server-detail-tabs :deep(.el-tab-pane) {
  height: 100%;
}

.tab-label {
  display: flex;
  align-items: center;
  gap: 6px;
}

.status-icon {
  font-size: 14px;
}

.status-icon.running {
  color: #67c23a;
}

.status-icon.stopped {
  color: #909399;
}

/* 服务器详情样式 */
.server-info-container {
  padding-bottom: 16px;
  border-bottom: 1px solid #e5e7eb;
  margin-bottom: 16px;
  transition: all 0.3s ease;
}

.server-info-container.collapsed {
  padding-bottom: 0;
  margin-bottom: 8px;
}

.server-info-container.collapsed .detail-header {
  margin-bottom: 0;
  padding-bottom: 8px;
}

.server-info-container.collapsed .server-info,
.server-info-container.collapsed .server-actions {
  display: none;
}

.detail-header {
  display: flex;
  align-items: center;
  margin-bottom: 24px;
  padding-bottom: 16px;
  border-bottom: 2px solid #f0f2f5;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.collapse-btn {
  margin-right: 8px;
}

.detail-header h3 {
  margin: 0;
  color: #303133;
  font-size: 24px;
  font-weight: 600;
}

.server-info {
  margin-bottom: 24px;
  max-width: 600px;
}

.server-actions {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.server-actions .el-button {
  min-width: 120px;
}

/* Element Plus 组件样式覆盖 */
:deep(.el-collapse) {
  border: none;
}

:deep(.el-collapse-item__header) {
  background: #f8f9fa;
  border: 1px solid #e5e7eb;
  border-radius: 6px;
  padding: 0px 2px;
  margin-bottom: 6px;
  font-size: 14px;
  transition: all 0.2s;
}

:deep(.el-collapse-item__header:hover) {
  background: #e9ecef;
  border-color: #409eff;
}

:deep(.el-collapse-item__header.is-active) {
  background: #e6f7ff;
  border-color: #409eff;
  color: #409eff;
}

:deep(.el-collapse-item__wrap) {
  border: none;
}

:deep(.el-collapse-item__content) {
  padding: 0 8px 16px 8px;
}

:deep(.el-collapse-item__arrow) {
  margin-left: 8px;
}

/* 服务器详情内容容器 */
.server-detail-content {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 20px;
  box-sizing: border-box;
}

/* 服务器选项卡样式 */
.server-tabs-section {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.server-tabs {
  flex: 1;
  display: flex;
  flex-direction: column;
}

:deep(.server-tabs .el-tabs__content) {
  flex: 1;
  display: flex;
  flex-direction: column;
}

:deep(.server-tabs .el-tab-pane) {
  flex: 1;
  display: flex;
  flex-direction: column;
}

:deep(.server-tabs .el-tabs__header) {
  margin-bottom: 16px;
}

:deep(.server-tabs .el-tabs__nav-wrap) {
  background: #f8f9fa;
  border-radius: 8px 8px 0 0;
  border: 1px solid #e5e7eb;
  border-bottom: none;
}

:deep(.server-tabs .el-tabs__item) {
  padding: 8px 16px;
  height: 36px;
  line-height: 20px;
  font-size: 14px;
  color: #666;
}

:deep(.server-tabs .el-tabs__item.is-active) {
  background: #fff;
  color: #409eff;
  border-bottom-color: #fff;
}

:deep(.server-tabs .el-tabs__item:hover) {
  color: #409eff;
}

</style>