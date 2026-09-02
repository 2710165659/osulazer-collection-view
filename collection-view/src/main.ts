import { createApp } from "vue";
import App from "./App.vue";
import ElementPlus from "element-plus";
import "element-plus/dist/index.css";
import zhCn from "element-plus/es/locale/lang/zh-cn";
import { createPinia } from "pinia";

const pinia = createPinia(); // 全局状态由单一 Pinia 实例管理。
createApp(App)
  .use(ElementPlus, { locale: zhCn }) // Element Plus 的分页和无障碍说明统一使用中文。
  .use(pinia)
  .mount("#app");
