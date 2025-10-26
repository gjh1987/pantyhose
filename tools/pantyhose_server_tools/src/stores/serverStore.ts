import { reactive } from 'vue';

// 定义服务器信息接口
export interface ServerInfo {
  id: string;
  name?: string;
  back_tcp_port?: number;
  front_tcp_port?: number;
  front_ws_port?: number;
  type?: string;
  isRunning?: boolean;
  checked?: boolean;
}

// 定义服务器组接口
export interface ServerGroup {
  id: string;
  name: string;
  type: 'group';
  children: ServerInfo[];
}

// 全局服务器状态
export const serverStore = reactive({
  // 服务器组数据
  serverGroups: [] as ServerGroup[],
  
  // 服务器日志数据 (key: serverId, value: log lines)
  serverLogs: {} as Record<string, string[]>,
  
  // 配置参数
  configFileName: 'config.xml' as string,
  executableName: 'pantyhose.exe' as string,
  
  // 设置服务器数据
  setServerGroups(groups: ServerGroup[]) {
    this.serverGroups.splice(0, this.serverGroups.length, ...groups);
  },
  
  // 获取服务器数据
  getServerGroups(): ServerGroup[] {
    return this.serverGroups;
  },
  
  // 清空服务器数据
  clearServerGroups() {
    this.serverGroups.splice(0, this.serverGroups.length);
  },
  
  // 检查是否有服务器数据
  hasServerData(): boolean {
    return this.serverGroups.length > 0;
  },
  
  // 更新服务器运行状态
  updateServerStatus(serverId: string, isRunning: boolean) {
    for (const group of this.serverGroups) {
      const server = group.children.find(s => s.id === serverId);
      if (server) {
        server.isRunning = isRunning;
        break;
      }
    }
  },
  
  // 获取运行中的服务器数量
  getRunningCount(group: ServerGroup): number {
    return group.children.filter(server => server.isRunning).length;
  },
  
  // 设置配置参数
  setConfig(configFileName: string, executableName: string) {
    this.configFileName = configFileName;
    this.executableName = executableName;
  },
  
  // 获取配置参数
  getConfig() {
    return {
      configFileName: this.configFileName,
      executableName: this.executableName
    };
  },
  
  // 服务器路径
  serverPath: '.\\bin' as string,
  
  // Proto文件路径
  protoPath: 'tools\\proto\\config' as string,
  
  // 获取所有服务器
  getServers(): ServerInfo[] {
    const servers: ServerInfo[] = [];
    for (const group of this.serverGroups) {
      servers.push(...group.children);
    }
    return servers;
  },
  
  // 检查服务器是否在运行
  isServerRunning(serverId: string): boolean {
    for (const group of this.serverGroups) {
      const server = group.children.find(s => s.id === serverId);
      if (server) {
        return server.isRunning || false;
      }
    }
    return false;
  },
  
  // 设置服务器日志（累加新日志）
  setServerLogs(serverId: string, logs: string[]) {
    if (!this.serverLogs[serverId]) {
      this.serverLogs[serverId] = [];
    }
    // 累加新日志
    this.serverLogs[serverId].push(...logs);
    
    // 限制日志数量，保留最新的 1000 条
    const maxLogs = 1000;
    if (this.serverLogs[serverId].length > maxLogs) {
      this.serverLogs[serverId] = this.serverLogs[serverId].slice(-maxLogs);
    }
  },
  
  // 获取服务器日志
  getServerLogs(serverId: string): string[] {
    return this.serverLogs[serverId] || [];
  },
  
  // 清空单个服务器日志（只清空显示）
  clearServerLogs(serverId: string) {
    this.serverLogs[serverId] = [];
  },
  
  // 清空所有服务器日志
  clearAllServerLogs() {
    this.serverLogs = {};
  }
});

// 导出 useServerStore 函数用于组件中使用
export function useServerStore() {
  return serverStore;
}