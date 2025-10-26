<template>
  <div class="navbar">
    <div class="navbar-brand">
      <img src="/src/assets/icon.png" alt="Logo" class="brand-logo" />
      <span class="brand-title">黑死魂服务器管理</span>
    </div>
    <el-menu
      :default-active="activeIndex"
      mode="horizontal"
      @select="handleSelect"
      class="navbar-menu"
      :ellipsis="false"
    >
      <el-menu-item index="servers">
        <el-icon><Connection /></el-icon>
        <span>服务器</span>
      </el-menu-item>
      <el-menu-item index="docs">
        <el-icon><Document /></el-icon>
        <span>说明</span>
      </el-menu-item>
      <el-sub-menu index="tools">
        <template #title>
          <el-icon><Setting /></el-icon>
          <span>工具</span>
        </template>
        <el-menu-item index="clear-logs">
          <el-icon><Delete /></el-icon>
          <span>清理全部日志</span>
        </el-menu-item>
        <el-menu-item index="build-server">
          <el-icon><Cpu /></el-icon>
          <span>编译服务器</span>
        </el-menu-item>
        <el-menu-item index="kill-servers">
          <el-icon><Close /></el-icon>
          <span>强制关闭所有 {{ serverStore.getConfig().executableName }}</span>
        </el-menu-item>
      </el-sub-menu>
    </el-menu>

    <!-- 清理日志对话框 -->
    <el-dialog
      v-model="clearLogsDialogVisible"
      title="清理全部日志"
      width="500px"
      :close-on-click-modal="false"
    >
      <el-alert
        title="注意"
        type="info"
        :closable="false"
        show-icon
        style="margin-bottom: 20px"
      >
        此操作将清空所有服务器的日志显示内容
      </el-alert>
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="clearLogsDialogVisible = false">取消</el-button>
          <el-button 
            type="primary" 
            @click="handleClearAllLogs"
            :loading="isClearing"
          >
            确认清空
          </el-button>
        </span>
      </template>
    </el-dialog>

    <!-- 编译服务器对话框 -->
    <el-dialog
      v-model="buildServerDialogVisible"
      title="编译服务器"
      width="600px"
      :close-on-click-modal="false"
    >
      <el-form label-width="100px">
        <el-form-item label="编译选项">
          <el-radio-group v-model="buildOption">
            <el-radio label="debug">Debug模式</el-radio>
            <el-radio label="release">Release模式</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="清理缓存">
          <el-switch v-model="cleanBuild" />
          <span style="margin-left: 10px; color: #909399;">清理之前的编译缓存</span>
        </el-form-item>
      </el-form>
      <div v-if="buildOutput" style="margin-top: 20px;">
        <el-divider />
        <div class="build-output" style="background: #f5f7fa; padding: 10px; border-radius: 4px; max-height: 300px; overflow-y: auto;">
          <pre style="margin: 0; font-family: 'Consolas', 'Monaco', monospace; font-size: 12px;">{{ buildOutput }}</pre>
        </div>
      </div>
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="buildServerDialogVisible = false">关闭</el-button>
          <el-button 
            type="danger" 
            @click="handleStopBuild"
            :disabled="!isBuilding"
          >
            停止编译
          </el-button>
          <el-button 
            type="primary" 
            @click="handleBuildServer"
            :loading="isBuilding"
            :disabled="isBuilding"
          >
            开始编译
          </el-button>
        </span>
      </template>
    </el-dialog>

    <!-- 强制关闭对话框 -->
    <el-dialog
      v-model="killServersDialogVisible"
      title="强制关闭"
      width="500px"
      :close-on-click-modal="false"
    >
      <el-alert
        title="注意"
        type="warning"
        :closable="false"
        show-icon
        style="margin-bottom: 20px"
      >
        此操作将强制关闭所有正在运行的进程，请确认操作
      </el-alert>
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="killServersDialogVisible = false">取消</el-button>
          <el-button 
            type="danger" 
            @click="handleKillAllServers"
            :loading="isKillingServers"
          >
            确认强制关闭
          </el-button>
        </span>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, nextTick, onUnmounted } from 'vue';
import { Connection, Document, Setting, Delete, Cpu, Close } from '@element-plus/icons-vue';
import { ElMessage } from 'element-plus';
import { invoke } from '@tauri-apps/api/core';
import { serverStore } from '../stores/serverStore';

// 不再使用路由

// 对话框状态
const clearLogsDialogVisible = ref(false);
const buildServerDialogVisible = ref(false);
const killServersDialogVisible = ref(false);
const isClearing = ref(false);
const isBuilding = ref(false);
const isKillingServers = ref(false);
const buildOption = ref('release');
const cleanBuild = ref(false);
const buildOutput = ref('');

// 当前激活的菜单项
const activeIndex = ref('servers');

// 定义 emit
const emit = defineEmits<{
  'tab-change': [tab: string];
}>();

// 处理菜单选择
const handleSelect = (key: string) => {
  switch (key) {
    case 'servers':
      activeIndex.value = 'servers';
      emit('tab-change', 'servers');
      break;
    case 'docs':
      activeIndex.value = 'docs';
      emit('tab-change', 'docs');
      break;
    case 'clear-logs':
      clearLogsDialogVisible.value = true;
      break;
    case 'build-server':
      buildServerDialogVisible.value = true;
      buildOutput.value = '';
      break;
    case 'kill-servers':
      killServersDialogVisible.value = true;
      break;
  }
};

