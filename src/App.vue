<script setup lang="ts">
import { ref, reactive, computed, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  open as openDialog,
  save as saveDialog,
  type OpenDialogOptions,
  type SaveDialogOptions,
} from "@tauri-apps/plugin-dialog";
import { Command } from "@tauri-apps/plugin-shell";
import { openPath } from "@tauri-apps/plugin-opener";
import { exists, readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";

// 模板配置数据结构
interface TemplateConfig {
  id: string;
  name: string;
  path: string;
  placeholders: string[];
  defaultValues: Record<string, string>;
  outputDir: string;
  createdAt: string;
  updatedAt: string;
}

// 应用状态
const outputPath = ref<string>("");
const isLoading = ref<boolean>(false);
const errorMsg = ref<string>("");
const successMsg = ref<string>("");
const showConfig = ref<boolean>(false);
const selectedTemplateId = ref<string>("");

// 模板列表
const templates = ref<TemplateConfig[]>([]);

// 当前选中的模板配置
const selectedTemplate = computed(() => {
  return templates.value.find(t => t.id === selectedTemplateId.value) || null;
});

// 表单数据
const formData = reactive<Record<string, string>>({
  // 初始空对象，根据模板动态生成
});

// 占位符列表
const placeholders = computed(() => {
  return selectedTemplate.value?.placeholders || [];
});

// 存储配置文件路径
const CONFIG_FILE = "./templates-config.json";

// 初始化日期数据
const today = new Date();
const currentYear = today.getFullYear().toString();
const currentMonth = (today.getMonth() + 1).toString().padStart(2, "0");
const currentDay = today.getDate().toString().padStart(2, "0");

// 保存配置到本地存储
async function saveConfig() {
  try {
    await writeTextFile(CONFIG_FILE, JSON.stringify(templates.value, null, 2));
  } catch (error) {
    console.error("保存配置失败:", error);
  }
}

// 从本地存储加载配置
async function loadConfig() {
  try {
    const fileExists = await exists(CONFIG_FILE);
    if (fileExists) {
      const content = await readTextFile(CONFIG_FILE);
      templates.value = JSON.parse(content);
      if (templates.value.length > 0) {
        selectedTemplateId.value = templates.value[0].id;
        updateFormData();
      }
    }
  } catch (error) {
    console.error("加载配置失败:", error);
    templates.value = [];
  }
}

// 添加新模板
async function addTemplate() {
  try {
    const options: OpenDialogOptions = {
      title: "选择Word模板",
      filters: [
        {
          name: "Word Documents",
          extensions: ["docx"],
        },
      ],
      multiple: false,
    };

    const selected = await openDialog(options);
    if (selected) {
      const templatePath = Array.isArray(selected) ? selected[0] : selected;
      
      // 提取模板中的占位符
      const placeholders = await extractPlaceholdersFromTemplate(templatePath);
      
      // 创建新的模板配置
      const newTemplate: TemplateConfig = {
        id: `template-${Date.now()}`,
        name: templatePath.split("/").pop() || "新模板",
        path: templatePath,
        placeholders: placeholders,
        defaultValues: {
          // 设置默认日期
          ...(placeholders.includes("年份") && { "年份": currentYear }),
          ...(placeholders.includes("月份") && { "月份": currentMonth }),
          ...(placeholders.includes("日") && { "日": currentDay }),
        },
        outputDir: "",
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      
      // 添加到模板列表
      templates.value.push(newTemplate);
      selectedTemplateId.value = newTemplate.id;
      
      // 更新表单数据
      updateFormData();
      
      // 保存配置
      await saveConfig();
      
      errorMsg.value = "";
      successMsg.value = "模板添加成功！";
    }
  } catch (error) {
    errorMsg.value = `添加模板失败：${error instanceof Error ? error.message : String(error)}`;
  }
}

// 删除模板
async function deleteTemplate(templateId: string) {
  if (confirm("确定要删除这个模板吗？")) {
    templates.value = templates.value.filter(t => t.id !== templateId);
    if (selectedTemplateId.value === templateId) {
      selectedTemplateId.value = templates.value[0]?.id || "";
      updateFormData();
    }
    await saveConfig();
    successMsg.value = "模板删除成功！";
  }
}

// 选择模板文件（已被addTemplate替代）
// async function selectTemplate() {
//   await addTemplate();
// }

// 从模板中提取占位符
async function extractPlaceholdersFromTemplate(templatePath: string): Promise<string[]> {
  try {
    isLoading.value = true;
    errorMsg.value = "";
    
    // 获取Python可执行文件路径
    const pythonExecutable = await invoke<string>("get_python_executable");
    
    console.log("Python可执行文件路径:", pythonExecutable);
    console.log("模板路径:", templatePath);
    
    // 调用Python程序提取占位符，指定python目录作为工作目录
    // 注意：Tauri shell plugin 需要配置允许的命令 scope
    // 这里我们尝试直接执行，但如果是绝对路径，需要在 tauri.conf.json 中配置
    // 或者我们使用 sidecar 模式（生产环境推荐）
    
    // 如果是开发环境且找到了 venv，我们尝试用绝对路径
    // 如果是生产环境，pythonExecutable 应该是 resource 中的路径，或者是系统 python3
    
    console.log("执行命令:", pythonExecutable, ["main.py", templatePath, "", "", "extract"]);
    
    // 我们需要获取 main.py 的绝对路径
    // 从 pythonExecutable 中推断
    // pythonExecutable: .../python/.venv/bin/python3
    // main.py should be in .../python/main.py
    
    let mainPyPath = "main.py";
    let commandName = pythonExecutable;

    if (pythonExecutable.includes(".venv")) {
      // 这是一个简单的推断，假设结构是 standard
      // pythonExecutable: .../python/.venv/bin/python3
      // 我们需要 .../python/main.py
      // 向上 3 级
      const venvBinIndex = pythonExecutable.indexOf(".venv");
      if (venvBinIndex > 0) {
        const pythonDir = pythonExecutable.substring(0, venvBinIndex);
        mainPyPath = pythonDir + "main.py";
      }
      
      // 使用 capabilities 中定义的别名，避免绝对路径匹配问题
      commandName = "venv-python";
    } else {
        // 如果是系统 python3，使用 python-script 别名或者直接 python3
        // 假设 capabilities 中定义了 python-script -> python3
        if (pythonExecutable === "python3") {
            commandName = "python-script";
        }
    }
    
    console.log("main.py 路径:", mainPyPath);
    console.log("Command Name:", commandName);

    const command = Command.create(commandName, [
      mainPyPath, 
      templatePath, 
      "", 
      "", 
      "extract"
    ]);
    
    // 尝试: spawn 选项中可能有 cwd
    const result = await command.execute();
    
    if (result.code === 0) {
      console.log("命令输出:", result.stdout);
      const placeholders = JSON.parse(result.stdout);
      console.log("解析后的占位符:", placeholders);
      return placeholders;
    } else {
      console.error("命令执行失败:", result.stderr || result.stdout);
      throw new Error(result.stderr || result.stdout);
    }
  } catch (error) {
    console.error("提取占位符失败:", error);
    errorMsg.value = `提取占位符失败: ${error instanceof Error ? error.message : String(error)}. 请检查 Python 环境是否包含 python-docx 依赖。`;
    // 如果提取失败，返回空列表
    return [];
  } finally {
    isLoading.value = false;
  }
}

// 更新表单数据
function updateFormData() {
  // 清空当前表单数据
  Object.keys(formData).forEach(key => delete formData[key]);
  
  // 如果有选中的模板，初始化表单数据
  if (selectedTemplate.value) {
    // 初始化默认值
    selectedTemplate.value.placeholders.forEach(placeholder => {
      formData[placeholder] = selectedTemplate.value?.defaultValues[placeholder] || "";
    });
    
    // 设置输出路径
    if (selectedTemplate.value.outputDir) {
      outputPath.value = selectedTemplate.value.outputDir + "/output.docx";
    }
  }
}

// 选择输出文件位置
async function selectOutput() {
  try {
    const options: SaveDialogOptions = {
      title: "保存生成的Word文件",
      filters: [
        {
          name: "Word Documents",
          extensions: ["docx"],
        },
      ],
      defaultPath: selectedTemplate.value?.outputDir ? `${selectedTemplate.value.outputDir}/output.docx` : "output.docx",
    };

    const selected = await saveDialog(options);
    if (selected) {
      outputPath.value = selected;
      errorMsg.value = "";
      successMsg.value = ""; // 重置成功消息，禁用打开文件按钮
    }
  } catch (error) {
    errorMsg.value = `选择保存位置失败：${error instanceof Error ? error.message : String(error)}`;
  }
}

// 选择输出目录
async function selectOutputDir() {
  if (!selectedTemplate.value) return;
  
  try {
    const options: OpenDialogOptions = {
      title: "选择默认输出目录",
      directory: true,
      multiple: false,
    };

    const selected = await openDialog(options);
    if (selected) {
      const dirPath = Array.isArray(selected) ? selected[0] : selected;
      selectedTemplate.value.outputDir = dirPath;
      selectedTemplate.value.updatedAt = new Date().toISOString();
      await saveConfig();
      successMsg.value = "输出目录设置成功！";
    }
  } catch (error) {
    errorMsg.value = `选择输出目录失败：${error instanceof Error ? error.message : String(error)}`;
  }
}

// 更新模板名称
function updateTemplateName(newName: string) {
  if (selectedTemplate.value) {
    selectedTemplate.value.name = newName;
    selectedTemplate.value.updatedAt = new Date().toISOString();
  }
}

// 更新默认值
function updateDefaultValue(placeholder: string, value: string) {
  if (selectedTemplate.value) {
    selectedTemplate.value.defaultValues[placeholder] = value;
    selectedTemplate.value.updatedAt = new Date().toISOString();
  }
}

// 保存模板配置
async function saveTemplateConfig() {
  if (selectedTemplate.value) {
    await saveConfig();
    successMsg.value = "模板配置保存成功！";
    showConfig.value = false;
  }
}

// 生成Word文件
async function generateDocument() {
  try {
    if (!selectedTemplate.value) {
      errorMsg.value = "请先选择模板文件";
      return;
    }

    if (!outputPath.value) {
      errorMsg.value = "请先选择保存位置";
      return;
    }

    isLoading.value = true;
    errorMsg.value = "";
    successMsg.value = "";

    // 获取Python可执行文件路径
    const pythonExecutable = await invoke<string>("get_python_executable");
    
    // 构建命令行参数
    const dataJson = JSON.stringify(formData);
    
    // 我们需要获取 main.py 的绝对路径
    // 从 pythonExecutable 中推断
    let mainPyPath = "main.py";
    let commandName = pythonExecutable;

    if (pythonExecutable.includes(".venv")) {
      // 这是一个简单的推断，假设结构是 standard
      // pythonExecutable: .../python/.venv/bin/python3
      // 我们需要 .../python/main.py
      // 向上 3 级
      const venvBinIndex = pythonExecutable.indexOf(".venv");
      if (venvBinIndex > 0) {
        const pythonDir = pythonExecutable.substring(0, venvBinIndex);
        mainPyPath = pythonDir + "main.py";
      }
      
      // 使用 capabilities 中定义的别名，避免绝对路径匹配问题
      commandName = "venv-python";
    } else {
        // 如果是系统 python3，使用 python-script 别名或者直接 python3
        // 假设 capabilities 中定义了 python-script -> python3
        if (pythonExecutable === "python3") {
            commandName = "python-script";
        }
    }
    
    console.log("执行生成命令:", commandName, [mainPyPath, selectedTemplate.value.path, outputPath.value, "dataJson..."]);

    // 调用Python程序
    const result = await Command.create(commandName, [
      mainPyPath,
      selectedTemplate.value.path, 
      outputPath.value, 
      dataJson,
      "fill"
    ]).execute();
    
    if (result.code === 0) {
      successMsg.value = "文件生成成功！";
    } else {
      errorMsg.value = `生成文件失败：${result.stderr || result.stdout}`;
    }
  } catch (error) {
    errorMsg.value = `生成文件失败：${error instanceof Error ? error.message : String(error)}`;
  } finally {
    isLoading.value = false;
  }
}

// 打开生成的文件
async function openGeneratedFile() {
  if (outputPath.value) {
    try {
      await openPath(outputPath.value);
    } catch (error) {
      errorMsg.value = `打开文件失败：${error instanceof Error ? error.message : String(error)}`;
    }
  }
}

// 生命周期钩子
onMounted(() => {
  // 加载保存的配置
  loadConfig();
});

// 监听选中模板变化
watch(selectedTemplateId, () => {
  updateFormData();
  successMsg.value = ""; // 重置成功消息，禁用打开文件按钮
});
</script>

<template>
  <main class="container">
    <h1>文档自动填充工具</h1>
    <p>将数据自动填充到Word模板中，生成新的Word文件</p>

    <!-- 错误信息 -->
    <div v-if="errorMsg" class="message error">{{ errorMsg }}</div>
    <!-- 成功信息 -->
    <div v-if="successMsg" class="message success">{{ successMsg }}</div>

    <!-- 模板管理 -->
    <div class="form-section">
      <div class="section-header">
        <h2>1. 模板管理</h2>
        <button @click="addTemplate" :disabled="isLoading" class="secondary">
          添加模板
        </button>
      </div>
      
      <div class="template-list" v-if="templates.length > 0">
        <div 
          v-for="template in templates" 
          :key="template.id" 
          class="template-item"
          :class="{ 'selected': template.id === selectedTemplateId }"
        >
          <div class="template-info">
            <h3>{{ template.name }}</h3>
            <p class="template-path">{{ template.path }}</p>
            <p class="template-meta">
              {{ template.placeholders.length }}个占位符 · 
              创建于: {{ new Date(template.createdAt).toLocaleString() }}
            </p>
          </div>
          <div class="template-actions">
            <button @click="selectedTemplateId = template.id" class="secondary">
              选择
            </button>
            <button @click="showConfig = true" class="secondary">
              配置
            </button>
            <button @click="deleteTemplate(template.id)" class="danger">
              删除
            </button>
          </div>
        </div>
      </div>
      
      <div v-else class="empty-state">
        <p>暂无模板，请点击"添加模板"按钮添加</p>
      </div>
    </div>

    <!-- 模板配置 -->
    <div class="form-section" v-if="showConfig && selectedTemplate">
      <div class="section-header">
        <h2>模板配置</h2>
        <button @click="showConfig = false" class="secondary">
          关闭
        </button>
      </div>
      
      <div class="config-form">
        <div class="form-item">
          <label for="template-name">模板名称</label>
          <input
            id="template-name"
            type="text"
            v-model="selectedTemplate.name"
            @input="updateTemplateName(selectedTemplate.name)"
          />
        </div>
        
        <div class="form-item">
          <label for="output-dir">默认输出目录</label>
          <div class="file-selector">
            <input
              id="output-dir"
              type="text"
              v-model="selectedTemplate.outputDir"
              readonly
            />
            <button @click="selectOutputDir">
              浏览
            </button>
          </div>
        </div>
        
        <h3>默认值设置</h3>
        <div class="form-grid">
          <div
            v-for="placeholder in selectedTemplate.placeholders"
            :key="placeholder"
            class="form-item"
          >
            <label :for="`default-${placeholder}`">{{ placeholder }}</label>
            <input
              :id="`default-${placeholder}`"
              type="text"
              v-model="selectedTemplate.defaultValues[placeholder]"
              @input="updateDefaultValue(placeholder, selectedTemplate.defaultValues[placeholder])"
              placeholder="设置默认值"
            />
          </div>
        </div>
        
        <div class="action-buttons">
          <button @click="saveTemplateConfig" class="primary">
            保存配置
          </button>
        </div>
      </div>
    </div>

    <!-- 填写数据 -->
    <div class="form-section" v-if="placeholders.length > 0">
      <h2>2. 填写数据</h2>
      <div class="form-grid">
        <div
          v-for="placeholder in placeholders"
          :key="placeholder"
          class="form-item"
        >
          <label :for="placeholder">{{ placeholder }}</label>
          <input
            :id="placeholder"
            v-model="formData[placeholder]"
            type="text"
            :placeholder="`请输入${placeholder}`"
            :disabled="isLoading"
          />
        </div>
      </div>
    </div>

    <!-- 保存位置 -->
    <div class="form-section" v-if="placeholders.length > 0">
      <h2>3. 保存位置</h2>
      <div class="file-selector">
        <input
          type="text"
          v-model="outputPath"
          placeholder="请选择输出文件位置"
          readonly
        />
        <button @click="selectOutput" :disabled="isLoading">
          浏览
        </button>
      </div>
    </div>

    <!-- 生成文件 -->
    <div class="form-section" v-if="placeholders.length > 0">
      <h2>4. 生成文件</h2>
      <div class="action-buttons">
        <button
          class="primary"
          @click="generateDocument"
          :disabled="isLoading || !selectedTemplate || !outputPath"
        >
          <span v-if="isLoading">生成中...</span>
          <span v-else>生成文件</span>
        </button>
        <button
          @click="openGeneratedFile"
          :disabled="isLoading || !successMsg"
        >
          打开文件
        </button>
      </div>
    </div>
  </main>
</template>

<style scoped>
.container {
  max-width: 900px;
  margin: 0 auto;
  padding: 2rem;
}

.form-section {
  margin: 2rem 0;
  padding: 1.5rem;
  background-color: #ffffff;
  border-radius: 8px;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.form-section h2 {
  margin-top: 0;
  margin-bottom: 1rem;
  font-size: 1.2rem;
  color: #333;
  border-bottom: 1px solid #e0e0e0;
  padding-bottom: 0.5rem;
}

.form-section h3 {
  margin: 1.5rem 0 1rem 0;
  font-size: 1rem;
  color: #555;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
}

.section-header h2 {
  margin: 0;
  border: none;
  padding: 0;
}

.file-selector {
  display: flex;
  gap: 0.5rem;
}

.file-selector input {
  flex: 1;
  background-color: #f5f5f5;
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 1rem;
}

.form-item {
  display: flex;
  flex-direction: column;
}

.form-item label {
  margin-bottom: 0.5rem;
  font-weight: 500;
  color: #555;
}

.action-buttons {
  display: flex;
  gap: 1rem;
}

.action-buttons button.primary {
  background-color: #396cd8;
  color: white;
}

.action-buttons button.primary:hover {
  background-color: #2d53a5;
  border-color: #2d53a5;
}

.action-buttons button.secondary {
  background-color: #e0e0e0;
  color: #333;
}

.action-buttons button.secondary:hover {
  background-color: #d0d0d0;
  border-color: #396cd8;
}

.action-buttons button.danger {
  background-color: #f44336;
  color: white;
}

.action-buttons button.danger:hover {
  background-color: #d32f2f;
  border-color: #d32f2f;
}

.message {
  padding: 1rem;
  margin: 1rem 0;
  border-radius: 4px;
  font-weight: 500;
}

.message.error {
  background-color: #fee;
  color: #d32f2f;
  border: 1px solid #fcc;
}

.message.success {
  background-color: #efe;
  color: #2e7d32;
  border: 1px solid #cfc;
}

/* 模板管理样式 */
.template-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.template-item {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  padding: 1rem;
  background-color: #f9f9f9;
  border: 1px solid #e0e0e0;
  border-radius: 8px;
  transition: all 0.2s ease;
}

.template-item:hover {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.template-item.selected {
  border-color: #396cd8;
  background-color: #f0f4ff;
}

.template-info {
  flex: 1;
}

.template-info h3 {
  margin: 0 0 0.5rem 0;
  font-size: 1rem;
  color: #333;
}

.template-path {
  margin: 0 0 0.5rem 0;
  font-size: 0.85rem;
  color: #666;
  word-break: break-all;
}

.template-meta {
  margin: 0;
  font-size: 0.8rem;
  color: #888;
}

.template-actions {
  display: flex;
  gap: 0.5rem;
  flex-direction: column;
}

.empty-state {
  padding: 2rem;
  text-align: center;
  color: #888;
  background-color: #f9f9f9;
  border-radius: 8px;
}

/* 配置表单样式 */
.config-form {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.config-form .form-item {
  margin-bottom: 1rem;
}

.config-form .form-grid {
  margin-top: 1rem;
}

/* 按钮样式 */
button {
  padding: 0.5rem 1rem;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.9rem;
  transition: all 0.2s ease;
}

button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

@media (max-width: 768px) {
  .container {
    padding: 1rem;
  }

  .file-selector {
    flex-direction: column;
  }

  .action-buttons {
    flex-direction: column;
  }
  
  .template-item {
    flex-direction: column;
    gap: 1rem;
  }
  
  .template-actions {
    flex-direction: row;
    justify-content: flex-start;
  }
}
</style>

<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

body {
  margin: 0;
  padding: 0;
  min-height: 100vh;
}

.container {
  margin: 0;
  padding: 2rem;
  display: flex;
  flex-direction: column;
  justify-content: flex-start;
  text-align: left;
}

input,
button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

button {
  cursor: pointer;
}

button:hover:not(:disabled) {
  border-color: #396cd8;
}

button:active:not(:disabled) {
  border-color: #396cd8;
  background-color: #e8e8e8;
}

button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

input,
button {
  outline: none;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }

  .form-section {
    background-color: #3f3f3f;
  }

  .form-section h2 {
    border-bottom-color: #555;
  }

  .file-selector input {
    background-color: #4f4f4f;
  }

  input,
  button {
    color: #ffffff;
    background-color: #0f0f0f98;
  }

  button:active:not(:disabled) {
    background-color: #0f0f0f69;
  }
}
</style>