#!/bin/bash

# 完整的应用构建脚本

set -e

echo "========================================"
echo "文档自动填充工具构建脚本"
echo "========================================"

# 进入Python目录，构建Python可执行文件
echo "\n1. 构建Python可执行文件..."
cd python
bash build.sh

# 返回项目根目录
cd ..

# 安装前端依赖
echo "\n2. 安装前端依赖..."
yarn install

# 构建前端应用
echo "\n3. 构建前端应用..."
yarn build

# 构建Tauri应用
echo "\n4. 构建Tauri应用..."
yarn tauri build

echo "\n========================================"
echo "构建完成！"
echo "========================================"
echo "应用路径：$(pwd)/src-tauri/target/release/document-autofill.app"
