<template>
  <div class="settings-container">
    <div class="header-section">
      <p class="subtitle">配置服务器目录路径以开始管理</p>
    </div>
    
    <el-form label-width="120px">
      <el-form-item label="服务器目录">
        <el-input v-model="serverPath" placeholder="请输入服务器目录路径" />
      </el-form-item>
      
      <el-form-item label="配置文件名">
        <el-input v-model="configFileName" placeholder="配置文件名" />
      </el-form-item>
      
      <el-form-item label="执行程序名">
        <el-input v-model="executableName" placeholder="执行程序名" />
      </el-form-item>
      
      <el-form-item label="协议目录">
        <el-input v-model="protoPath" placeholder="Proto文件目录路径（如：tools/proto/config）" />
      </el-form-item>
      
      <el-form-item>
        <el-button type="primary" @click="handleStart" style="margin-top: 20px;">开始</el-button>
        <el-alert v-if="errorMessage" type="error" show-icon style="margin-top: 10px;">
          {{ errorMessage }}
        </el-alert>
      </el-form-item>
    </el-form>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useRouter } from 'vue-router';
import { serverStore } from '../stores/serverStore';

const serverPath = ref('.\\bin');
const configFileName = ref('config.xml');
const executableName = ref('pantyhose.exe');
const protoPath = ref('tools\\proto\\config');

// 组件挂载时加载保存的配置
onMounted(async () => {
  try {
    // 加载服务器路径
    const savedPath = await invoke('load_server_path') as string | null;
    if (savedPath) {
      serverPath.value = savedPath;
    }
    
    // 加载启动配置参数（包括proto路径）
    const [savedConfigFileName, savedExecutableName, savedProtoPath] = await invoke('load_startup_config') as [string | null, string | null, string | null];
    if (savedConfigFileName) {
      configFileName.value = savedConfigFileName;
    }
    if (savedExecutableName) {
      executableName.value = savedExecutableName;
    }
    if (savedProtoPath) {
      protoPath.value = savedProtoPath;
    }
    
    // 如果都为null，保持使用默认值
  } catch (error) {
    console.warn('加载配置失败，使用默认值:', error);
  }
});
const errorMessage = ref('');
const router = useRouter();

const handleStart = async () => {
  try {
    // 清除之前的错误消息
    errorMessage.value = '';
    
    // 保存当前配置到本地文件
    await invoke('save_server_path', { path: serverPath.value });
    await invoke('save_startup_config', {
      configFileName: configFileName.value,
      executableName: executableName.value,
      protoPath: protoPath.value
    });
    
    // 调用Rust的解析函数
    const result = await invoke('parse_server_config', {
      path: serverPath.value,
      configFileName: configFileName.value
    }) as { servers: any[] };
    
    console.log('解析结果:', result);
    console.log('服务器数据:', result.servers);
    
    // 为每个服务器添加运行状态和勾选状态（默认为false）
    const processedServers = result.servers.map(group => ({
      ...group,
      children: group.children.map((server: any) => ({
        ...server,
        isRunning: server.isRunning || false,
        checked: server.checked || false
      }))
    }));
    
    // 保存到全局状态
    serverStore.setServerGroups(processedServers);
    serverStore.serverPath = serverPath.value;
    serverStore.protoPath = protoPath.value;
    
    // 保存配置参数到全局状态
    serverStore.setConfig(configFileName.value, executableName.value);
    
    // 解析成功，导航到主布局界面
    router.push('/main');
  } catch (error) {
    // 处理错误情况
    if (typeof error === 'string') {
      // 检查是否是配置文件不存在的错误
      if (error.includes('配置文件不存在')) {
        errorMessage.value = '配置文件不存在，请检查路径是否正确';
      } else if (error.includes('解析错误')) {
        errorMessage.value = '配置文件格式错误, 请检查XML结构';
      } else {
        errorMessage.value = `操作失败: ${error}`;
      }
    } else if (error instanceof Error) {
      errorMessage.value = `发生错误: ${error.message}`;
      console.error('错误详情:', error);
    } else {
      errorMessage.value = '解析服务器配置时发生未知错误';
      console.error('未知错误:', error);
    }
  }
};
</script>

<style scoped>
.settings-container {
  padding: 40px 20px;
  min-width: 600px;
  margin: 0 auto;
  background-color: #f8f9fa;
  display: flex;
  flex-direction: column;
  justify-content: flex-start;
}

.header-section {
  text-align: center;
  margin-bottom: 40px;
}

.title {
  margin: 0 0 10px 0;
  color: #303133;
  font-size: 32px;
  font-weight: 700;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.subtitle {
  margin: 0;
  color: #909399;
  font-size: 16px;
  font-weight: 400;
}

:deep(.el-form) {
  background: white;
  padding: 40px;
  border-radius: 8px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

:deep(.el-form-item__label) {
  font-size: 16px;
  font-weight: 500;
}

:deep(.el-input__inner) {
  height: 44px;
  font-size: 16px;
}

:deep(.el-button) {
  height: 44px;
  font-size: 16px;
  padding: 12px 24px;
}
</style>