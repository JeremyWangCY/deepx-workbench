use crate::{
    dsh_entry, emit_progress, harness_package_manifest, healthy, hidden, install_runtime,
    marketplace_installed, migrate_private_plugins, node_bin, overlay_script, run_output_with_timeout,
    runtime_dir, set_update_channel, stop_harness_service, update_channel, update_runtime, valid_runtime,
    write_no_browser_patch, UpdateChannel,
};
use serde::{Deserialize, Serialize};
use std::{fs, process::Command, time::Duration};
use tauri::{AppHandle, Manager, Url};

#[derive(Debug, Serialize)]
pub struct RuntimeStatus {
    pub ready: bool,
    pub version: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateStatus {
    pub installed_version: Option<String>,
    pub latest_version: Option<String>,
    pub update_available: bool,
}

#[derive(Debug, Serialize)]
pub struct MarketplaceStatus {
    pub installed: bool,
}

#[derive(Debug, Serialize)]
pub struct ChannelStatus {
    pub channel: UpdateChannel,
}

#[derive(Debug, Deserialize)]
pub struct ChannelSelection {
    pub channel: UpdateChannel,
}

fn package_version(manifest: std::path::PathBuf) -> Option<String> {
    let value = fs::read_to_string(manifest)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
    value?.get("version")?.as_str().map(str::to_owned)
}

#[tauri::command]
pub fn runtime_status(app: AppHandle) -> RuntimeStatus {
    RuntimeStatus {
        ready: valid_runtime(&app),
        version: package_version(harness_package_manifest(&app)),
    }
}

#[tauri::command]
pub async fn update_status(app: AppHandle) -> UpdateStatus {
    let channel = update_channel(&app);
    let url = match channel {
        UpdateChannel::Latest => "https://registry.npmjs.org/@deepseek-ai%2Fdsh/latest",
        UpdateChannel::Next => "https://registry.npmjs.org/@deepseek-ai%2Fdsh",
    };
    let response = match reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20))
        .build()
    {
        Ok(client) => client.get(url).send().await.ok(),
        Err(_) => None,
    };
    let latest = match response {
        Some(response) => response
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|value| match channel {
                UpdateChannel::Latest => value.get("version")?.as_str().map(str::to_owned),
                UpdateChannel::Next => value
                    .pointer("/dist-tags/next")?
                    .as_str()
                    .map(str::to_owned),
            }),
        None => None,
    };
    let installed = package_version(harness_package_manifest(&app));
    let update_available = match (&installed, &latest) {
        (Some(installed), Some(latest)) => installed != latest,
        _ => false,
    };
    UpdateStatus {
        installed_version: installed,
        latest_version: latest,
        update_available,
    }
}

#[tauri::command]
pub fn get_update_channel(app: AppHandle) -> ChannelStatus {
    ChannelStatus {
        channel: update_channel(&app),
    }
}

#[tauri::command]
pub fn select_update_channel(app: AppHandle, selection: ChannelSelection) -> Result<(), String> {
    set_update_channel(&app, selection.channel)
}

async fn wait_for_harness(
    child: &mut std::process::Child,
    log_path: &std::path::Path,
) -> Result<(), String> {
    for _ in 0..180 {
        if healthy().await {
            return Ok(());
        }
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            let log = fs::read_to_string(log_path).unwrap_or_default();
            let detail = log.chars().rev().take(1_500).collect::<String>().chars().rev().collect::<String>();
            return Err(if detail.trim().is_empty() {
                format!("Harness 启动失败（退出码 {:?}）", status.code())
            } else {
                format!("Harness 启动失败：{}", detail.trim())
            });
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let _ = child.kill();
    Err("Harness 启动超时".to_string())
}

async fn stop_current_harness() -> Result<(), String> {
    stop_harness_service();
    for _ in 0..20 {
        if !healthy().await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    Err("旧的 Harness 服务停止超时".to_string())
}

#[tauri::command]
pub async fn launch_harness(app: AppHandle) -> Result<(), String> {
    let migrated = migrate_private_plugins(&app)?;
    let no_browser_patch = write_no_browser_patch(&app)?;
    if healthy().await {
        if !migrated {
            return Ok(());
        }
        stop_current_harness().await?;
    }
    if !valid_runtime(&app) {
        return Err("Harness 尚未安装".to_string());
    }

    let mut command = Command::new(node_bin(&app));
    command.arg(dsh_entry(&app)).args([
        "--profile",
        "web",
        "--patch",
        &no_browser_patch.to_string_lossy(),
        "--no-open",
        "--port",
        "3080",
    ]);
    let log_path = runtime_dir(&app).join("harness-startup.log");
    let log = fs::File::create(&log_path).map_err(|error| error.to_string())?;
    hidden(&mut command);
    let mut child = command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::from(log.try_clone().map_err(|error| error.to_string())?))
        .stderr(std::process::Stdio::from(log))
        .spawn()
        .map_err(|error| error.to_string())?;

    wait_for_harness(&mut child, &log_path).await
}

#[tauri::command]
pub async fn show_harness(app: AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("主窗口不存在")?;
    window
        .navigate(Url::parse("http://127.0.0.1:3080/").map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(250)).await;
        if window.eval(overlay_script()).is_ok() {
            return Ok(());
        }
    }
    Err("无法初始化 DeepX 控件".to_string())
}

