<script setup lang="ts">
import { ref, reactive, computed, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  open as openDialog,
  save as saveDialog,
  confirm,
  type OpenDialogOptions,
  type SaveDialogOptions,
} from "@tauri-apps/plugin-dialog";
import { Command } from "@tauri-apps/plugin-shell";
import { openPath } from "@tauri-apps/plugin-opener";
import { exists, readTextFile, mkdir, create, readFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { appDataDir, appLogDir } from "@tauri-apps/api/path";

// 模板配置数据结构
interface TemplateConfig {
  id: string;
  name: string;
  path: string;
  placeholders: string[];
  defaultValues: Record<string, string>;
  outputDir: string;
  filenameTemplate: string;
  createdAt: string;
  updatedAt: string;
  timeFieldConfig: Record<string, string>;
  dateFormat: string;
}

// 应用状态
const isLoading = ref<boolean>(false);
const errorMsg = ref<string>("");
const successMsg = ref<string>("");
const selectedTemplateId = ref<string>("");
const currentStep = ref<1 | 2>(1);
const generatedOutputPath = ref<string>("");
const hasEditedForm = ref<boolean>(false);
const showConfigDialog = ref<boolean>(false);

// 批量生成结果列表
const batchGeneratedFiles = ref<string[]>([]);

// 时间类型选择对话框
const showTimeTypeDialog = ref<boolean>(false);
const currentConfiguringField = ref<string>("");
const selectedTimeType = ref<string>("");

// 时间类型选项
const timeTypeOptions = [
  {
    value: "currentDate",
    label: "当前日期",
    description: "自动填充完整日期",
    placeholder: "{{执行时日期}}"
  },
  {
    value: "currentDate.year",
    label: "当前年份",
    description: "自动填充年份",
    placeholder: "{{执行时年}}"
  },
  {
    value: "currentDate.month",
    label: "当前月份",
    description: "自动填充月份",
    placeholder: "{{执行时月}}"
  },
  {
    value: "currentDate.day",
    label: "当前日",
    description: "自动填充日期",
    placeholder: "{{执行时日}}"
  }
];

// 日期格式选项
const dateFormatOptions = [
  {
    value: "yyyy-MM-dd",
    label: "2025-01-01",
    description: "标准格式（年-月-日）"
  },
  {
    value: "yyyy/MM/dd",
    label: "2025/01/01",
    description: "斜杠格式（年/月/日）"
  },
  {
    value: "yyyy年MM月dd日",
    label: "2025年01月01日",
    description: "中文格式"
  },
  {
    value: "MM/dd/yyyy",
    label: "01/01/2025",
    description: "美国格式（月/日/年）"
  },
  {
    value: "dd/MM/yyyy",
    label: "01/01/2025",
    description: "欧洲格式（日/月/年）"
  }
];

// 功能模式：single（单文件生成）或 batch（批量生成）
const functionMode = ref<'single' | 'batch'>('single');

// 批量处理相关状态
const batchData = ref<any[]>([]);
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

function canGoToStep(step: 1 | 2) {
  if (step === 1) return true;
  if (step === 2) return canGoStep2.value;
  return false;
}

async function goToStep(step: 1 | 2) {
  if (!canGoToStep(step)) return;

  currentStep.value = step;
}

async function nextStep() {
  if (currentStep.value === 1) {
    await goToStep(2); // 从步骤1跳到步骤2
    return;
  }
  if (currentStep.value === 2) {
    // 第二步直接生成文件
    if (!hasEditedForm.value) updateFormData();
    await saveConfig();
    await handleGenerate();
    return;
  }
}

async function prevStep() {
  if (currentStep.value === 2) {
    await goToStep(1);
    return;
  }
}

function markFormEdited() {
  hasEditedForm.value = true;
  generatedOutputPath.value = "";
}

// 存储配置文件路径
const CONFIG_FILE = ref<string>("");

// 保存配置到本地存储
async function saveConfig() {
  try {
    if (!CONFIG_FILE.value) {
      console.error("配置文件路径未初始化");
      return;
    }
    console.log("========== 开始保存配置 ==========");
    console.log("配置保存路径：", CONFIG_FILE.value);
    console.log("当前模板数量：", templates.value.length);
    console.log("模板数据：", JSON.stringify(templates.value, null, 2));
    
    // 提取目录路径
    const dirPath = CONFIG_FILE.value.substring(0, CONFIG_FILE.value.lastIndexOf('/'));
    console.log("配置文件目录：", dirPath);
    
    // 检查目录是否存在，如果不存在则创建
    try {
      const dirExists = await exists(dirPath);
      console.log("目录是否存在：", dirExists);
      
      if (!dirExists) {
        console.log("目录不存在，正在创建...");
        await mkdir(dirPath, { recursive: true });
        console.log("目录创建成功！");
      }
    } catch (mkdirError) {
      console.error("创建目录失败：", mkdirError);
      throw mkdirError;
    }
    
    // 保存配置文件
    await writeTextFile(CONFIG_FILE.value, JSON.stringify(templates.value, null, 2));
    
    console.log("配置保存成功！");
    console.log("========== 配置保存完成 ==========");
  } catch (error) {
    console.error("========== 保存配置失败 ==========");
    console.error("错误详情：", error);
    console.error("==================================");
  }
}

// 从本地存储加载配置
async function loadConfig() {
  try {
    if (!CONFIG_FILE.value) {
      console.error("配置文件路径未初始化");
      return;
    }
    console.log("========== 开始加载配置 ==========");
    console.log("配置文件路径：", CONFIG_FILE.value);
    
    try {
      console.log("正在尝试读取配置文件...");
      const content = await readTextFile(CONFIG_FILE.value);
      console.log("配置文件内容长度：", content.length);
      
      templates.value = JSON.parse(content);
      console.log("成功解析配置，模板数量：", templates.value.length);
      console.log("模板数据：", JSON.stringify(templates.value, null, 2));
      
      // 确保所有模板都有 timeFieldConfig 字段（向后兼容）
      templates.value.forEach(template => {
        if (!template.timeFieldConfig) {
          template.timeFieldConfig = {};
        }
        if (!template.dateFormat) {
          template.dateFormat = "yyyy-MM-dd";
        }
      });
      
      if (templates.value.length > 0) {
        selectedTemplateId.value = templates.value[0].id;
        updateFormData();
        console.log("已选中第一个模板：", selectedTemplateId.value);
      }
    } catch (readError) {
      console.log("配置文件不存在或读取失败，使用空配置");
      console.log("错误详情：", readError);
      templates.value = [];
    }
    console.log("========== 配置加载完成 ==========");
  } catch (error) {
    console.error("========== 加载配置失败 ==========");
    console.error("错误详情：", error);
    console.error("==================================");
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
      const originalPath = Array.isArray(selected) ? selected[0] : selected;
      
      // 获取应用数据目录，用于存储复制的模板
      const dataDir = await appDataDir();
      const templatesDir = `${dataDir}/templates`;
      
      // 确保模板目录存在
      try {
        const dirExists = await exists(templatesDir);
        if (!dirExists) {
          await mkdir(templatesDir, { recursive: true });
        }
      } catch (mkdirError) {
        console.error("创建模板目录失败：", mkdirError);
        throw mkdirError;
      }
      
      // 生成模板文件名和路径
      const templateFilename = `${Date.now()}_${originalPath.split("/").pop() || "template.docx"}`;
      const copiedPath = `${templatesDir}/${templateFilename}`;
      
      // 复制模板文件到应用数据目录
      await invoke("copy_file", {
        source: originalPath,
        destination: copiedPath
      });
      
      // 提取模板中的占位符（使用复制后的路径）
      const placeholders = await extractPlaceholdersFromTemplate(copiedPath);
      
      // 创建新的模板配置
      const newTemplate: TemplateConfig = {
        id: `template-${Date.now()}`,
        name: originalPath.split("/").pop() || "新模板",
        path: copiedPath, // 使用复制后的路径
        placeholders: placeholders,
        defaultValues: {
        },
        outputDir: "",
        filenameTemplate: "",
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        timeFieldConfig: {},
        dateFormat: "yyyy-MM-dd",
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
  // 使用 Tauri confirm 函数的正确格式
  const confirmed = await confirm(
    "确定要删除这个模板吗？",
    "删除确认"
  );
  
  if (confirmed) {
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
  Object.keys(formData).forEach(key => delete formData[key]);
  
  if (selectedTemplate.value) {
    selectedTemplate.value.placeholders.forEach(placeholder => {
      const timeType = selectedTemplate.value?.timeFieldConfig?.[placeholder];
      if (timeType) {
        formData[placeholder] = calculateTimeValue(timeType);
      } else {
        formData[placeholder] = selectedTemplate.value?.defaultValues[placeholder] || "";
      }
    });
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
    // 立即保存配置，确保模板名称持久化
    saveConfig();
  }
}

// 更新默认值
function updateDefaultValue(placeholder: string, value: string) {
  if (selectedTemplate.value) {
    selectedTemplate.value.defaultValues[placeholder] = value;
    selectedTemplate.value.updatedAt = new Date().toISOString();
    // 立即保存配置，确保默认值持久化
    saveConfig();
  }
}

// 判断字段是否为时间字段
function isTimeField(placeholder: string): boolean {
  if (!selectedTemplate.value?.timeFieldConfig) {
    return false;
  }
  return !!selectedTemplate.value.timeFieldConfig[placeholder];
}

// 切换时间字段状态
function toggleTimeField(placeholder: string, event: Event) {
  const checkbox = event.target as HTMLInputElement;
  
  if (checkbox.checked) {
    // 打开时间类型选择对话框
    currentConfiguringField.value = placeholder;
    selectedTimeType.value = selectedTemplate.value?.timeFieldConfig?.[placeholder] || "";
    showTimeTypeDialog.value = true;
  } else {
    // 移除时间字段配置
    if (selectedTemplate.value?.timeFieldConfig) {
      delete selectedTemplate.value.timeFieldConfig[placeholder];
      selectedTemplate.value.updatedAt = new Date().toISOString();
      saveConfig();
      
      // 立即更新表单数据，确保配置生效
      updateFormData();
    }
  }
}

// 确认时间类型选择
function confirmTimeType() {
  if (!selectedTemplate.value || !currentConfiguringField.value || !selectedTimeType.value) {
    return;
  }
  
  // 设置时间字段配置
  if (!selectedTemplate.value.timeFieldConfig) {
    selectedTemplate.value.timeFieldConfig = {};
  }
  selectedTemplate.value.timeFieldConfig[currentConfiguringField.value] = selectedTimeType.value;
  
  // 获取对应的占位符
  const option = timeTypeOptions.find(opt => opt.value === selectedTimeType.value);
  if (option) {
    selectedTemplate.value.defaultValues[currentConfiguringField.value] = option.placeholder;
  }
  
  selectedTemplate.value.updatedAt = new Date().toISOString();
  saveConfig();
  showTimeTypeDialog.value = false;
  
  // 立即更新表单数据，确保配置生效
  updateFormData();
}

// 计算时间值（支持占位符替换）
function calculateTimeValue(type: string): string {
  const today = new Date();
  const year = today.getFullYear();
  const month = (today.getMonth() + 1).toString().padStart(2, '0');
  const day = today.getDate().toString().padStart(2, '0');
  
  switch(type) {
    case 'currentDate':
      const dateFormat = selectedTemplate.value?.dateFormat || 'yyyy-MM-dd';
      return dateFormat
        .replace('yyyy', year.toString())
        .replace('MM', month)
        .replace('dd', day);
    case 'currentDate.year':
      return year.toString();
    case 'currentDate.month':
      return month;
    case 'currentDate.day':
      return day;
    default:
      return '';
  }
}

// 替换时间占位符为实际值
function replaceTimePlaceholders(value: any): string {
  const strValue = String(value || '');
  if (!strValue.includes('{{执行时')) {
    return strValue;
  }
  
  const today = new Date();
  const year = today.getFullYear();
  const month = (today.getMonth() + 1).toString().padStart(2, '0');
  const day = today.getDate().toString().padStart(2, '0');
  
  const dateFormat = selectedTemplate.value?.dateFormat || 'yyyy-MM-dd';
  const formattedDate = dateFormat
    .replace('yyyy', year.toString())
    .replace('MM', month)
    .replace('dd', day);
  
  return strValue
    .replace(/{{执行时日期}}/g, formattedDate)
    .replace(/{{执行时年}}/g, year.toString())
    .replace(/{{执行时月}}/g, month)
    .replace(/{{执行时日}}/g, day);
}

// 更新文件名模板
function updateFilenameTemplate(template: string) {
  if (selectedTemplate.value) {
    selectedTemplate.value.filenameTemplate = template;
    selectedTemplate.value.updatedAt = new Date().toISOString();
    // 立即保存配置，确保文件名模板持久化
    saveConfig();
  }
}

// 检查字段是否在文件名模板中
function isFieldInFilename(field: string): boolean {
  if (!selectedTemplate.value?.filenameTemplate) {
    return false;
  }
  return selectedTemplate.value.filenameTemplate.includes(`{{${field}}}`);
}

// 切换字段是否在文件名模板中
function toggleFieldInFilename(field: string, event: Event) {
  if (!selectedTemplate.value) return;

  const checkbox = event.target as HTMLInputElement;
  const isChecked = checkbox.checked;

  if (isChecked) {
    if (!selectedTemplate.value.filenameTemplate) {
      selectedTemplate.value.filenameTemplate = `{{${field}}}.docx`;
    } else {
      const docxIndex = selectedTemplate.value.filenameTemplate.indexOf('.docx');
      if (docxIndex === -1) {
        selectedTemplate.value.filenameTemplate += `{{${field}}}`;
      } else {
        selectedTemplate.value.filenameTemplate =
          selectedTemplate.value.filenameTemplate.slice(0, docxIndex) +
          `{{${field}}}` +
          selectedTemplate.value.filenameTemplate.slice(docxIndex);
      }
    }
  } else {
    selectedTemplate.value.filenameTemplate =
      selectedTemplate.value.filenameTemplate.replace(`{{${field}}}`, '').replace(/\.docx$/, '.docx');
  }

  selectedTemplate.value.updatedAt = new Date().toISOString();
  saveConfig();
}

// 根据文件名模板和表单数据生成实际文件名
function generateFilenameFromTemplate(template: string, data: Record<string, string>): string {
  if (!template) {
    return `output_${Date.now()}.docx`;
  }
  
  let filename = template;
  
  // 替换所有 {{字段名}} 占位符
  for (const [key, value] of Object.entries(data)) {
    const placeholder = `{{${key}}}`;
    filename = filename.replace(new RegExp(placeholder, 'g'), value || '');
  }
  
  // 如果模板中没有 .docx 后缀，添加它
  if (!filename.endsWith('.docx')) {
    filename += '.docx';
  }
  
  return filename;
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

    const { utils, write } = await import('xlsx');
    
    // 创建工作表数据
    const headers = selectedTemplate.value.placeholders;
    const worksheetData = [headers];
    
    // 创建工作簿和工作表
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
      
      // 使用xlsx库生成Excel数据
      const excelBuffer = write(workbook, { bookType: 'xlsx', type: 'array' });
      
      // 使用Tauri的fs API写入二进制文件
      const file = await create(selected);
      await file.write(excelBuffer);
      await file.close();
      
      successMsg.value = `Excel模板导出成功！保存位置：${selected}`;
    }
  } catch (error) {
    errorMsg.value = `导出Excel模板失败：${error instanceof Error ? error.message : String(error)}`;
  } finally {
    isLoading.value = false;
  }
}

// 处理生成文件按钮点击
async function handleGenerate() {
  if (functionMode.value === 'single') {
    await generateDocument();
  } else {
    await generateBatchDocuments();
  }
}

// 生成Word文件
async function generateDocument() {
  try {
    if (!selectedTemplate.value) {
      errorMsg.value = "请先选择模板文件";
      return;
    }

    // 检查是否配置了输出目录，如果没有则让用户选择
    if (!selectedTemplate.value.outputDir) {
      await selectOutputDir();
      // 如果用户取消了目录选择，则返回
      if (!selectedTemplate.value.outputDir) {
        return;
      }
    }

    isLoading.value = true;
    errorMsg.value = "";
    successMsg.value = "";

    // 替换时间占位符为实际值
    const processedFormData: Record<string, string> = {};
    for (const [key, value] of Object.entries(formData)) {
      processedFormData[key] = replaceTimePlaceholders(value);
    }

    // 获取Python可执行文件路径
    const pythonExecutable = await invoke<string>("get_python_executable");
    
    // 构建命令行参数
    const dataJson = JSON.stringify(processedFormData);
    
    // 生成文件名
    const filename = generateFilenameFromTemplate(selectedTemplate.value.filenameTemplate, formData);
    const outputPath = `${selectedTemplate.value.outputDir}/${filename}`;
    
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
    
    console.log("执行生成命令:", commandName, [mainPyPath, selectedTemplate.value.path, outputPath, "dataJson..."]);

    // 调用Python程序
    const result = await Command.create(commandName, [
      mainPyPath,
      selectedTemplate.value.path, 
      outputPath, 
      dataJson,
      "fill"
    ]).execute();
    
    if (result.code === 0) {
      successMsg.value = `文件生成成功！保存位置：${outputPath}`;
      generatedOutputPath.value = outputPath;
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
  if (generatedOutputPath.value) {
    try {
      await openPath(generatedOutputPath.value);
    } catch (error) {
      errorMsg.value = `打开文件失败：${error instanceof Error ? error.message : String(error)}`;
    }
  }
}

// 打开批量生成的文件
async function openBatchFile(filePath: string) {
  try {
    await openPath(filePath);
  } catch (error) {
    errorMsg.value = `打开文件失败：${error instanceof Error ? error.message : String(error)}`;
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
      
      // 清除上一次的导入和生成结果
      batchData.value = [];
      batchGeneratedFiles.value = [];
      
      const { read, utils } = await import('xlsx');
      
      // 使用Tauri的fs API读取二进制文件
      const fileContent = await readFile(filePath);
      
      // 解析Excel文件
      const workbook = read(fileContent, {
        type: 'array'
      });
      
      // 获取第一个工作表
      const worksheetName = workbook.SheetNames[0];
      const worksheet = workbook.Sheets[worksheetName];
      
      // 先获取Excel的实际列名（第一行）
      const range = utils.decode_range(worksheet['!ref'] || 'A1');
      const excelHeaders: string[] = [];
      for (let col = range.s.c; col <= range.e.c; col++) {
        const cellAddress = utils.encode_cell({ r: 0, c: col });
        const cell = worksheet[cellAddress];
        excelHeaders.push(cell ? String(cell.v) : `列${col + 1}`);
      }
      
      // 创建占位符到Excel列名的映射
      const placeholderToHeader: Record<string, string> = {};
      selectedTemplate.value.placeholders.forEach(placeholder => {
        // 查找匹配的列名（精确匹配或部分匹配）
        const matchingHeader = excelHeaders.find(h => h === placeholder);
        if (matchingHeader) {
          placeholderToHeader[placeholder] = matchingHeader;
        }
      });
      
      // 转换为JSON格式，使用Excel实际列名
      const jsonData = utils.sheet_to_json(worksheet, {
        header: excelHeaders,
        range: 1 // 跳过第一行（表头行）
      });
      
      // 重新映射列名，将Excel列名转换为占位符
      const templatePlaceholders = selectedTemplate.value?.placeholders || [];
      batchData.value = jsonData.map((row: any) => {
        const mappedRow: Record<string, any> = {};
        templatePlaceholders.forEach(placeholder => {
          const excelHeader = placeholderToHeader[placeholder];
          if (excelHeader && row[excelHeader] !== undefined) {
            mappedRow[placeholder] = row[excelHeader];
          } else {
            mappedRow[placeholder] = "";
          }
        });
        return mappedRow;
      }).filter((row: any, index: number) => {
        // 检查第一行是否为标题行
        if (index === 0 && selectedTemplate.value) {
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

    // 检查是否配置了输出目录，如果没有则让用户选择
    if (!selectedTemplate.value.outputDir) {
      await selectOutputDir();
      // 如果用户取消了目录选择，则返回
      if (!selectedTemplate.value.outputDir) {
        return;
      }
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
    
    console.log("开始批量生成，共", batchData.value.length, "条记录");
    console.log("输出目录:", selectedTemplate.value.outputDir);
    console.log("文件名模板:", selectedTemplate.value.filenameTemplate);
    
    for (let i = 0; i < batchData.value.length; i++) {
      const rowData = batchData.value[i];
      
      console.log("处理第", i+1, "条数据:", rowData);
      
      // 替换时间占位符为实际值
      const processedRowData: Record<string, string> = {};
      for (const [key, value] of Object.entries(rowData)) {
        processedRowData[key] = replaceTimePlaceholders(value);
      }
      
      // 使用文件名模板生成文件名
      const filename = generateFilenameFromTemplate(selectedTemplate.value.filenameTemplate, processedRowData);
      console.log("生成文件名:", filename);
      
      // 构建输出路径
      const outputFilePath = `${selectedTemplate.value.outputDir}/${filename}`;
      console.log("输出路径:", outputFilePath);
      
      // 构建命令行参数
      const dataJson = JSON.stringify(processedRowData);
      
      try {
        // 调用Python程序
        console.log("调用Python生成文件...");
        const result = await Command.create(commandName, [
          mainPyPath,
          selectedTemplate.value.path, 
          outputFilePath, 
          dataJson,
          "fill"
        ]).execute();
        
        console.log("Python返回结果:", result.code, result.stderr);
        
        if (result.code === 0) {
          successCount++;
          batchGeneratedFiles.value.push(outputFilePath);
          console.log("文件生成成功:", outputFilePath);
        } else {
          failureCount++;
          console.error(`生成文件失败 (${i+1}/${batchData.value.length})：${result.stderr || result.stdout}`);
        }
      } catch (error) {
        failureCount++;
        console.error(`生成文件异常 (${i+1}/${batchData.value.length})：${error instanceof Error ? error.message : String(error)}`);
      }
    }
    
    console.log("批量生成完成，成功:", successCount, "失败:", failureCount);
    
    if (successCount > 0) {
      successMsg.value = `批量生成完成！成功：${successCount} 个，失败：${failureCount} 个`;
    } else if (failureCount > 0) {
      errorMsg.value = `批量生成失败！请检查模板配置或Python环境`;
    }
    
  } catch (error) {
    errorMsg.value = `批量生成失败：${error instanceof Error ? error.message : String(error)}`;
  } finally {
    isLoading.value = false;
  }
}

// 生命周期钩子
onMounted(async () => {
  console.log("========== 应用启动 ==========");
  
  // 初始化配置文件路径
  try {
    const dataDir = await appDataDir();
    CONFIG_FILE.value = `${dataDir}/templates-config.json`;
    console.log("应用数据目录：", dataDir);
    console.log("配置文件路径：", CONFIG_FILE.value);
  } catch (error) {
    console.error("获取应用数据目录失败:", error);
  }
  
  // 加载保存的配置
  await loadConfig();
  
  console.log("========== 应用启动完成 ==========");
});

// 打开日志目录
async function openLogs() {
  try {
    const logDir = await appLogDir();
    await openPath(logDir);
  } catch (error) {
    console.error("Failed to open log dir:", error);
    errorMsg.value = "无法打开日志目录";
  }
}

// 监听选中模板变化
watch(selectedTemplateId, () => {
  updateFormData();
  hasEditedForm.value = false;
  generatedOutputPath.value = "";
  batchGeneratedFiles.value = [];
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
        <button @click="openLogs" class="secondary small" style="margin-right: 10px">
          查看日志
        </button>
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
        <button @click="addTemplate" :disabled="isLoading" class="special">
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
        <span class="step-label">填写数据</span>
      </button>
    </nav>

    <!-- 错误信息 -->
    <div v-if="errorMsg" class="message error">{{ errorMsg }}</div>
    <!-- 成功信息 -->
    <div v-if="successMsg" class="message success">
      <span>{{ successMsg }}</span>
      <button
        v-if="generatedOutputPath"
        @click="openGeneratedFile"
        class="secondary small"
        style="margin-left: 10px;"
      >
        打开文件
      </button>
    </div>

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
            <button class="primary" @click="selectedTemplateId = template.id; goToStep(2)">
              使用
            </button>
            <button class="secondary" @click="selectedTemplateId = template.id; showConfigDialog = true">
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
          </div>
        </div>
      </div>

      <!-- 批量生成结果列表 -->
      <div v-if="batchGeneratedFiles.length > 0" class="batch-result-section">
        <div class="panel">
          <h3 class="panel-title">生成结果</h3>
          <div class="generated-files-list">
            <div v-for="(filePath, index) in batchGeneratedFiles" :key="index" class="generated-file-item">
              <span class="file-name">{{ filePath }}</span>
              <button class="secondary" @click="openBatchFile(filePath)" :disabled="isLoading">
                打开
              </button>
            </div>
          </div>
        </div>
      </div>

      <div class="footer-actions">
        <div class="left">
          <button class="secondary" @click="prevStep" :disabled="isLoading">上一步</button>
        </div>
        <div class="right">
          <button 
            class="primary" 
            @click="nextStep" 
            :disabled="isLoading || !canGoToStep(2) || (functionMode === 'batch' && batchData.length === 0)"
            style="background-color: #10b981; border-color: #10b981;"
          >
            生成文件
          </button>
        </div>
      </div>
    </section>



    <!-- 配置对话框 -->
    <div v-if="showConfigDialog && selectedTemplate" class="dialog-overlay" @click.self="showConfigDialog = false">
      <div class="dialog">
        <div class="dialog-header">
          <h2>配置模板</h2>
          <button class="close-button" @click="showConfigDialog = false">&times;</button>
        </div>
        <div class="dialog-body">
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

          <div class="form-item">
            <label for="filename-template">文件名模板</label>
            <input
              id="filename-template"
              type="text"
              v-model="selectedTemplate.filenameTemplate"
              placeholder="例如：&#123;&#123;授权方&#125;&#125;-授权书.docx"
              @input="updateFilenameTemplate(selectedTemplate.filenameTemplate)"
            />
            <div class="field-selector">
              <label class="field-selector-label">选择字段：</label>
              <div class="checkbox-group">
                <label
                  v-for="placeholder in selectedTemplate.placeholders"
                  :key="placeholder"
                  class="checkbox-item"
                >
                  <input
                    type="checkbox"
                    :checked="isFieldInFilename(placeholder)"
                    @change="toggleFieldInFilename(placeholder, $event)"
                  />
                  <span>{{ placeholder }}</span>
                </label>
              </div>
            </div>
            <div class="hint-text">
              使用说明：选择字段后会自动生成 &#123;&#123;字段名&#125;&#125; 格式的占位符，生成文档时会用实际值替换。例如：&#123;&#123;授权方&#125;&#125;-授权书.docx
            </div>
          </div>

          <div class="form-item">
            <label for="date-format">日期格式</label>
            <select
            id="date-format"
            class="form-select"
            v-model="selectedTemplate.dateFormat"
            @change="saveConfig(); updateFormData();"
          >
            <option
              v-for="option in dateFormatOptions"
              :key="option.value"
              :value="option.value"
            >
              {{ option.label }} - {{ option.description }}
            </option>
          </select>
            <div class="hint-text">
              说明：此格式用于"当前日期"类型的时间字段，生成文档时会使用选定的格式显示日期。
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

          <div class="hint">
            <div class="hint-title">时间字段配置说明</div>
            <div class="hint-body">
              <p>您可以为字段配置时间相关的默认值，系统会自动填充当前日期：</p>
              <ul>
                <li><strong>当前日期</strong>：自动填充完整日期，格式可在上方"日期格式"中配置</li>
                <li><strong>当前年份</strong>：自动填充年份，如 2026</li>
                <li><strong>当前月份</strong>：自动填充月份，如 01</li>
                <li><strong>当前日期</strong>：自动填充日期，如 01</li>
              </ul>
              <p>使用模板时，这些字段会自动更新为当天的日期。</p>
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
                <div class="default-value-container">
                  <input
                    :id="`default-${placeholder}`"
                    type="text"
                    v-model="selectedTemplate.defaultValues[placeholder]"
                    @input="updateDefaultValue(placeholder, selectedTemplate.defaultValues[placeholder])"
                    placeholder="设置默认值"
                  />
                  <label class="time-field-checkbox">
                    <input
                      type="checkbox"
                      :checked="isTimeField(placeholder)"
                      @change="toggleTimeField(placeholder, $event)"
                    />
                    <span>时间字段</span>
                  </label>
                </div>
              </div>
            </div>
          </div>
        </div>
        <div class="dialog-footer">
          <button class="secondary" @click="showConfigDialog = false">取消</button>
          <button class="primary" @click="saveTemplateConfig(); showConfigDialog = false; updateFormData();">保存</button>
        </div>
      </div>
    </div>

    <!-- 时间类型选择对话框 -->
    <div v-if="showTimeTypeDialog" class="dialog-overlay" @click.self="showTimeTypeDialog = false">
      <div class="dialog dialog-small">
        <div class="dialog-header">
          <h2>选择时间类型</h2>
          <button class="close-button" @click="showTimeTypeDialog = false">&times;</button>
        </div>
        <div class="dialog-body">
          <p class="field-info">当前字段：<strong>{{ currentConfiguringField }}</strong></p>
          <div class="time-type-options">
            <label
              v-for="option in timeTypeOptions"
              :key="option.value"
              class="time-type-option"
              :class="{ selected: selectedTimeType === option.value }"
            >
              <input
                type="radio"
                :value="option.value"
                v-model="selectedTimeType"
                :name="`time-type-${currentConfiguringField}`"
              />
              <div class="option-content">
                <div class="option-title">{{ option.label }}</div>
                <div class="option-desc">{{ option.description }}</div>
                <div class="option-placeholder">占位符：{{ option.placeholder }}</div>
              </div>
            </label>
          </div>
        </div>
        <div class="dialog-footer">
          <button class="secondary" @click="showTimeTypeDialog = false">取消</button>
          <button class="primary" @click="confirmTimeType">确定</button>
        </div>
      </div>
    </div>
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
  max-height: 400px;
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

/* 批量生成结果列表 */
.batch-result-section {
  margin-top: 16px;
}

.generated-files-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 300px;
  overflow-y: auto;
}

.generated-file-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  background: rgba(255, 255, 255, 0.8);
  border: 1px solid rgba(229, 231, 235, 0.9);
  border-radius: 8px;
}

.file-name {
  font-weight: 500;
  color: #111827;
  font-size: 14px;
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

.form-item input {
  width: 100%;
  max-width: 100%;
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
  box-sizing: border-box;
}

.form-item input:focus {
  border-color: #396cd8;
}

.action-buttons {
  display: flex;
  gap: 1rem;
}

/* action-buttons中的按钮继承全局样式 */
.action-buttons button {
  border-radius: 8px;
}

/* 确保.action-buttons中的按钮使用全局定义的样式 */
.action-buttons button.primary,
.action-buttons button.secondary,
.action-buttons button.danger,
.action-buttons button.success,
.action-buttons button.special {
  background-color: inherit;
  color: inherit;
  border-color: inherit;
}

.action-buttons button.primary:hover,
.action-buttons button.danger:hover,
.action-buttons button.success:hover,
.action-buttons button.special:hover {
  transform: translateY(-1px);
  box-shadow: inherit;
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
  border: 1px solid transparent;
  border-radius: 8px;
  cursor: pointer;
  font-size: 0.9rem;
  font-weight: 500;
  transition: all 0.2s ease;
}

button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

/* 主要操作按钮 - 蓝色（用于核心功能，如生成文件、下一步） */
button.primary {
  background-color: #3b82f6;
  color: white;
  border-color: #3b82f6;
}

button.primary:hover:not(:disabled) {
  background-color: #2563eb;
  border-color: #2563eb;
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(59, 130, 246, 0.3);
}

/* 次要操作按钮 - 灰色（用于辅助功能，如上一步、取消、浏览） */
button.secondary {
  background-color: #f3f4f6;
  color: #374151;
  border-color: #e5e7eb;
}

button.secondary:hover:not(:disabled) {
  background-color: #e5e7eb;
  border-color: #d1d5db;
}

/* 危险按钮 - 红色（用于删除、清除等危险操作） */
button.danger {
  background-color: #ef4444;
  color: white;
  border-color: #ef4444;
}

button.danger:hover:not(:disabled) {
  background-color: #dc2626;
  border-color: #dc2626;
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(239, 68, 68, 0.3);
}

/* 成功按钮 - 绿色（用于成功相关操作，如生成文件成功后的打开文件） */
button.success {
  background-color: #10b981;
  color: white;
  border-color: #10b981;
}

button.success:hover:not(:disabled) {
  background-color: #059669;
  border-color: #059669;
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3);
}

/* 特殊功能按钮 - 紫色（用于添加模板等特殊操作） */
button.special {
  background-color: #8b5cf6;
  color: white;
  border-color: #8b5cf6;
}

button.special:hover:not(:disabled) {
  background-color: #7c3aed;
  border-color: #7c3aed;
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(139, 92, 246, 0.3);
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

/* 对话框样式优化 */
.dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.dialog {
  background-color: white;
  border-radius: 16px;
  box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.25), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
  max-width: 900px;
  width: 95%;
  max-height: 90vh;
  overflow-y: auto;
}

.dialog-header {
  padding: 20px 24px;
  border-bottom: 1px solid #e5e7eb;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.dialog-header h2 {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
  color: #111827;
}

.close-button {
  background: none;
  border: none;
  font-size: 24px;
  cursor: pointer;
  color: #6b7280;
  padding: 0;
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  transition: background-color 0.2s;
}

.close-button:hover {
  background-color: #f3f4f6;
}

.dialog-body {
  padding: 24px;
}

.dialog-footer {
  padding: 20px 24px;
  border-top: 1px solid #e5e7eb;
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

.field-selector {
  margin-top: 12px;
}

.field-selector-label {
  font-weight: 500;
  color: #4b5563;
  margin-bottom: 10px;
  display: block;
  font-size: 14px;
}

.checkbox-group {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  gap: 12px;
  margin-top: 10px;
}

.checkbox-item {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  font-size: 0.95rem;
  padding: 8px 12px;
  border-radius: 8px;
  transition: background-color 0.2s;
}

.checkbox-item:hover {
  background-color: #f9fafb;
}

.checkbox-item input[type="checkbox"] {
  width: 18px;
  height: 18px;
  accent-color: #396cd8;
}

.hint-text {
  margin-top: 12px;
  font-size: 0.875rem;
  color: #6b7280;
  background-color: #f9fafb;
  padding: 12px;
  border-radius: 8px;
  border: 1px solid #e5e7eb;
  line-height: 1.5;
}

/* 时间类型选择对话框 */
.dialog-small {
  max-width: 500px;
}

.field-info {
  margin: 0 0 16px 0;
  padding: 12px;
  background-color: #f0f9ff;
  border: 1px solid #bae6fd;
  border-radius: 8px;
  font-size: 14px;
  color: #0369a1;
  line-height: 1.5;
}

.field-info strong {
  font-weight: 600;
  color: #0c4a6e;
}

.time-type-options {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.time-type-option {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 14px 16px;
  border: 2px solid #e5e7eb;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s;
  background-color: #ffffff;
}

.time-type-option:hover {
  border-color: #93c5fd;
  background-color: #f0f9ff;
}

.time-type-option.selected {
  border-color: #396cd8;
  background-color: #eff6ff;
  box-shadow: 0 0 0 3px rgba(57, 108, 216, 0.1);
}

.time-type-option input[type="radio"] {
  width: 20px;
  height: 20px;
  margin-top: 2px;
  accent-color: #396cd8;
  cursor: pointer;
}

.option-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.option-title {
  font-weight: 600;
  font-size: 15px;
  color: #111827;
}

.option-desc {
  font-size: 13px;
  color: #6b7280;
  line-height: 1.4;
}

.option-placeholder {
  font-size: 12px;
  color: #396cd8;
  font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
  background-color: #f0f9ff;
  padding: 4px 8px;
  border-radius: 4px;
  display: inline-block;
  margin-top: 4px;
  font-weight: 500;
}

/* 表单样式优化 */
.form-item {
  margin-bottom: 20px;
}

.form-item label {
  margin-bottom: 8px;
  font-weight: 500;
  color: #374151;
  font-size: 14px;
}

.form-item input[type="text"] {
  width: 100%;
  padding: 10px 14px;
  font-size: 14px;
  border: 1px solid #d1d5db;
  border-radius: 8px;
  transition: border-color 0.2s, box-shadow 0.2s;
  background-color: #ffffff;
}

.form-item input[type="text"]:focus {
  outline: none;
  border-color: #396cd8;
  box-shadow: 0 0 0 3px rgba(57, 108, 216, 0.1);
}

.form-item select {
  width: 100%;
  padding: 10px 14px;
  font-size: 14px;
  border: 1px solid #d1d5db;
  border-radius: 8px;
  transition: border-color 0.2s, box-shadow 0.2s;
  background-color: #ffffff;
  cursor: pointer;
}

.form-item select:focus {
  outline: none;
  border-color: #396cd8;
  box-shadow: 0 0 0 3px rgba(57, 108, 216, 0.1);
}

/* 时间字段容器 */
.default-value-container {
  display: flex;
  gap: 10px;
  align-items: center;
}

.default-value-container input[type="text"] {
  flex: 1;
  padding: 10px 14px;
  font-size: 14px;
  border: 1px solid #d1d5db;
  border-radius: 8px;
  transition: border-color 0.2s, box-shadow 0.2s;
  background-color: #ffffff;
}

.default-value-container input[type="text"]:focus {
  outline: none;
  border-color: #396cd8;
  box-shadow: 0 0 0 3px rgba(57, 108, 216, 0.1);
}

.time-field-checkbox {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  padding: 8px 12px;
  border-radius: 8px;
  background-color: #f9fafb;
  border: 1px solid #e5e7eb;
  transition: all 0.2s;
  font-size: 13px;
  font-weight: 500;
  color: #4b5563;
  user-select: none;
}

.time-field-checkbox:hover {
  background-color: #f3f4f6;
  border-color: #d1d5db;
}

.time-field-checkbox input[type="checkbox"] {
  width: 18px;
  height: 18px;
  accent-color: #396cd8;
  cursor: pointer;
}

.time-field-checkbox span {
  white-space: nowrap;
}

.file-selector {
  display: flex;
  gap: 10px;
  align-items: center;
}

.file-selector input[type="text"] {
  flex: 1;
  background-color: #f9fafb;
  border-color: #e5e7eb;
}

/* 模板信息面板 */
.hint {
  margin-top: 20px;
  border-radius: 12px;
  padding: 16px;
  background: rgba(255, 255, 255, 0.95);
  border: 1px solid #e5e7eb;
}

.hint-title {
  font-weight: 600;
  color: #111827;
  font-size: 14px;
  margin-bottom: 12px;
}

.hint-body {
  display: grid;
  gap: 10px;
}

.hint-body p {
  margin: 0;
  color: #4b5563;
  font-size: 13px;
  line-height: 1.6;
}

.hint-body ul {
  margin: 8px 0 0 0;
  padding-left: 20px;
  color: #4b5563;
  font-size: 13px;
  line-height: 1.8;
}

.hint-body li {
  margin-bottom: 4px;
}

.hint-body strong {
  color: #111827;
  font-weight: 600;
}

.kv {
  display: grid;
  grid-template-columns: 60px 1fr;
  gap: 12px;
  align-items: center;
}

.k {
  color: #6b7280;
  font-size: 13px;
  font-weight: 500;
}

.v {
  color: #111827;
  font-size: 13px;
  word-break: break-all;
}

/* 默认值设置面板 */
.panel {
  margin-top: 20px;
  background: rgba(249, 250, 251, 0.95);
  border: 1px solid #e5e7eb;
  border-radius: 12px;
  padding: 16px;
}

.panel-title {
  margin: 0 0 16px 0;
  font-size: 16px;
  font-weight: 600;
  color: #111827;
}

/* 按钮样式优化 */
.dialog-footer button {
  padding: 10px 20px;
  font-size: 14px;
  font-weight: 500;
  border-radius: 8px;
  transition: all 0.2s;
}

.dialog-footer button.primary {
  background-color: #396cd8;
  color: white;
  border: 1px solid #396cd8;
}

.dialog-footer button.primary:hover {
  background-color: #2d53a5;
  border-color: #2d53a5;
}

.dialog-footer button.secondary {
  background-color: #ffffff;
  color: #374151;
  border: 1px solid #d1d5db;
}

.dialog-footer button.secondary:hover {
  background-color: #f9fafb;
  border-color: #396cd8;
}

/* 适配移动端 */
@media (max-width: 768px) {
  .dialog {
    width: 98%;
    max-height: 95vh;
  }
  
  .dialog-small {
    max-width: 98%;
  }
  
  .dialog-body {
    padding: 16px;
  }
  
  .checkbox-group {
    grid-template-columns: 1fr;
  }
  
  .file-selector {
    flex-direction: column;
    gap: 8px;
  }
  
  .file-selector input[type="text"] {
    width: 100%;
  }
  
  .time-type-option {
    padding: 12px 14px;
  }
  
  .option-title {
    font-size: 14px;
  }
  
  .option-desc {
    font-size: 12px;
  }
  
  .option-placeholder {
    font-size: 11px;
    padding: 3px 6px;
  }
  
  .default-value-container {
    flex-direction: column;
    align-items: stretch;
  }
  
  .time-field-checkbox {
    width: 100%;
    justify-content: center;
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
