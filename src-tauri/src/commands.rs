use crate::{
    dsh_entry, emit_progress, harness_package_manifest, healthy, hidden, install_runtime,
    marketplace_installed, migrate_private_plugins, node_bin, overlay_script, run_output,
    set_update_channel, stop_harness_service, update_channel, valid_runtime,
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

async fn wait_for_harness() -> bool {
    for _ in 0..60 {
        if healthy().await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    false
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
    hidden(&mut command);
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|error| error.to_string())?;

    if wait_for_harness().await {
        Ok(())
    } else {
        Err("Harness 服务启动超时".to_string())
    }
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
pub async fn update_harness(app: AppHandle) -> Result<(), String> {
    emit_progress(&app, 25, "正在停止旧的 Harness 服务...");
    stop_current_harness().await?;
    install_runtime(app.clone()).await?;
    emit_progress(&app, 94, "正在启动 DeepSeek Harness...");
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
    run_output(command).map_err(|error| format!("插件市场安装失败: {error}"))?;
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