#[tauri::command]
pub async fn update_deepx(app: AppHandle) -> Result<(), String> {
    #[cfg(windows)]
    {
        emit_progress(&app, 5, "正在检查 DeepX 最新版本...");
        let client = reqwest::Client::builder()
            .user_agent("DeepX Workbench")
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(900))
            .build()
            .map_err(|error| format!("更新网络初始化失败: {error}"))?;
        let release = client
            .get("https://api.github.com/repos/JeremyWangCY/deepx-workbench/releases/latest")
            .send()
            .await
            .map_err(|error| format!("检查 DeepX 更新失败: {error}"))?
            .error_for_status()
            .map_err(|error| format!("检查 DeepX 更新失败: {error}"))?
            .json::<serde_json::Value>()
            .await
            .map_err(|error| format!("读取 DeepX 更新信息失败: {error}"))?;
        let tag = release
            .get("tag_name")
            .and_then(|value| value.as_str())
            .ok_or("最新 DeepX Release 缺少版本号")?;
        if tag.trim_start_matches('v') == app.package_info().version.to_string() {
            emit_progress(&app, 100, "DeepX 已是最新版本");
            return Ok(());
        }
        let asset_url = release
            .get("assets")
            .and_then(|value| value.as_array())
            .and_then(|assets| {
                assets.iter().find_map(|asset| {
                    let name = asset.get("name")?.as_str()?;
                    let url = asset.get("browser_download_url")?.as_str()?;
                    name.ends_with("_x64-setup.exe").then_some(url)
                })
            })
            .filter(|url| {
                url.starts_with(
                    "https://github.com/JeremyWangCY/deepx-workbench/releases/download/",
                )
            })
            .ok_or("最新 DeepX Release 中没有 Windows x64 安装包")?;
        let cache = app.path().app_cache_dir().map_err(|error| error.to_string())?;
        fs::create_dir_all(&cache).map_err(|error| error.to_string())?;
        let installer = cache.join("deepx-workbench-update.exe");
        emit_progress(&app, 25, format!("正在下载 DeepX {tag}..."));
        let bytes = client
            .get(asset_url)
            .send()
            .await
            .map_err(|error| format!("下载 DeepX 更新失败: {error}"))?
            .error_for_status()
            .map_err(|error| format!("下载 DeepX 更新失败: {error}"))?
            .bytes()
            .await
            .map_err(|error| format!("读取 DeepX 更新失败: {error}"))?;
        if !bytes.starts_with(b"MZ") {
            return Err("下载的 DeepX 安装包无效".to_string());
        }
        fs::write(&installer, &bytes).map_err(|error| format!("保存 DeepX 更新失败: {error}"))?;
        emit_progress(&app, 90, "正在启动 DeepX 更新安装器...");
        Command::new(&installer)
            .spawn()
            .map_err(|error| format!("启动 DeepX 更新安装器失败: {error}"))?;
        let app_for_exit = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(750));
            app_for_exit.exit(0);
        });
        Ok(())
    }

    #[cfg(not(windows))]
    {
        let _ = app;
        Err("DeepX 更新仅支持 Windows 安装包".to_string())
    }
}
#[tauri::command]
pub async fn initialize_harness(app: AppHandle) -> Result<(), String> {
    install_runtime(app.clone()).await?;
    emit_progress(&app, 94, "正在启动...");
    launch_harness(app.clone()).await?;
    show_harness(app).await
}

#[tauri::command]
pub async fn update_harness(app: AppHandle) -> Result<(), String> {
    emit_progress(&app, 25, "正在更新...");
    stop_current_harness().await?;
    update_runtime(app.clone()).await?;
    emit_progress(&app, 94, "正在启动...");
    launch_harness(app.clone()).await?;
    show_harness(app).await
}

#[tauri::command]
pub fn marketplace_status(app: AppHandle) -> MarketplaceStatus {
    MarketplaceStatus {
        installed: marketplace_installed(&app),
    }
}

#[tauri::command]
pub async fn install_marketplace(app: AppHandle) -> Result<(), String> {
    emit_progress(&app, 15, "正在准备插件市场...");
    if !valid_runtime(&app) {
        install_runtime(app.clone()).await?;
    }
    let migrated = migrate_private_plugins(&app)?;
    if migrated && healthy().await {
        emit_progress(&app, 35, "正在切换到共享插件目录...");
        stop_current_harness().await?;
    }
    emit_progress(&app, 55, "正在安装 / 更新 dshmarket...");

    let mut command = Command::new(node_bin(&app));
    command
        .arg(dsh_entry(&app))
        .args(["plugin", "--profile", "web", "add", "dshmarket"]);
    run_output_with_timeout(command, Duration::from_secs(300))
        .map_err(|error| format!("插件市场安装失败: {error}"))?;
    if !marketplace_installed(&app) {
        return Err("插件市场命令已完成，但未在 web 配置中找到 dshmarket".to_string());
    }
    if !healthy().await {
        launch_harness(app.clone()).await?;
        show_harness(app.clone()).await?;
    }
    emit_progress(&app, 100, "插件市场已就绪");
    Ok(())
}
