use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub percentage: u8,
    pub detail: String,
}

fn app_data(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap()
}

pub(crate) fn runtime_dir(app: &AppHandle) -> PathBuf {
    app_data(app).join("runtime")
}

pub(crate) fn node_dir(app: &AppHandle) -> PathBuf {
    runtime_dir(app).join("node")
}

fn private_node_bin(app: &AppHandle) -> PathBuf {
    node_dir(app).join(if cfg!(windows) {
        "node.exe"
    } else {
        "bin/node"
    })
}

pub(crate) fn system_node() -> Option<PathBuf> {
    let mut candidates = vec![
        PathBuf::from(r"C:\Program Files\nodejs\node.exe"),
        PathBuf::from(r"C:\Program Files (x86)\nodejs\node.exe"),
    ];
    if let Some(program) = std::env::var_os("ProgramFiles") {
        candidates.push(Path::new(&program).join("nodejs\\node.exe"));
    }
    for candidate in candidates {
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join("node.exe");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

pub(crate) fn node_bin(app: &AppHandle) -> PathBuf {
    let private = private_node_bin(app);
    if private.is_file() {
        private
    } else {
        system_node().unwrap_or(private)
    }
}

pub(crate) fn dsh_entry(app: &AppHandle) -> PathBuf {
    runtime_dir(app).join("node_modules/@deepseek-ai/dsh/lib/bin.js")
}

pub(crate) fn harness_package_manifest(app: &AppHandle) -> PathBuf {
    runtime_dir(app).join("node_modules/@deepseek-ai/dsh/package.json")
}

pub(crate) fn profile_dir(app: &AppHandle) -> PathBuf {
    app_data(app).join("dsh/profiles/web")
}

pub(crate) fn npm_bin(app: &AppHandle) -> PathBuf {
    if private_node_bin(app).is_file() {
        node_dir(app).join("node_modules/npm/bin/npm-cli.js")
    } else {
        system_node()
            .and_then(|exe| {
                exe.parent()
                    .map(|dir| dir.join("node_modules/npm/bin/npm-cli.js"))
            })
            .unwrap_or_else(|| node_dir(app).join("node_modules/npm/bin/npm-cli.js"))
    }
}

pub(crate) fn valid_runtime(app: &AppHandle) -> bool {
    node_bin(app).is_file() && dsh_entry(app).is_file()
}

pub(crate) fn marketplace_installed(app: &AppHandle) -> bool {
    let manifest = profile_dir(app).join("package.json");
    fs::read_to_string(manifest)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| value.get("dependencies")?.get("dshmarket").cloned())
        .is_some()
}

pub(crate) fn emit_progress(app: &AppHandle, percentage: u8, detail: impl Into<String>) {
    let _ = app.emit(
        "runtime-progress",
        Progress {
            percentage,
            detail: detail.into(),
        },
    );
}

pub(crate) fn hidden(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
}

pub(crate) fn run_output(mut command: Command) -> Result<String, String> {
    hidden(&mut command);
    let output = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| error.to_string())?;
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if output.status.success() {
        Ok(text.trim().to_string())
    } else {
        Err(text.trim().to_string())
    }
}

async fn download_node(app: &AppHandle) -> Result<(), String> {
    emit_progress(app, 5, "正在读取 Node.js LTS 版本...");
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(900))
        .build()
        .map_err(|error| format!("网络初始化失败: {error}"))?;
    let releases: Vec<serde_json::Value> = client
        .get("https://nodejs.org/dist/index.json")
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;
    let release = releases
        .iter()
        .find(|release| {
            release
                .get("lts")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
                && release
                    .get("version")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .starts_with("v22")
        })
        .or_else(|| releases.first())
        .ok_or("Node.js 版本列表为空")?;
    let version = release
        .get("version")
        .and_then(|value| value.as_str())
        .ok_or("Node.js 版本无效")?;
    let archive_url = format!("https://nodejs.org/dist/{version}/node-{version}-win-x64.zip");

    emit_progress(app, 15, format!("正在下载 Node.js {version}..."));
    let archive_bytes = client
        .get(archive_url)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .bytes()
        .await
        .map_err(|error| error.to_string())?;
    let root = runtime_dir(app);
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    let archive_path = root.join("node.zip");
    fs::write(&archive_path, &archive_bytes).map_err(|error| error.to_string())?;

    let target = node_dir(app);
    if target.exists() {
        fs::remove_dir_all(&target).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&target).map_err(|error| error.to_string())?;
    let extract_target = target.clone();
    let extract_archive = archive_path.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let file = fs::File::open(&extract_archive).map_err(|error| error.to_string())?;
        let mut archive = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
            let name = entry.name().replace("\\", "/");
            let parts: Vec<&str> = name.split('/').collect();
            if parts.len() < 2 {
                continue;
            }
            let relative = parts[1..].join("/");
            if relative.is_empty() {
                continue;
            }
            let output = extract_target.join(relative);
            if entry.is_dir() {
                fs::create_dir_all(&output).map_err(|error| error.to_string())?;
            } else {
                if let Some(parent) = output.parent() {
                    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                let mut destination =
                    fs::File::create(&output).map_err(|error| error.to_string())?;
                std::io::copy(&mut entry, &mut destination).map_err(|error| error.to_string())?;
            }
        }
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())??;

    let _ = fs::remove_file(archive_path);
    emit_progress(app, 42, "Node.js 已准备完成");
    Ok(())
}

pub(crate) async fn install_runtime(app: AppHandle) -> Result<(), String> {
    let have_private = private_node_bin(&app).is_file();
    if !have_private && system_node().is_none() {
        download_node(&app).await?;
    }
    let node = node_bin(&app);
    if !node.is_file() {
        return Err("未找到可用的 Node.js 运行时".to_string());
    }
    emit_progress(&app, 52, "正在安装官方 DeepSeek Harness...");
    fs::create_dir_all(runtime_dir(&app)).map_err(|error| error.to_string())?;
    let mut command = Command::new(node);
    command
        .arg(npm_bin(&app))
        .args([
            "install",
            "--no-audit",
            "--no-fund",
            "--no-package-lock",
            "--prefix",
        ])
        .arg(runtime_dir(&app))
        .arg("@deepseek-ai/dsh@latest")
        .current_dir(runtime_dir(&app));
    run_output(command).map_err(|error| format!("Harness 安装失败: {error}"))?;
    emit_progress(&app, 90, "官方 DeepSeek Harness 已就绪");
    Ok(())
}

pub(crate) fn stop_harness_service() {
    #[cfg(windows)]
    {
        let script = r#"
$pids = @()
$pids += Get-NetTCPConnection -LocalPort 3080 -State Listen -ErrorAction SilentlyContinue |
    Select-Object -ExpandProperty OwningProcess -Unique
$pids += Get-CimInstance Win32_Process -Filter "Name='node.exe'" -ErrorAction SilentlyContinue |
    Where-Object { $_.CommandLine -like '*@deepseek-ai/dsh/lib/bin.js*' } |
    Select-Object -ExpandProperty ProcessId
$pids | Where-Object { $_ -and $_ -ne $PID } | Sort-Object -Unique | ForEach-Object {
    Stop-Process -Id $_ -Force -ErrorAction SilentlyContinue
}
"#;
        let mut command = Command::new("powershell.exe");
        command.args(["-NoProfile", "-NonInteractive", "-Command", script]);
        hidden(&mut command);
        let _ = command.output();
    }
}

pub(crate) async fn healthy() -> bool {
    reqwest::Client::new()
        .get("http://127.0.0.1:3080/")
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}
