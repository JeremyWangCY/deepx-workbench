use crate::{
    configure_runtime_environment, dsh_entry, emit_progress, harness_package_manifest, healthy,
    hidden, install_runtime, marketplace_installed, marketplace_version, migrate_private_plugins,
    node_bin, repair_marketplace_metadata, run_output_with_timeout, runtime_dir,
    seed_bundled_marketplace, stop_harness_service, update_runtime, valid_runtime,
    write_no_browser_patch,
};
use serde::Serialize;
use std::{
    fs,
    process::Command,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Position, Url};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Serialize)]
pub struct RuntimeStatus {
    pub ready: bool,
    pub version: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct VersionStatus {
    pub current: Option<String>,
    pub latest: Option<String>,
    pub update_available: bool,
}

#[derive(Debug, Serialize)]
pub struct UpdateStatus {
    pub deepx: VersionStatus,
    pub harness: VersionStatus,
    pub marketplace: VersionStatus,
}

#[derive(Debug, Serialize, Clone)]
pub struct UpdateProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub speed: u64,
    pub percent: u8,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct MarketplaceStatus {
    pub installed: bool,
    pub version: Option<String>,
}

fn package_version(manifest: std::path::PathBuf) -> Option<String> {
    let value = fs::read_to_string(manifest)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
    value?.get("version")?.as_str().map(str::to_owned)
}

#[tauri::command]
pub fn window_action(app: AppHandle, action: String) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "主窗口不存在".to_string())?;
    let result = match action.as_str() {
        "minimize" => window.minimize(),
        "toggle_maximize" => {
            let maximized = window.is_maximized().map_err(|error| error.to_string())?;
            if maximized {
                window.unmaximize()
            } else {
                window.maximize()
            }
        }
        "close" => window.close(),
        "start_dragging" => window.start_dragging(),
        _ => return Err(format!("不支持的窗口操作: {action}")),
    };
    result.map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_winbar_size(app: AppHandle, width: f64, height: f64) -> Result<(), String> {
    let winbar = app
        .get_webview_window("winbar")
        .ok_or_else(|| "winbar 不存在".to_string())?;
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "主窗口不存在".to_string())?;
    winbar
        .set_size(PhysicalSize::new(width as u32, height as u32))
        .map_err(|error| error.to_string())?;
    // Re-dock to the main window's top-right using the winbar's new width.
    if let (Ok(pos), Ok(size)) = (main.outer_position(), main.outer_size()) {
        let width_px = width as i32;
        let _ = winbar.set_position(Position::Physical(PhysicalPosition::new(
            pos.x + size.width as i32 - width_px,
            pos.y,
        )));
    }
    let _ = winbar.show();
    Ok(())
}

#[tauri::command]
pub fn runtime_status(app: AppHandle) -> RuntimeStatus {
    RuntimeStatus {
        ready: valid_runtime(&app),
        version: package_version(harness_package_manifest(&app)),
    }
}

fn version_status(current: Option<String>, latest: Option<String>) -> VersionStatus {
    let update_available = match (&current, &latest) {
        (Some(current), Some(latest)) => current != latest,
        (None, Some(_)) => true,
        _ => false,
    };
    VersionStatus {
        current,
        latest,
        update_available,
    }
}

async fn npm_latest_release(client: &reqwest::Client, package: &str) -> Option<String> {
    client
        .get(format!(
            "https://registry.npmjs.org/{}/latest",
            package.replace('/', "%2f")
        ))
        .send()
        .await
        .ok()?
        .json::<serde_json::Value>()
        .await
        .ok()?
        .get("version")
        .and_then(|value| value.as_str())
        .map(str::to_owned)
}

async fn github_latest(client: &reqwest::Client) -> Option<String> {
    client
        .get("https://api.github.com/repos/JeremyWangCY/deepx-workbench/releases/latest")
        .send()
        .await
        .ok()?
        .json::<serde_json::Value>()
        .await
        .ok()?
        .get("tag_name")
        .and_then(|value| value.as_str())
        .map(|value| value.trim_start_matches('v').to_string())
}

#[tauri::command]
pub async fn update_status(app: AppHandle) -> UpdateStatus {
    let current_deepx = Some(app.package_info().version.to_string());
    let current_harness = package_version(harness_package_manifest(&app));
    let current_marketplace = marketplace_version(&app);
    let client = reqwest::Client::builder()
        .user_agent("DeepX Workbench")
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20))
        .build()
        .ok();
    let (latest_deepx, latest_harness, latest_marketplace) = match client {
        Some(client) => (
            github_latest(&client).await,
            npm_latest_release(&client, "@deepseek-ai/dsh").await,
            npm_latest_release(&client, "dshmarket").await,
        ),
        None => (None, None, None),
    };
    UpdateStatus {
        deepx: version_status(current_deepx, latest_deepx),
        harness: version_status(current_harness, latest_harness),
        marketplace: version_status(current_marketplace, latest_marketplace),
    }
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
            let detail = log
                .chars()
                .rev()
                .take(1_500)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>();
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
    if healthy().await {
        if !migrated {
            return Ok(());
        }
        stop_current_harness().await?;
    }
    if migrated {
        repair_marketplace_metadata(&app)?;
    }
    let no_browser_patch = write_no_browser_patch(&app)?;
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
    configure_runtime_environment(&mut command, &app)?;
    let mut child = command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::from(
            log.try_clone().map_err(|error| error.to_string())?,
        ))
        .stderr(std::process::Stdio::from(log))
        .spawn()
        .map_err(|error| error.to_string())?;

    wait_for_harness(&mut child, &log_path).await
}

