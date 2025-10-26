<template>
  <div class="docs-container">
    <div class="page-header">
      <h2>使用说明</h2>
      <p>服务器管理工具使用指南</p>
    </div>
    
    <div class="content-area">
      <el-card class="docs-card">
        <template #header>
          <div class="card-header">
            <el-icon><Document /></el-icon>
            <span>功能介绍</span>
          </div>
        </template>
        
        <div class="docs-content">
          <h3>🖥️ 服务器管理</h3>
          <ul>
            <li>查看服务器状态和配置信息</li>
            <li>启动/停止服务器</li>
            <li>监控服务器运行状态</li>
            <li>管理服务器组和分类</li>
          </ul>

          <h3>⚙️ 配置管理</h3>
          <ul>
            <li>设置服务器目录路径</li>
            <li>解析XML配置文件</li>
            <li>自动保存配置信息</li>
          </ul>

          <h3>🎮 GM功能</h3>
          <ul>
            <li>游戏管理员工具（开发中）</li>
            <li>玩家管理功能（开发中）</li>
            <li>游戏数据管理（开发中）</li>
          </ul>

          <h3>📋 使用步骤</h3>
          <ol>
            <li>在首页设置服务器目录路径</li>
            <li>点击"开始"按钮解析配置文件</li>
            <li>在服务器列表中查看和管理服务器</li>
            <li>使用分割器调整界面布局</li>
          </ol>
        </div>
      </el-card>

      <el-card class="docs-card">
        <template #header>
          <div class="card-header">
            <el-icon><QuestionFilled /></el-icon>
            <span>常见问题</span>
          </div>
        </template>
        
        <div class="docs-content">
          <h4>Q: 配置文件不存在怎么办？</h4>
          <p>A: 请检查服务器目录路径是否正确，确保目录下存在 config.xml 文件。</p>

          <h4>Q: 如何调整界面布局？</h4>
          <p>A: 拖拽左右面板之间的分割器可以调整面板宽度。</p>

          <h4>Q: 服务器启动失败怎么办？</h4>
          <p>A: 检查端口是否被占用，确保服务器文件完整。</p>
        </div>
      </el-card>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Document, QuestionFilled } from '@element-plus/icons-vue';
</script>

<style scoped>
.docs-container {
  padding: 30px;
  background-color: #f8f9fa;
  min-height: calc(100vh - 60px);
}

.page-header {
  background: white;
  padding: 30px;
  border-radius: 8px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  margin-bottom: 20px;
}

.page-header h2 {
  margin: 0 0 8px 0;
  color: #303133;
  font-size: 28px;
  font-weight: 600;
}

.page-header p {
  margin: 0;
  color: #909399;
  font-size: 16px;
}

.content-area {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
}

.docs-card {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.card-header {
  display: flex;
  align-items: center;
  font-size: 18px;
  font-weight: 600;
  color: #303133;
}

.card-header .el-icon {
  margin-right: 8px;
  font-size: 20px;
}

.docs-content {
  line-height: 1.6;
}

.docs-content h3 {
  color: #409eff;
  margin: 20px 0 10px 0;
  font-size: 18px;
}

.docs-content h4 {
  color: #303133;
  margin: 15px 0 8px 0;
  font-size: 16px;
}

.docs-content ul,
.docs-content ol {
  padding-left: 20px;
  margin: 10px 0;
}

.docs-content li {
  margin: 5px 0;
  color: #606266;
}

.docs-content p {
  color: #606266;
  margin: 8px 0;
}

@media (max-width: 768px) {
  .content-area {
    grid-template-columns: 1fr;
  }
}
</style>