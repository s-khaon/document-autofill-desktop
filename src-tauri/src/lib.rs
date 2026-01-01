// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::fs::copy;
use std::path::Path;
use std::path::PathBuf;
use log::{info, error};
use tauri::Manager;
use std::process::Command;
use chrono::Local;

#[tauri::command]
fn run_python_backend(app: tauri::AppHandle, args: Vec<String>) -> Result<String, String> {
    // 1. 获取 Python 可执行文件路径 (复用逻辑)
    let python_path = get_python_executable(app);
    
    info!("Executing Python backend: {} with args: {:?}", python_path, args);

    // 2. 检查是 python3 还是 bundled executable
    let mut cmd;
    
    if python_path.ends_with("word_filler") || python_path.ends_with("word_filler.exe") {
        // Bundled executable: 直接运行，不需要 python 解释器
        cmd = Command::new(&python_path);
        // Bundled args: 直接传参
        cmd.args(&args);
    } else {
        // System/Venv Python: 需要作为解释器运行 main.py
        cmd = Command::new(&python_path);
        // 第一个参数应该是 main.py 路径，需要确保它被正确传递
        // 前端传来的 args 应该已经包含 main.py
        cmd.args(&args);
    }

    // 3. 执行命令
    // 设置 working directory? 
    // 对于 bundled，通常不需要。对于 script，可能需要。
    // 这里我们假设 args 里的路径都是绝对路径。

    // 针对 macOS App Bundle 的特殊处理
    // 如果是被 Gatekeeper 隔离，可能需要设置一些环境变量？暂时先不加。

    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                info!("Python execution success: {}", stdout);
                Ok(stdout)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                error!("Python execution failed. Status: {:?}, Stderr: {}, Stdout: {}", output.status, stderr, stdout);
                Err(format!("Execution failed: {}\nStdout: {}", stderr, stdout))
            }
        }
        Err(e) => {
            error!("Failed to spawn python command: {}", e);
            Err(format!("Failed to execute command: {}", e))
        }
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_python_executable(app: tauri::AppHandle) -> String {
    // 获取 Python 可执行文件路径
    // 1. 优先检查打包后的 word_filler 可执行文件
    let resource_dir = app.path().resource_dir().unwrap_or_default();

    let candidate_paths = [
        // 常规放在 resources/word_filler
        resource_dir.join("resources").join("word_filler"),
        resource_dir.join("resources").join("word_filler.exe"),
        // 直接放在 Resources 根目录
        resource_dir.join("word_filler"),
        resource_dir.join("word_filler.exe"),
        // 当前项目配置下，来自 ../python/dist/word_filler，会被放在 _up_/python/dist 下
        resource_dir.join("_up_").join("python").join("dist").join("word_filler"),
        resource_dir.join("_up_").join("python").join("dist").join("word_filler.exe"),
    ];

    for path in candidate_paths {
        if path.exists() {
            info!("Found bundled python executable: {:?}", path);
            return path.to_string_lossy().to_string();
        }
    }

    // 2. 开发模式下，尝试查找虚拟环境
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // 获取项目根目录 (src-tauri 的父目录)
    if let Some(project_root) = manifest_dir.parent() {
        // 检查常见位置
        let venv_paths = vec![
            // python/.venv/bin/python3
            project_root
                .join("python")
                .join(".venv")
                .join("bin")
                .join("python3"),
            project_root
                .join("python")
                .join("venv")
                .join("bin")
                .join("python3"),
            // .venv/bin/python3
            project_root.join(".venv").join("bin").join("python3"),
            project_root.join("venv").join("bin").join("python3"),
        ];

        for venv_path in venv_paths {
            if venv_path.exists() {
                println!("Found venv python: {:?}", venv_path);
                return venv_path.to_string_lossy().to_string();
            }
        }
    }

    // 3. 默认回退到系统 python3
    "python3".to_string()
}

#[tauri::command]
fn copy_file(source: String, destination: String) -> Result<(), String> {
    // 确保源文件存在
    if !Path::new(&source).exists() {
        return Err(format!("源文件不存在: {}", source));
    }

    // 复制文件
    match copy(&source, &destination) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("复制文件失败: {}", e)),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
                .max_file_size(2 * 1024 * 1024) // 2MB
                .format(|out, message, record| {
                    out.finish(format_args!(
                        "[{} {} {}] {}",
                        Local::now().format("%Y-%m-%d %H:%M:%S"),
                        record.level(),
                        record.target(),
                        message
                    ))
                })
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|_app| {
            info!("Backend logging system initialized successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_python_executable,
            copy_file,
            run_python_backend
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