// 处理清理所有日志
const handleClearAllLogs = async () => {
  try {
    isClearing.value = true;
    
    // 获取所有服务器并调用后端清理命令
    const serverGroups = serverStore.getServerGroups();
    for (const group of serverGroups) {
      for (const server of group.children) {
        try {
          await invoke('clear_server_memory_logs', { serverId: server.id });
        } catch (error) {
          console.error(`清理服务器 ${server.id} 日志失败:`, error);
        }
      }
    }
    
    // 清空前端显示
    serverStore.clearAllServerLogs();
    
    ElMessage.success('已清空所有服务器的日志');
    clearLogsDialogVisible.value = false;
  } catch (error) {
    console.error('清理日志失败:', error);
    ElMessage.error('清理日志失败: ' + error);
  } finally {
    isClearing.value = false;
  }
};

// 日志更新定时器
let buildLogInterval: ReturnType<typeof setInterval> | null = null;

// 处理编译服务器
const handleBuildServer = async () => {
  try {
    isBuilding.value = true;
    buildOutput.value = '';
    
    // 从 serverStore 获取可执行程序名
    const config = serverStore.getConfig();
    
    // 启动编译进程
    await invoke('start_build_server', {
      mode: buildOption.value,
      clean: cleanBuild.value,
      executableName: config.executableName
    });
    
    // 开始定时获取日志
    buildLogInterval = setInterval(async () => {
      try {
        // 获取新的日志
        const logs = await invoke('get_build_logs') as string[];
        if (logs && logs.length > 0) {
          buildOutput.value += logs.join('\n') + '\n';
          
          // 自动滚动到底部
          nextTick(() => {
            const outputElement = document.querySelector('.build-output');
            if (outputElement) {
              outputElement.scrollTop = outputElement.scrollHeight;
            }
          });
        }
        
        // 检查编译是否还在运行
        const running = await invoke('is_build_running') as boolean;
        if (!running) {
          // 编译结束，停止定时器
          if (buildLogInterval) {
            clearInterval(buildLogInterval);
            buildLogInterval = null;
          }
          isBuilding.value = false;
          
          // 检查是否成功
          if (buildOutput.value.includes('编译成功完成')) {
            ElMessage.success('服务器编译完成');
          } else if (buildOutput.value.includes('[ERROR]')) {
            ElMessage.error('编译失败，请查看输出日志');
          }
        }
      } catch (error) {
        console.error('获取编译日志失败:', error);
      }
    }, 500); // 每500ms更新一次日志
    
  } catch (error) {
    console.error('启动编译失败:', error);
    buildOutput.value = `启动编译失败: ${error}`;
    ElMessage.error('启动编译失败: ' + error);
    isBuilding.value = false;
  }
};

// 停止编译
const handleStopBuild = async () => {
  try {
    await invoke('stop_build');
    
    // 停止日志更新
    if (buildLogInterval) {
      clearInterval(buildLogInterval);
      buildLogInterval = null;
    }
    
    isBuilding.value = false;
    ElMessage.info('编译已停止');
  } catch (error) {
    console.error('停止编译失败:', error);
    ElMessage.error('停止编译失败: ' + error);
  }
};

// 处理强制关闭
const handleKillAllServers = async () => {
  try {
    isKillingServers.value = true;
    
    // 从 serverStore 获取可执行程序名
    const config = serverStore.getConfig();
    
    // 调用后端接口关闭所有进程
    await invoke('kill_all_servers', { executableName: config.executableName });
    
    ElMessage.success(`已强制关闭所有 ${config.executableName} 进程`);
    killServersDialogVisible.value = false;
  } catch (error) {
    console.error('强制关闭进程失败:', error);
    ElMessage.error('强制关闭进程失败: ' + error);
  } finally {
    isKillingServers.value = false;
  }
};

// 组件卸载时清理定时器
onUnmounted(() => {
  if (buildLogInterval) {
    clearInterval(buildLogInterval);
  }
});

</script>

<style scoped>
.navbar {
  background: white;
  border-bottom: 1px solid #e5e7eb;
  display: flex;
  align-items: center;
  justify-content: space-between;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  height: 60px;
}

.navbar-brand {
  display: flex;
  align-items: center;
}

.brand-logo {
  height: 60px;
  object-fit: contain;
}

.brand-title {
  font-size: 20px;
  font-weight: 600;
  color: #409eff;
  white-space: nowrap;
  margin-left: 0px;
  margin-top: 6px; /* 调整这个值来改变文字垂直位置 */
}

.navbar-menu {
  border-bottom: none;
  height: 60px;
}

:deep(.el-menu--horizontal) {
  border-bottom: none;
}

:deep(.el-menu-overflow) {
  display: none !important;
}

:deep(.el-menu--horizontal > .el-menu-item) {
  flex-shrink: 0;
}

:deep(.el-menu--horizontal > .el-sub-menu) {
  flex-shrink: 0;
}

:deep(.el-menu--horizontal .el-menu--popup) {
  min-width: 150px;
}

:deep(.el-menu-item) {
  font-size: 14px;
  font-weight: 500;
  padding: 0 20px;
  height: 60px;
  line-height: 60px;
}

:deep(.el-sub-menu) {
  font-size: 14px;
  font-weight: 500;
}

:deep(.el-sub-menu__title) {
  padding: 0 20px;
  height: 60px;
  line-height: 60px;
}

:deep(.el-menu-item:hover) {
  background-color: #f5f7fa;
}

:deep(.el-menu-item.is-active) {
  color: #409eff;
  border-bottom: 2px solid #409eff;
}

:deep(.el-menu-item .el-icon) {
  margin-right: 8px;
  font-size: 18px;
}


:deep(.el-dropdown-menu__item) {
  font-size: 13px;
}

:deep(.el-dropdown-menu__item .el-icon) {
  margin-right: 5px;
}
</style>