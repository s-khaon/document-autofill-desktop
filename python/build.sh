#!/bin/bash

# 构建Python可执行文件

set -e

echo "正在构建Python可执行文件..."

# 创建虚拟环境
if [ ! -d "venv" ]; then
    python3 -m venv venv
fi

# 激活虚拟环境
source venv/bin/activate

# 安装依赖
pip install -r requirements.txt
pip install pyinstaller

# 使用PyInstaller打包
echo "使用PyInstaller打包..."
pyinstaller --onefile --name word_filler main.py

echo "构建完成！"
echo "可执行文件路径：$(pwd)/dist/word_filler"
