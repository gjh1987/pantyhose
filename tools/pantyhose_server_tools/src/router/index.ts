import { createRouter, createWebHashHistory } from 'vue-router';
import Settings from '../components/Settings.vue';
import MainLayout from '../components/MainLayout.vue';
import { RouteRecordRaw } from 'vue-router';

const routes: RouteRecordRaw[] = [
  // 环境配置页面 - 不显示导航栏
  { path: '/', component: Settings },
  
  // 主应用布局 - 显示导航栏
  {
    path: '/main',
    component: MainLayout
  }
];

const router = createRouter({
  history: createWebHashHistory(),
  routes
});

export default router;