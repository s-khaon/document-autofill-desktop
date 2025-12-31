# 文档自动填充工具

一个完全离线的macOS桌面应用，用于将用户填写的数据自动填充到指定的Word（.docx）模板中，并生成新的Word文件。

## 技术栈

- **桌面壳**：Tauri 2.x
- **前端**：Vue 3 + TypeScript
- **Word 处理**：Python 3.12 + python-docx
- **Python 打包**：PyInstaller

## 项目结构

```
├── src/                      # 前端源代码
│   ├── assets/               # 静态资源
│   ├── App.vue               # 主应用组件
│   └── main.ts               # 应用入口
├── python/                   # Python 源代码
│   ├── main.py               # Python 主程序
│   ├── requirements.txt      # Python 依赖
│   ├── build.sh              # Python 构建脚本

├── src-tauri/                # Tauri 配置文件
│   ├── src/                  # Tauri 后端代码
│   └── tauri.conf.json       # Tauri 配置
├── build.sh                  # 完整构建脚本
├── package.json              # 前端依赖
└── README.md                 # 项目说明
```

## 本地开发

### 环境要求

- Node.js 18+（推荐使用 nvm 管理）
- Yarn 包管理器
- Rust 1.83+（用于构建 Tauri 应用）
- Python 3.12+（用于开发和测试 Python 脚本）

### 安装依赖

```bash
# 安装前端依赖
yarn install

# 安装 Python 依赖

## 使用 uv (推荐)

```bash
cd python
# 创建虚拟环境
uv venv
# 激活虚拟环境
source .venv/bin/activate
# 安装依赖
uv pip install -r requirements.txt
```

## 使用传统 venv (可选)

```bash
cd python
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
```

详细的 uv 使用说明请参考 [UV_USAGE.md](UV_USAGE.md) 文件。

### 开发流程

1. **启动前端开发服务器**

```bash
yarn dev
```

2. **启动 Tauri 开发模式**

```bash
yarn tauri dev
```

3. **开发 Python 脚本**

在 `python/` 目录下修改 Python 代码，然后重新构建 Python 可执行文件：

```bash
cd python
bash build.sh
```

## 构建和打包

### 构建完整应用

使用项目根目录下的 `build.sh` 脚本可以一键构建完整应用：

```bash
bash build.sh
```

该脚本会执行以下步骤：
1. 构建 Python 可执行文件
2. 安装前端依赖
3. 构建前端应用
4. 构建 Tauri 应用

### 分步构建

1. **构建 Python 可执行文件**

```bash
cd python
bash build.sh
```

2. **构建前端应用**

```bash
yarn build
```

3. **构建 Tauri 应用**

```bash
yarn tauri build
```

## 测试

### 测试模板

项目中已提供测试模板 `template/模板文档.docx`，包含各种占位符用于测试。

### 测试 Python 脚本

```bash
cd python
# 激活虚拟环境 (使用 uv)
source .venv/bin/activate
# 或使用传统 venv
source venv/bin/activate

python main.py ../template/模板文档.docx output.docx '{"授权方": "示例授权方", "被授权方": "示例被授权方", "平台": "抖音", "达人名称": "示例达人", "达人ID": "1234567890", "授权视频链接": "https://example.com/video", "年份": "2024", "月份": "12", "日": "31"}'
```

## 运行

### 开发模式运行

```bash
yarn tauri dev
```

### 生产模式运行

构建完成后，应用程序包将生成在以下位置：

```
src-tauri/target/release/bundle/macos/document-autofill.app
```

直接双击 `document-autofill.app` 即可运行应用。

## 使用说明

1. **选择模板**：点击"浏览"按钮，选择要使用的 Word 模板文件（.docx）
2. **填写数据**：根据模板中的占位符，在表单中填写要填充的数据
3. **选择保存位置**：点击"浏览"按钮，选择生成的 Word 文件的保存位置
4. **生成文件**：点击"生成文件"按钮，等待文件生成完成
5. **打开文件**：点击"打开文件"按钮，查看生成的 Word 文件

## 注意事项

1. **完全离线**：应用程序完全离线运行，不依赖任何外部服务
2. **不依赖 Office**：应用程序不依赖用户本机是否安装 Office
3. **占位符格式**：模板中的占位符必须使用 `{{field_name}}` 格式
4. **支持的内容类型**：支持普通段落、表格单元格、页眉和页脚中的占位符
5. **Python 可执行文件**：Python 可执行文件已通过 PyInstaller 打包，包含所有依赖

## 故障排除

### Rust 版本过低

```
error: package `icu_properties v2.1.2` cannot be built because it requires rustc 1.83 or newer, while the currently active rustc version is 1.71.1
```

解决方法：

```bash
rustup update
```

### Python 依赖安装失败

```
ERROR: Could not open requirements file: [Errno 2] No such file or directory: 'requirements.txt'
```

解决方法：

确保在 `python/` 目录下执行安装命令，并确保 `requirements.txt` 文件存在。

### DMG 打包失败

如果在构建过程中出现 DMG 打包失败的情况，可以直接使用生成的 `.app` 文件，它已经包含了所有功能。

## 许可证

MIT
