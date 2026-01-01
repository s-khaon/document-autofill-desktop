// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::fs::copy;
use std::path::Path;
use std::path::PathBuf;
use log::info;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_python_executable() -> String {
    // 获取 Python 可执行文件路径
    // 1. 检查打包资源目录
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("resources");
    path.push("word_filler");

    if path.exists() {
        return path.to_string_lossy().to_string();
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
                        "[{} {}] {}",
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
            copy_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
