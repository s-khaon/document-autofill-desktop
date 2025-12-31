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
import { appDataDir } from "@tauri-apps/api/path";

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
const selectedTemplateId = ref<string>("");
const currentStep = ref<1 | 2 | 3 | 4>(1);
const generatedOutputPath = ref<string>("");
const hasEditedForm = ref<boolean>(false);

// 功能模式：single（单文件生成）或 batch（批量生成）
const functionMode = ref<'single' | 'batch'>('single');

// 批量处理相关状态
const batchData = ref<any[]>([]);
const batchOutputDir = ref<string>("");
const filenameField = ref<string>("");
const importedFilePath = ref<string>("");

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

const canGoStep2 = computed(() => !!selectedTemplate.value);
const canGoStep3 = computed(() => !!selectedTemplate.value && placeholders.value.length > 0);
const canGoStep4 = computed(() => canGoStep3.value);

function canGoToStep(step: 1 | 2 | 3 | 4) {
  if (step === 1) return true;
  if (step === 2) return canGoStep2.value;
  if (step === 3) return canGoStep3.value; // 允许从步骤1直接跳到步骤3
  return canGoStep4.value;
}

async function goToStep(step: 1 | 2 | 3 | 4) {
  if (!canGoToStep(step)) return;

  if (currentStep.value === 2 && step !== 2 && selectedTemplate.value) {
    await saveConfig();
  }

  currentStep.value = step;
}

async function nextStep() {
  if (currentStep.value === 1) {
    await goToStep(3); // 从步骤1直接跳到步骤3
    return;
  }
  if (currentStep.value === 2) {
    if (!hasEditedForm.value) updateFormData();
    await saveConfig();
    await goToStep(3);
    return;
  }
  if (currentStep.value === 3) {
    await goToStep(4);
  }
}

async function prevStep() {
  if (currentStep.value === 2) {
    await goToStep(1);
    return;
  }
  if (currentStep.value === 3) {
    await goToStep(1); // 从步骤3返回时回到步骤1，跳过步骤2
    return;
  }
  if (currentStep.value === 4) {
    await goToStep(3);
  }
}

function markFormEdited() {
  hasEditedForm.value = true;
  generatedOutputPath.value = "";
}

// 存储配置文件路径
const CONFIG_FILE = ref<string>("");

// 初始化日期数据
const today = new Date();
const currentYear = today.getFullYear().toString();
const currentMonth = (today.getMonth() + 1).toString().padStart(2, "0");
const currentDay = today.getDate().toString().padStart(2, "0");

// 保存配置到本地存储
async function saveConfig() {
  try {
    if (!CONFIG_FILE.value) {
      console.error("配置文件路径未初始化");
      return;
    }
    await writeTextFile(CONFIG_FILE.value, JSON.stringify(templates.value, null, 2));
  } catch (error) {
    console.error("保存配置失败:", error);
  }
}

