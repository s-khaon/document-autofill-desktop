import { createApp } from "vue";
import App from "./App.vue";
import { attachConsole } from "@tauri-apps/plugin-log";

// 初始化日志并挂载应用
async function initApp() {
  try {
    // 等待日志控制台连接完成
    await attachConsole();
    console.log("Frontend logging attached");
  } catch (e) {
    console.error("Failed to attach console logging:", e);
  }

  createApp(App).mount("#app");
}

initApp();
