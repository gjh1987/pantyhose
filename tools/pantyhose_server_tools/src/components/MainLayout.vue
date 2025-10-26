<template>
  <div class="main-layout">
    <NavBar @tab-change="handleTabChange" />
    <div class="layout-content">
      <ServerList v-show="activeTab === 'servers'" />
      <Docs v-show="activeTab === 'docs'" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import NavBar from './NavBar.vue';
import ServerList from './ServerList.vue';
import Docs from './Docs.vue';

// 当前激活的标签页
const activeTab = ref('servers');

// 处理标签页切换
const handleTabChange = (tab: string) => {
  activeTab.value = tab;
};
</script>

<style scoped>
.main-layout {
  height: 100vh;
  display: flex;
  flex-direction: column;
}

.layout-content {
  flex: 1;
  overflow: auto;
}
</style>