// 从本地存储加载配置
async function loadConfig() {
  try {
    if (!CONFIG_FILE.value) {
      console.error("配置文件路径未初始化");
      return;
    }
    const fileExists = await exists(CONFIG_FILE.value);
    if (fileExists) {
      const content = await readTextFile(CONFIG_FILE.value);
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
      hasEditedForm.value = false;
      generatedOutputPath.value = "";
      currentStep.value = 1;
      
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
      generatedOutputPath.value = "";
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
      if (generatedOutputPath.value && outputPath.value.startsWith(dirPath) === false) {
        generatedOutputPath.value = "";
      }
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
  }
}

// 导出Excel模板
async function exportExcelTemplate() {
  try {
    if (!selectedTemplate.value) {
      errorMsg.value = "请先选择模板";
      return;
    }

    const { utils, writeFile } = await import('xlsx');
    
    // 创建工作表数据
    const headers = selectedTemplate.value.placeholders;
    const worksheetData = [headers];
    
    // 创建工作簿
    const workbook = utils.book_new();
    const worksheet = utils.aoa_to_sheet(worksheetData);
    
    // 添加工作表到工作簿
    utils.book_append_sheet(workbook, worksheet, '模板数据');
    
    // 选择保存位置
    const options: SaveDialogOptions = {
      title: "导出Excel模板",
      filters: [
        {
          name: "Excel Files",
          extensions: ["xlsx"],
        },
      ],
      defaultPath: `${selectedTemplate.value.name.replace('.docx', '')}_模板.xlsx`,
    };

    const selected = await saveDialog(options);
    if (selected) {
      // 生成Excel文件
      isLoading.value = true;
      
      // 使用xlsx库写入文件
      // 注意：writeFile函数在浏览器环境中会直接下载，但在Tauri中需要特殊处理
      // 我们需要先将工作簿转换为二进制数据，然后使用fs写入文件
      const excelData = writeFile(workbook, selected, {
        bookType: "xlsx",
        type: "buffer"
      });
      
      // 使用Tauri的fs API写入文件
      await writeTextFile(selected, new TextDecoder().decode(excelData as ArrayBuffer));
      
      successMsg.value = `Excel模板导出成功！保存位置：${selected}`;
    }
  } catch (error) {
    errorMsg.value = `导出Excel模板失败：${error instanceof Error ? error.message : String(error)}`;
  } finally {
    isLoading.value = false;
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
      generatedOutputPath.value = outputPath.value;
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
  const pathToOpen = generatedOutputPath.value || outputPath.value;
  if (pathToOpen) {
    try {
      await openPath(pathToOpen);
    } catch (error) {
      errorMsg.value = `打开文件失败：${error instanceof Error ? error.message : String(error)}`;
    }
  }
}

// 选择批量输出目录
async function selectBatchOutputDir() {
  try {
    const options: OpenDialogOptions = {
      title: "选择输出目录",
      directory: true,
      multiple: false,
    };

    const selected = await openDialog(options);
    if (selected) {
      const dirPath = Array.isArray(selected) ? selected[0] : selected;
      batchOutputDir.value = dirPath;
      errorMsg.value = "";
    }
  } catch (error) {
    errorMsg.value = `选择输出目录失败：${error instanceof Error ? error.message : String(error)}`;
  }
}

// 打开批量输出目录
async function openBatchOutputDir() {
  if (batchOutputDir.value) {
    try {
      await openPath(batchOutputDir.value);
    } catch (error) {
      errorMsg.value = `打开目录失败：${error instanceof Error ? error.message : String(error)}`;
    }
  }
}

// 导入Excel数据
async function importExcelData() {
  try {
    if (!selectedTemplate.value) {
      errorMsg.value = "请先选择模板";
      return;
    }

    const options: OpenDialogOptions = {
      title: "导入Excel数据",
      filters: [
        {
          name: "Excel Files",
          extensions: ["xlsx", "xls"],
        },
      ],
      multiple: false,
    };

    const selected = await openDialog(options);
    if (selected) {
      const filePath = Array.isArray(selected) ? selected[0] : selected;
      importedFilePath.value = filePath;
      isLoading.value = true;
      
      const { read, utils } = await import('xlsx');
      
      // 读取文件内容
      const fileContent = await readTextFile(filePath);
      
      // 解析Excel文件
      const workbook = read(new Uint8Array(Array.from(fileContent).map(char => char.charCodeAt(0))), {
        type: 'array'
      });
      
      // 获取第一个工作表
      const worksheetName = workbook.SheetNames[0];
      const worksheet = workbook.Sheets[worksheetName];
      
      // 转换为JSON格式
      const jsonData = utils.sheet_to_json(worksheet, {
        header: selectedTemplate.value.placeholders
      });
      
      // 过滤掉标题行（如果有）
      batchData.value = jsonData.filter((row: any, index: number) => {
        // 检查第一行是否为标题行
        if (index === 0 && selectedTemplate.value) {
          // 如果第一行的第一个单元格等于第一个标题，则认为是标题行，过滤掉
          const firstCell = row[selectedTemplate.value.placeholders[0]];
          return firstCell !== selectedTemplate.value.placeholders[0];
        }
        return true;
      });
      
      successMsg.value = `成功导入 ${batchData.value.length} 条记录`;
      errorMsg.value = "";
    }
  } catch (error) {
    errorMsg.value = `导入Excel数据失败：${error instanceof Error ? error.message : String(error)}`;
    batchData.value = [];
  } finally {
    isLoading.value = false;
  }
}

// 批量生成文档
async function generateBatchDocuments() {
  try {
    if (!selectedTemplate.value) {
      errorMsg.value = "请先选择模板文件";
      return;
    }

    if (!batchOutputDir.value) {
      errorMsg.value = "请先选择输出目录";
      return;
    }

    if (batchData.value.length === 0) {
      errorMsg.value = "请先导入数据";
      return;
    }

    isLoading.value = true;
    errorMsg.value = "";
    successMsg.value = "";

    // 获取Python可执行文件路径
    const pythonExecutable = await invoke<string>("get_python_executable");
    
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
    
    // 批量生成文件
    let successCount = 0;
    let failureCount = 0;
    
    for (let i = 0; i < batchData.value.length; i++) {
      const rowData = batchData.value[i];
      
      // 生成文件名
      let filename = `output_${i+1}.docx`;
      if (filenameField.value && rowData[filenameField.value]) {
        filename = `${rowData[filenameField.value]}.docx`;
      }
      
      // 构建输出路径
      const outputFilePath = `${batchOutputDir.value}/${filename}`;
      
      // 构建命令行参数
      const dataJson = JSON.stringify(rowData);
      
      try {
        // 调用Python程序
        const result = await Command.create(commandName, [
          mainPyPath,
          selectedTemplate.value.path, 
          outputFilePath, 
          dataJson,
          "fill"
        ]).execute();
        
        if (result.code === 0) {
          successCount++;
        } else {
          failureCount++;
          console.error(`生成文件失败 (${i+1}/${batchData.value.length})：${result.stderr || result.stdout}`);
        }
      } catch (error) {
        failureCount++;
        console.error(`生成文件失败 (${i+1}/${batchData.value.length})：${error instanceof Error ? error.message : String(error)}`);
      }
    }
    
    successMsg.value = `批量生成完成！成功：${successCount} 个，失败：${failureCount} 个`;
    
    // 打开输出目录
    await openPath(batchOutputDir.value);
    
  } catch (error) {
    errorMsg.value = `批量生成失败：${error instanceof Error ? error.message : String(error)}`;
  } finally {
    isLoading.value = false;
  }
}

// 生命周期钩子
onMounted(async () => {
  // 初始化配置文件路径
  try {
    const dataDir = await appDataDir();
    CONFIG_FILE.value = `${dataDir}templates-config.json`;
    console.log("配置文件路径:", CONFIG_FILE.value);
  } catch (error) {
    console.error("获取应用数据目录失败:", error);
  }
  
  // 加载保存的配置
  loadConfig();
});

// 监听选中模板变化
watch(selectedTemplateId, () => {
  updateFormData();
  hasEditedForm.value = false;
  generatedOutputPath.value = "";
  currentStep.value = 1;
});
</script>

<template>
  <main class="app">
    <header class="app-header">
      <div class="title">
        <h1>文档自动填充</h1>
        <p>选择模板 → 填写数据 → 生成并打开（默认值仅在配置模板时设置）</p>
      </div>
      <div class="header-actions">
        <div class="mode-switch">
          <label>
            <input 
              type="radio" 
              value="single" 
              v-model="functionMode" 
              name="functionMode"
            />
            单文件生成
          </label>
          <label>
            <input 
              type="radio" 
              value="batch" 
              v-model="functionMode" 
              name="functionMode"
            />
            批量生成
          </label>
        </div>
        <button @click="addTemplate" :disabled="isLoading" class="secondary">
          添加模板
        </button>
      </div>
    </header>

    <nav class="stepper" aria-label="步骤">
      <button
        class="step"
        :class="{ active: currentStep === 1 }"
        @click="goToStep(1)"
      >
        <span class="step-index">1</span>
        <span class="step-label">选择模板</span>
      </button>
      <button
        class="step"
        :class="{ active: currentStep === 2 }"
        :disabled="!canGoToStep(2)"
        @click="goToStep(2)"
      >
        <span class="step-index">2</span>
        <span class="step-label">配置默认值</span>
      </button>
      <button
        class="step"
        :class="{ active: currentStep === 3 }"
        :disabled="!canGoToStep(3)"
        @click="goToStep(3)"
      >
        <span class="step-index">3</span>
        <span class="step-label">填写数据</span>
      </button>
      <button
        class="step"
        :class="{ active: currentStep === 4 }"
        :disabled="!canGoToStep(4)"
        @click="goToStep(4)"
      >
        <span class="step-index">4</span>
        <span class="step-label">生成文件</span>
      </button>
    </nav>

    <!-- 错误信息 -->
    <div v-if="errorMsg" class="message error">{{ errorMsg }}</div>
    <!-- 成功信息 -->
    <div v-if="successMsg" class="message success">{{ successMsg }}</div>

    <section class="card" v-if="currentStep === 1">
      <div class="card-header">
        <h2>选择模板</h2>
        <p class="muted">从历史模板中选择，或新增一个 docx 模板</p>
      </div>

      <div class="template-list" v-if="templates.length > 0">
        <button
          v-for="template in templates"
          :key="template.id"
          type="button"
          class="template-row"
          :class="{ selected: template.id === selectedTemplateId }"
          @click="selectedTemplateId = template.id"
        >
          <div class="template-row-main">
            <div class="template-row-title">
              <span class="template-name">{{ template.name }}</span>
              <span class="pill">{{ template.placeholders.length }} 个占位符</span>
            </div>
            <div class="template-path">{{ template.path }}</div>
            <div class="template-meta">
              创建于 {{ new Date(template.createdAt).toLocaleString() }}
            </div>
          </div>
          <div class="template-row-actions" @click.stop>
            <button class="primary" @click="selectedTemplateId = template.id; goToStep(3)">
              使用
            </button>
            <button class="secondary" @click="selectedTemplateId = template.id; goToStep(2)">
              配置
            </button>
            <button class="danger" @click="deleteTemplate(template.id)">
              删除
            </button>
          </div>
        </button>
      </div>

      <div v-else class="empty-state">
        <p>暂无模板。点击右上角“添加模板”开始。</p>
      </div>

      <div class="footer-actions">
        <div class="left">
          <div v-if="selectedTemplate" class="summary">
            已选择：<span class="strong">{{ selectedTemplate.name }}</span>
          </div>
        </div>
        <div class="right">
          <button class="primary" @click="nextStep" :disabled="isLoading || !canGoToStep(2)">
            下一步
          </button>
        </div>
      </div>
    </section>

    <section class="card" v-else-if="currentStep === 2 && selectedTemplate">
      <div class="card-header">
        <h2>配置默认值</h2>
        <p class="muted">设置模板名称、默认输出目录与默认填充值</p>
      </div>

      <div class="two-col">
        <div class="panel">
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
              <button @click="selectOutputDir" :disabled="isLoading">
                浏览
              </button>
            </div>
          </div>

          <div class="hint">
            <div class="hint-title">模板信息</div>
            <div class="hint-body">
              <div class="kv">
                <span class="k">路径</span>
                <span class="v">{{ selectedTemplate.path }}</span>
              </div>
              <div class="kv">
                <span class="k">占位符</span>
                <span class="v">{{ selectedTemplate.placeholders.length }} 个</span>
              </div>
            </div>
          </div>
        </div>

        <div class="panel">
          <h3 class="panel-title">默认值设置</h3>
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
        </div>
      </div>

      <div class="footer-actions">
        <div class="left">
          <button class="secondary" @click="prevStep" :disabled="isLoading">上一步</button>
        </div>
        <div class="right">
          <button class="secondary" @click="saveTemplateConfig" :disabled="isLoading">保存配置</button>
          <button class="primary" @click="nextStep" :disabled="isLoading || !canGoToStep(3)">下一步</button>
        </div>
      </div>
    </section>

    <section class="card" v-else-if="currentStep === 3 && placeholders.length > 0">
      <div class="card-header">
        <h2>{{ functionMode === 'single' ? '填写数据' : '批量处理配置' }}</h2>
        <p class="muted">
          {{ functionMode === 'single' ? '按占位符逐项填写，本次填写不会影响模板默认值' : '导入Excel数据或导出模板进行填写' }}
        </p>
      </div>

      <!-- 单文件模式：表单填写 -->
      <div v-if="functionMode === 'single'" class="form-grid">
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
            @input="markFormEdited"
          />
        </div>
      </div>

      <!-- 批量模式：Excel导入导出 -->
      <div v-else-if="functionMode === 'batch'" class="batch-section">
        <div class="panel">
          <h3 class="panel-title">批量数据操作</h3>
          
          <div class="batch-actions">
            <div class="batch-action-group">
              <h4>1. 准备数据</h4>
              <div class="action-buttons">
                <button @click="exportExcelTemplate" :disabled="isLoading" class="secondary">
                  导出Excel模板
                </button>
                <button @click="importExcelData" :disabled="isLoading" class="primary">
                  导入Excel数据
                </button>
              </div>
              <div v-if="importedFilePath" class="imported-info">
                已导入：{{ importedFilePath }}
              </div>
            </div>
            
            <div class="batch-action-group">
              <h4>2. 数据预览</h4>
              <div v-if="batchData.length > 0" class="data-preview">
                <p>共导入 {{ batchData.length }} 条记录</p>
                <div class="table-container">
                  <table class="preview-table">
                    <thead>
                      <tr>
                        <th v-for="(header, index) in placeholders" :key="index">{{ header }}</th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr v-for="(row, rowIndex) in batchData.slice(0, 3)" :key="rowIndex">
                        <td v-for="(cell, cellIndex) in placeholders" :key="cellIndex">
                          {{ row[cell] || '' }}
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </div>
                <p v-if="batchData.length > 3" class="more-rows">... 还有 {{ batchData.length - 3 }} 条记录</p>
              </div>
              <div v-else class="empty-data">
                暂无数据，请先导入Excel文件
              </div>
            </div>
            
            <div class="batch-action-group">
              <h4>3. 文件名设置</h4>
              <div class="form-item">
                <label for="filename-field">选择作为文件名的字段</label>
                <select 
                  id="filename-field" 
                  v-model="filenameField" 
                  :disabled="isLoading || batchData.length === 0"
                  class="form-select"
                >
                  <option value="">请选择...</option>
                  <option v-for="placeholder in placeholders" :key="placeholder" :value="placeholder">
                    {{ placeholder }}
                  </option>
                </select>
                <p class="muted" style="margin-top: 5px;">生成的文件名将使用该字段的值</p>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="footer-actions">
        <div class="left">
          <button class="secondary" @click="prevStep" :disabled="isLoading">上一步</button>
        </div>
        <div class="right">
          <button class="primary" @click="nextStep" :disabled="isLoading || !canGoToStep(4)">下一步</button>
        </div>
      </div>
    </section>

    <section class="card" v-else-if="currentStep === 4 && placeholders.length > 0">
      <div class="card-header">
        <h2>{{ functionMode === 'single' ? '生成文件' : '批量生成' }}</h2>
        <p class="muted">
          {{ functionMode === 'single' ? '选择保存位置，生成后可直接打开' : '选择输出目录，批量生成文件' }}
        </p>
      </div>

      <div class="panel">
        <!-- 单文件模式：输出文件选择 -->
        <div v-if="functionMode === 'single'" class="form-item">
          <label>保存位置</label>
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

        <!-- 批量模式：输出目录选择 -->
        <div v-else-if="functionMode === 'batch'" class="batch-output-section">
          <div class="form-item">
            <label>输出目录</label>
            <div class="file-selector">
              <input
                type="text"
                v-model="batchOutputDir"
                placeholder="请选择输出目录"
                readonly
              />
              <button @click="selectBatchOutputDir" :disabled="isLoading">
                浏览
              </button>
            </div>
          </div>
          
          <div class="form-item">
            <label>文件名设置</label>
            <div class="filename-info">
              <p>文件名规则：{{ filenameField ? `[${filenameField}]` : '默认文件名' }}.docx</p>
              <p v-if="!filenameField" class="warning">注意：请选择作为文件名的字段，否则将使用默认文件名</p>
            </div>
          </div>
          
          <div class="batch-summary">
            <p>预计生成：{{ batchData.length }} 个文件</p>
          </div>
        </div>

        <div class="action-buttons">
          <button
            class="primary"
            @click="functionMode === 'single' ? generateDocument : generateBatchDocuments"
            :disabled="isLoading || !selectedTemplate || (functionMode === 'single' && !outputPath) || (functionMode === 'batch' && !batchOutputDir) || (functionMode === 'batch' && batchData.length === 0)"
          >
            <span v-if="isLoading">生成中...</span>
            <span v-else>
              {{ functionMode === 'single' ? '生成文件' : '批量生成' }}
            </span>
          </button>
          <button
            v-if="functionMode === 'single'"
            @click="openGeneratedFile"
            :disabled="isLoading || !generatedOutputPath"
          >
            打开文件
          </button>
          <button
            v-else-if="functionMode === 'batch'"
            @click="openBatchOutputDir"
            :disabled="isLoading || !batchOutputDir"
          >
            打开输出目录
          </button>
        </div>
      </div>

      <div class="footer-actions">
        <div class="left">
          <button class="secondary" @click="prevStep" :disabled="isLoading">上一步</button>
        </div>
        <div class="right">
          <button class="secondary" @click="goToStep(1)" :disabled="isLoading">重新选择模板</button>
        </div>
      </div>
    </section>
  </main>
</template>

<style scoped>
.app {
  max-width: 1024px;
  margin: 0 auto;
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.app-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.title h1 {
  margin: 0;
  font-size: 22px;
  line-height: 1.2;
  color: #111827;
}

.title p {
  margin: 6px 0 0 0;
  color: #6b7280;
  font-size: 13px;
}

.header-actions {
  display: flex;
  gap: 10px;
  align-items: center;
}

.mode-switch {
  display: flex;
  gap: 15px;
  background: rgba(255, 255, 255, 0.75);
  border: 1px solid rgba(229, 231, 235, 0.9);
  border-radius: 8px;
  padding: 8px 12px;
}

.mode-switch label {
  display: flex;
  align-items: center;
  gap: 5px;
  cursor: pointer;
  font-size: 0.9rem;
  padding: 4px 8px;
  border-radius: 4px;
  transition: all 0.2s ease;
}

.mode-switch input[type="radio"] {
  margin: 0;
  padding: 0;
}

.mode-switch label:hover {
  background: rgba(240, 244, 255, 0.9);
}

.mode-switch input[type="radio"]:checked + span {
  font-weight: 700;
  color: #396cd8;
}

/* 批量处理样式 */
.batch-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.batch-actions {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.batch-action-group {
  background: rgba(255, 255, 255, 0.8);
  border: 1px solid rgba(229, 231, 235, 0.9);
  border-radius: 10px;
  padding: 12px;
}

.batch-action-group h4 {
  margin: 0 0 10px 0;
  font-size: 14px;
  color: #111827;
}

.imported-info {
  margin-top: 10px;
  padding: 8px 12px;
  background: rgba(240, 244, 255, 0.9);
  border: 1px solid rgba(57, 108, 216, 0.3);
  border-radius: 8px;
  font-size: 0.9rem;
  color: #396cd8;
}

.data-preview {
  margin-top: 10px;
}

.data-preview p {
  margin: 0 0 10px 0;
  font-size: 0.9rem;
  color: #6b7280;
}

.table-container {
  max-height: 200px;
  overflow-y: auto;
  border: 1px solid rgba(229, 231, 235, 0.9);
  border-radius: 8px;
}

.preview-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.85rem;
}

.preview-table th,
.preview-table td {
  padding: 8px 12px;
  text-align: left;
  border-bottom: 1px solid rgba(229, 231, 235, 0.9);
}

.preview-table th {
  background: rgba(249, 250, 251, 0.9);
  font-weight: 600;
  color: #111827;
  position: sticky;
  top: 0;
  z-index: 1;
}

.preview-table tr:hover {
  background: rgba(240, 244, 255, 0.5);
}

.more-rows {
  margin-top: 5px;
  font-style: italic;
  color: #6b7280;
  font-size: 0.85rem;
}

.empty-data {
  padding: 20px;
  text-align: center;
  color: #888;
  background-color: #f9f9f9;
  border-radius: 8px;
}

.form-select {
  width: 100%;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  border: 1px solid rgba(229, 231, 235, 0.9);
  border-radius: 8px;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
  outline: none;
}

.form-select:focus {
  border-color: #396cd8;
}

.batch-output-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.filename-info {
  margin-top: 5px;
}

.filename-info p {
  margin: 5px 0;
  font-size: 0.9rem;
  color: #6b7280;
}

.filename-info .warning {
  color: #d97706;
  font-weight: 500;
}

.batch-summary {
  padding: 10px 12px;
  background: rgba(240, 253, 232, 0.9);
  border: 1px solid rgba(110, 231, 183, 0.6);
  border-radius: 8px;
  margin-top: 10px;
}

.batch-summary p {
  margin: 0;
  color: #065f46;
  font-weight: 500;
}

/* 适配移动端 */
@media (max-width: 768px) {
  .batch-action-group {
    padding: 10px;
  }
  
  .table-container {
    max-height: 150px;
  }
  
  .preview-table th,
  .preview-table td {
    padding: 6px 8px;
    font-size: 0.8rem;
  }
}

.stepper {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 10px;
}

.step {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.75);
  border: 1px solid rgba(229, 231, 235, 0.9);
  cursor: pointer;
  text-align: left;
}

.step:disabled {
  cursor: not-allowed;
  opacity: 0.6;
}

.step.active {
  border-color: rgba(57, 108, 216, 0.55);
  background: rgba(240, 244, 255, 0.9);
}

.step-index {
  width: 28px;
  height: 28px;
  border-radius: 999px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: #111827;
  color: #fff;
  font-weight: 700;
  font-size: 12px;
}

.step.active .step-index {
  background: #396cd8;
}

.step-label {
  font-weight: 600;
  color: #111827;
  font-size: 13px;
}

.card {
  background: rgba(255, 255, 255, 0.9);
  border: 1px solid rgba(229, 231, 235, 0.9);
  border-radius: 14px;
  padding: 16px;
  box-shadow: 0 6px 20px rgba(17, 24, 39, 0.06);
}

.card-header {
  margin-bottom: 14px;
}

.card-header h2 {
  margin: 0;
  font-size: 16px;
  color: #111827;
}

.muted {
  margin: 6px 0 0 0;
  color: #6b7280;
  font-size: 13px;
}

.panel {
  background: rgba(249, 250, 251, 0.8);
  border: 1px solid rgba(229, 231, 235, 0.9);
  border-radius: 12px;
  padding: 12px;
}

.panel-title {
  margin: 0 0 10px 0;
  font-size: 14px;
  color: #111827;
}

.two-col {
  display: grid;
  grid-template-columns: 360px 1fr;
  gap: 12px;
  align-items: start;
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

.template-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.template-row {
  width: 100%;
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 12px;
  align-items: start;
  padding: 12px;
  background: rgba(255, 255, 255, 0.75);
  border: 1px solid rgba(229, 231, 235, 0.9);
  border-radius: 12px;
  transition: transform 120ms ease, box-shadow 120ms ease, border-color 120ms ease;
  text-align: left;
}

.template-row:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 10px 22px rgba(17, 24, 39, 0.08);
}

.template-row.selected {
  border-color: rgba(57, 108, 216, 0.55);
  background: rgba(240, 244, 255, 0.85);
}

.template-row-main {
  min-width: 0;
}

.template-row-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.template-name {
  font-weight: 700;
  color: #111827;
}

.template-path {
  margin-top: 6px;
  font-size: 12px;
  color: #6b7280;
  word-break: break-all;
}

.template-meta {
  margin-top: 6px;
  font-size: 12px;
  color: #9ca3af;
}

.template-row-actions {
  display: flex;
  gap: 8px;
  flex-direction: row;
  align-items: center;
}

.empty-state {
  padding: 2rem;
  text-align: center;
  color: #888;
  background-color: #f9f9f9;
  border-radius: 8px;
}

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

.pill {
  display: inline-flex;
  align-items: center;
  border: 1px solid rgba(229, 231, 235, 0.9);
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 12px;
  color: #374151;
  background: rgba(255, 255, 255, 0.8);
  white-space: nowrap;
}

.hint {
  margin-top: 12px;
  border-radius: 12px;
  padding: 12px;
  background: rgba(255, 255, 255, 0.8);
  border: 1px solid rgba(229, 231, 235, 0.9);
}

.hint-title {
  font-weight: 700;
  color: #111827;
  font-size: 13px;
}

.hint-body {
  margin-top: 8px;
  display: grid;
  gap: 8px;
}

.kv {
  display: grid;
  grid-template-columns: 56px 1fr;
  gap: 10px;
  align-items: start;
}

.k {
  color: #6b7280;
  font-size: 12px;
}

.v {
  color: #111827;
  font-size: 12px;
  word-break: break-all;
}

.footer-actions {
  margin-top: 14px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.footer-actions .left,
.footer-actions .right {
  display: flex;
  gap: 10px;
  align-items: center;
}

.summary {
  color: #6b7280;
  font-size: 13px;
}

.strong {
  color: #111827;
  font-weight: 700;
}

@media (max-width: 768px) {
  .app {
    padding: 14px;
  }

  .stepper {
    grid-template-columns: 1fr;
  }

  .two-col {
    grid-template-columns: 1fr;
  }

  .file-selector {
    flex-direction: column;
  }

  .action-buttons {
    flex-direction: column;
  }

  .template-row {
    grid-template-columns: 1fr;
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