async fn navigate_to_harness(app: AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("主窗口不存在")?;
    window
        .navigate(Url::parse("http://127.0.0.1:3080/").map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn show_harness(app: AppHandle) -> Result<(), String> {
    navigate_to_harness(app).await
}

#[tauri::command]
pub async fn reload_harness(app: AppHandle) -> Result<(), String> {
    navigate_to_harness(app).await
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
        let cache = app
            .path()
            .app_cache_dir()
            .map_err(|error| error.to_string())?;
        fs::create_dir_all(&cache).map_err(|error| error.to_string())?;
        let installer = cache.join("deepx-workbench-update.exe");
        let _ = fs::remove_file(&installer);
        emit_progress(&app, 25, format!("正在下载 DeepX {tag}..."));
        let mut response = client
            .get(asset_url)
            .send()
            .await
            .map_err(|error| format!("下载 DeepX 更新失败: {error}"))?
            .error_for_status()
            .map_err(|error| format!("下载 DeepX 更新失败: {error}"))?;
        let total = response.content_length();
        let mut file = tokio::fs::File::create(&installer)
            .await
            .map_err(|error| format!("保存 DeepX 更新失败: {error}"))?;
        let started = Instant::now();
        let mut last_emit = Instant::now();
        let mut last_bytes: u64 = 0;
        let mut downloaded: u64 = 0;
        let mut speed: u64 = 0;
        let bytes_per_second = |bytes: u64, since: &mut Instant| -> u64 {
            let now = Instant::now();
            let millis = now.duration_since(*since).as_millis().max(1) as u64;
            *since = now;
            bytes * 1000 / millis
        };
        loop {
            let chunk = match response.chunk().await {
                Ok(Some(chunk)) => chunk,
                Ok(None) => break,
                Err(error) => {
                    let _ = fs::remove_file(&installer);
                    return Err(format!("读取 DeepX 更新失败: {error}"));
                }
            };
            if downloaded == 0 && !chunk.starts_with(b"MZ") {
                let _ = fs::remove_file(&installer);
                return Err("下载的 DeepX 安装包无效".to_string());
            }
            if let Err(error) = file.write_all(&chunk).await {
                let _ = fs::remove_file(&installer);
                return Err(format!("保存 DeepX 更新失败: {error}"));
            }
            downloaded += chunk.len() as u64;
            if started.elapsed().as_millis() > 0 && last_emit.elapsed().as_millis() >= 200 {
                speed = bytes_per_second(downloaded - last_bytes.min(downloaded), &mut last_emit);
                last_bytes = downloaded;
                let percent = match total {
                    Some(t) if t > 0 => ((downloaded as f64 / t as f64) * 100.0) as u8,
                    _ => 0,
                };
                let _ = app.emit(
                    "deepx-update-progress",
                    UpdateProgress {
                        downloaded,
                        total,
                        speed,
                        percent: percent.min(99),
                        detail: format!("正在下载 DeepX {tag}"),
                    },
                );
            }
        }
        if let Err(error) = file.flush().await {
            let _ = fs::remove_file(&installer);
            return Err(format!("保存 DeepX 更新失败: {error}"));
        }
        if let Err(error) = file.sync_all().await {
            let _ = fs::remove_file(&installer);
            return Err(format!("保存 DeepX 更新失败: {error}"));
        }
        let _ = app.emit(
            "deepx-update-progress",
            UpdateProgress {
                downloaded,
                total,
                speed,
                percent: 100,
                detail: "下载完成".to_string(),
            },
        );
        emit_progress(&app, 90, "正在启动 DeepX 更新安装器...");
        Command::new(&installer)
            // /S silent install (skips the remove-previous dialog that hangs
            // GUI mode when previous installs are half-broken), /R relaunches
            // the freshly installed app when the install finishes.
            .args(["/S", "/R"])
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
    if !marketplace_installed(&app) {
        emit_progress(&app, 92, "正在准备插件市场...");
        install_marketplace(app.clone()).await?;
    }
    emit_progress(&app, 94, "正在启动...");
    if !healthy().await {
        launch_harness(app.clone()).await?;
    }
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
        version: marketplace_version(&app),
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
    if migrated {
        repair_marketplace_metadata(&app)?;
    }
    let seeded = seed_bundled_marketplace(&app)?;
    if seeded {
        emit_progress(&app, 80, "插件市场已准备");
    } else {
        emit_progress(&app, 55, "正在安装 / 更新 dshmarket...");
        let mut command = Command::new(node_bin(&app));
        command.arg(dsh_entry(&app)).args([
            "plugin",
            "--profile",
            "web",
            "add",
            "dshmarket@latest",
            "--config.minimumReleaseAge=0",
        ]);
        configure_runtime_environment(&mut command, &app)?;
        run_output_with_timeout(command, Duration::from_secs(300))
            .map_err(|error| format!("插件市场安装失败: {error}"))?;
    }
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
