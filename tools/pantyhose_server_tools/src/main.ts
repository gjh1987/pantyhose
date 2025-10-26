import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import router from './router'
import './utils/NetClientManager' // 初始化网络客户端管理器

const pinia = createPinia()

createApp(App)
  .use(pinia)
  .use(ElementPlus)
  .use(router)
  .mount('#app')
