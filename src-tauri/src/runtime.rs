use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicU16, Ordering},
        Mutex, OnceLock,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, Url};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub percentage: u8,
    pub detail: String,
}

const RUNTIME_MARKER: &str = ".deepx-runtime-ready";

static HARNESS_PORT: AtomicU16 = AtomicU16::new(0);
static HARNESS_BOOT_URL: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn harness_boot_url_state() -> &'static Mutex<Option<String>> {
    HARNESS_BOOT_URL.get_or_init(|| Mutex::new(None))
}

pub(crate) fn remember_harness_port(port: u16) {
    if port > 0 {
        HARNESS_PORT.store(port, Ordering::SeqCst);
    }
}

pub(crate) fn remember_harness_launch_url(url: &str) -> Option<u16> {
    let parsed = Url::parse(url).ok()?;
    if parsed.scheme() != "http" || parsed.host_str() != Some("127.0.0.1") {
        return None;
    }
    let port = parsed.port()?;
    remember_harness_port(port);
    if parsed.query_pairs().any(|(key, _)| key == "token") {
        if let Ok(mut state) = harness_boot_url_state().lock() {
            *state = Some(url.to_string());
        }
    }
    Some(port)
}

pub(crate) fn take_harness_boot_url() -> Option<String> {
    harness_boot_url_state()
        .lock()
        .ok()
        .and_then(|mut state| state.take())
}

pub(crate) fn clear_harness_endpoint() {
    HARNESS_PORT.store(0, Ordering::SeqCst);
    if let Ok(mut state) = harness_boot_url_state().lock() {
        *state = None;
    }
}

const REQUIRED_DSH_PEERS: [&str; 28] = [
    "@deepseek-ai/cordis-plugin-group",
    "@deepseek-ai/dsh-anonymous-user-id",
    "@deepseek-ai/dsh-atomic-write",
    "@deepseek-ai/dsh-attachment",
    "@deepseek-ai/dsh-authorization",
    "@deepseek-ai/dsh-bash-local",
    "@deepseek-ai/dsh-code-runtime",
    "@deepseek-ai/dsh-compaction",
    "@deepseek-ai/dsh-fs",
    "@deepseek-ai/dsh-hook-protocol",
    "@deepseek-ai/dsh-invariants",
    "@deepseek-ai/dsh-jobs",
    "@deepseek-ai/dsh-output-retention",
    "@deepseek-ai/dsh-sandbox",
    "@deepseek-ai/dsh-scope",
    "@deepseek-ai/dsh-sdk-protocol",
    "@deepseek-ai/dsh-session-persistence",
    "@deepseek-ai/dsh-session-query",
    "@deepseek-ai/dsh-session-telemetry",
    "@deepseek-ai/dsh-session-title-llm",
    "@deepseek-ai/dsh-settings",
    "@deepseek-ai/dsh-shell",
    "@deepseek-ai/dsh-spill",
    "@deepseek-ai/dsh-subagent-in-process-driver",
    "@deepseek-ai/dsh-timeout",
    "@deepseek-ai/dsh-util-time",
    "@deepseek-ai/dsh-util-workspace-path",
    "@deepseek-ai/dsh-workflow",
];

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

pub(crate) fn node_bin(app: &AppHandle) -> PathBuf {
    private_node_bin(app)
}
pub(crate) fn dsh_entry(app: &AppHandle) -> PathBuf {
    runtime_dir(app).join("node_modules/@deepseek-ai/dsh/lib/bin.js")
}

pub(crate) fn harness_package_manifest(app: &AppHandle) -> PathBuf {
    runtime_dir(app).join("node_modules/@deepseek-ai/dsh/package.json")
}

pub(crate) fn dsh_home(app: &AppHandle) -> Result<PathBuf, String> {
    let home = app.path().home_dir().map_err(|error| error.to_string())?;
    Ok(home.join(".dsh"))
}

fn legacy_dsh_home(app: &AppHandle) -> PathBuf {
    app_data(app).join("dsh")
}

pub(crate) fn profile_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(dsh_home(app)?.join("profiles/web"))
}

pub(crate) fn pnpm_package_manifest(app: &AppHandle) -> PathBuf {
    runtime_dir(app).join("node_modules/pnpm/package.json")
}

pub(crate) fn pnpm_cmd(app: &AppHandle) -> PathBuf {
    runtime_dir(app).join("bin/pnpm.cmd")
}

fn configure_runtime_environment_at(
    command: &mut Command,
    app: &AppHandle,
    root: &Path,
) -> Result<(), String> {
    let mut paths = vec![root.join("bin"), root.join("node")];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    command.env(
        "PATH",
        std::env::join_paths(paths).map_err(|error| error.to_string())?,
    );
    command.env("COREPACK_HOME", root.join("corepack"));
    // Do NOT override PNPM_HOME to runtime_dir/bin: on Windows, pnpm derives its store
    // path relative to PNPM_HOME, causing ERR_PNPM_UNEXPECTED_STORE mismatch against
    // %LOCALAPPDATA%\pnpm\store\v11 recorded in node_modules/.modules.yaml.
    // runtime_dir/bin is already first in PATH, so bundled pnpm is resolved directly.
    command.env("npm_config_node_linker", "hoisted");
    if let Ok(home) = app.path().home_dir() {
        command.env("DSH_AGENTS_HOME", home.join(".agents"));
    }
    command.env("GIT_TERMINAL_PROMPT", "0");
    Ok(())
}

pub(crate) fn configure_runtime_environment(
    command: &mut Command,
    app: &AppHandle,
) -> Result<(), String> {
    configure_runtime_environment_at(command, app, &runtime_dir(app))
}

pub(crate) fn runtime_marker(app: &AppHandle) -> PathBuf {
    runtime_dir(app).join(RUNTIME_MARKER)
}

pub(crate) fn valid_runtime(app: &AppHandle) -> bool {
    node_bin(app).is_file()
        && dsh_entry(app).is_file()
        && pnpm_cmd(app).is_file()
        && pnpm_package_manifest(app).is_file()
        && runtime_marker(app).is_file()
}

fn package_version_at(manifest_path: &Path) -> Result<String, String> {
    let manifest = fs::read_to_string(manifest_path).map_err(|error| error.to_string())?;
    serde_json::from_str::<serde_json::Value>(&manifest)
        .map_err(|error| error.to_string())?
        .get("version")
        .and_then(|value| value.as_str())
        .map(str::to_owned)
        .ok_or_else(|| "Harness 版本信息无效".to_string())
}

fn aligned_peer_packages_from_manifest(manifest_path: &Path, version: &str) -> Vec<String> {
    let mut packages: Vec<String> = REQUIRED_DSH_PEERS.iter().map(|s| s.to_string()).collect();
    if let Ok(manifest_content) = fs::read_to_string(manifest_path) {
        if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&manifest_content) {
            for section in ["peerDependencies", "dependencies"] {
                if let Some(deps) = manifest.get(section).and_then(|v| v.as_object()) {
                    for key in deps.keys() {
                        if key.starts_with("@deepseek-ai/")
                            && key != "@deepseek-ai/dsh"
                            && !packages.contains(key)
                        {
                            packages.push(key.clone());
                        }
                    }
                }
            }
        }
    }
    packages
        .into_iter()
        .map(|package| {
            if package == "@deepseek-ai/cordis-plugin-group"
                || !package.starts_with("@deepseek-ai/dsh")
            {
                package
            } else {
                format!("{package}@{version}")
            }
        })
        .collect()
}

pub(crate) fn marketplace_version(app: &AppHandle) -> Option<String> {
    let profile = profile_dir(app).ok()?;
    fs::read_to_string(profile.join("node_modules/dshmarket/package.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| value.get("version")?.as_str().map(str::to_owned))
}

pub(crate) fn marketplace_installed(app: &AppHandle) -> bool {
    let profile = match profile_dir(app) {
        Ok(profile) => profile,
        Err(_) => return false,
    };
    marketplace_version(app).is_some()
        && profile
            .join("node_modules/dshmarket/lib/index.js")
            .is_file()
}

pub(crate) fn ensure_profile_store_compatibility(app: &AppHandle) -> Result<(), String> {
    let profile = profile_dir(app)?;
    let modules_manifest = profile.join("node_modules/.modules.yaml");
    if modules_manifest.is_file() {
        if let Ok(content) = fs::read_to_string(&modules_manifest) {
            let needs_reset = content.contains("com.jeremy.deepx-workbench")
                || content.contains("runtime\\bin\\store")
                || content.contains("runtime/bin/store");
            if needs_reset {
                let _ = fs::remove_file(&modules_manifest);
            }
        }
    }
    Ok(())
}

pub(crate) fn repair_marketplace_metadata(app: &AppHandle) -> Result<(), String> {
    let profile = profile_dir(app)?;
    let modules_manifest = profile.join("node_modules/.modules.yaml");
    let workspace_state = profile.join("node_modules/.pnpm-workspace-state-v1.json");
    let virtual_store = profile.join("node_modules/.pnpm");
    if modules_manifest.is_file() {
        fs::remove_file(modules_manifest).map_err(|error| error.to_string())?;
    }
    if workspace_state.is_file() {
        fs::remove_file(workspace_state).map_err(|error| error.to_string())?;
    }
    if virtual_store.is_dir() {
        fs::remove_dir_all(virtual_store).map_err(|error| error.to_string())?;
    }
    if let Ok(entries) = fs::read_dir(&profile) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.ends_with(".tmp") || name_str.ends_with(".lock.tmp") {
                if path.is_dir() {
                    let _ = fs::remove_dir_all(&path);
                } else {
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }
    Ok(())
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

pub(crate) fn run_output_with_timeout(
    mut command: Command,
    timeout: Duration,
) -> Result<String, String> {
    hidden(&mut command);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let started = Instant::now();

    loop {
        if child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_some()
        {
            let output = child
                .wait_with_output()
                .map_err(|error| error.to_string())?;
            let text = format!(
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return if output.status.success() {
                Ok(text.trim().to_string())
            } else {
                Err(text.trim().to_string())
            };
        }

        if started.elapsed() >= timeout {
            let _ = child.kill();
            let output = child
                .wait_with_output()
                .map_err(|error| error.to_string())?;
            let text = format!(
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
            .trim()
            .to_string();
            let detail = if text.is_empty() {
                String::new()
            } else {
                format!("\n{text}")
            };
            return Err(format!(
                "命令在 {} 秒内未完成，已停止。请检查网络后重试。{detail}",
                timeout.as_secs()
            ));
        }

        std::thread::sleep(Duration::from_millis(200));
    }
}

pub(crate) fn seed_bundled_marketplace(app: &AppHandle) -> Result<bool, String> {
    if marketplace_installed(app) {
        return Ok(false);
    }
    let destination = profile_dir(app)?;
    if destination.join("package.json").is_file() {
        return Ok(false);
    }
    let source = match bundled_runtime_dir(app) {
        Ok(dir) => dir.join("marketplace-profile"),
        Err(_) => runtime_dir(app).join("marketplace-profile"),
    };
    if !source.is_dir() {
        return Ok(false);
    }
    copy_directory(&source, &destination)?;
    let modules_manifest = destination.join("node_modules/.modules.yaml");
    let workspace_state = destination.join("node_modules/.pnpm-workspace-state-v1.json");
    let virtual_store = destination.join("node_modules/.pnpm");
    if modules_manifest.is_file() {
        fs::remove_file(modules_manifest).map_err(|error| error.to_string())?;
    }
    if workspace_state.is_file() {
        fs::remove_file(workspace_state).map_err(|error| error.to_string())?;
    }
    if virtual_store.is_dir() {
        fs::remove_dir_all(virtual_store).map_err(|error| error.to_string())?;
    }
    Ok(marketplace_installed(app))
}

fn bundled_runtime_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|error| error.to_string())?;
    let node = if cfg!(windows) {
        "node/node.exe"
    } else {
        "node/bin/node"
    };
    [
        resource_dir.join("runtime"),
        resource_dir.join("resources/runtime"),
    ]
    .into_iter()
    .find(|candidate| {
        candidate.join(node).is_file()
            && candidate
                .join("node_modules/@deepseek-ai/dsh/lib/bin.js")
                .is_file()
    })
    .ok_or_else(|| "安装包内缺少 DeepSeek Harness 运行时，请重新下载安装包".to_string())
}

fn copy_directory(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let target = destination.join(entry.file_name());
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            copy_directory(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn copy_runtime_for_update(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name();
        let name_text = name.to_string_lossy();
        if name_text == "harness-startup.log"
            || name_text == "harness-startup.previous.log"
            || name_text == "harness-supervisor.log"
            || name_text.starts_with("reinstall-backup-")
        {
            continue;
        }

        // pnpm's content-addressable store is disposable cache data and can contain
        // junctions/symlinks back into profiles or temporary projects. Never carry it
        // into a transactional runtime candidate; npm/pnpm will recreate what it needs.
        if source.ends_with("bin") && name_text == "store" {
            continue;
        }

        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        // Do not dereference runtime symlinks while staging an update. Copying their
        // targets would make the candidate depend on unrelated user/profile paths.
        if file_type.is_symlink() {
            continue;
        }

        let target = destination.join(&name);
        if file_type.is_dir() {
            copy_runtime_for_update(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn recover_interrupted_runtime_swap(app: &AppHandle) -> Result<bool, String> {
    if valid_runtime(app) {
        return Ok(false);
    }

    let current = runtime_dir(app);
    let previous = app_data(app).join("runtime.previous");
    if !previous.is_dir() || validate_runtime_at(&previous).is_err() {
        return Ok(false);
    }

    let failed = app_data(app).join("runtime.failed");
    if failed.exists() {
        fs::remove_dir_all(&failed).map_err(|error| error.to_string())?;
    }
    if current.exists() {
        fs::rename(&current, &failed)
            .map_err(|error| format!("保存中断更新留下的运行时失败: {error}"))?;
    }
    if let Err(error) = fs::rename(&previous, &current) {
        if failed.exists() {
            let _ = fs::rename(&failed, &current);
        }
        return Err(format!("恢复中断更新前的 Harness 运行时失败: {error}"));
    }
    Ok(true)
}

pub(crate) async fn install_runtime(app: AppHandle) -> Result<(), String> {
    let restored = recover_interrupted_runtime_swap(&app)?;
    if valid_runtime(&app) {
        if restored {
            emit_progress(&app, 90, "已恢复上一次更新前的 Harness 运行时");
        }
        return Ok(());
    }

    match bundled_runtime_dir(&app) {
        Ok(source) => {
            let destination = runtime_dir(&app);
            emit_progress(&app, 12, "正在安装...");
            tauri::async_runtime::spawn_blocking(move || {
                if destination.exists() {
                    fs::remove_dir_all(&destination).map_err(|error| error.to_string())?;
                }
                copy_directory(&source, &destination)
            })
            .await
            .map_err(|error| format!("内置运行时复制任务异常: {error}"))??;
        }
        Err(_) => {
            download_runtime_archive(&app).await?;
        }
    }

    if !valid_runtime(&app) {
        return Err("内置DeepSeek Harness 运行时不完整，请重新下载安装包".to_string());
    }
    emit_progress(&app, 90, "安装完成");
    Ok(())
}

async fn download_runtime_archive(app: &AppHandle) -> Result<(), String> {
    let version = app.package_info().version.to_string();
    let asset = format!("deepx-runtime-v{version}.zip");
    let url = format!(
        "https://github.com/JeremyWangCY/deepx-workbench/releases/download/v{version}/{asset}"
    );
    let cache = app
        .path()
        .app_cache_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&cache).map_err(|error| error.to_string())?;
    let archive = cache.join(&asset);
    let _ = fs::remove_file(&archive);

    emit_progress(
        app,
        15,
        format!("正在下载 DeepSeek Harness 运行时 v{version}..."),
    );
    let client = reqwest::Client::new();
    let mut response = client
        .get(&url)
        .send()
        .await
        .map_err(|error| format!("下载 Harness 运行时失败: {error}"))?
        .error_for_status()
        .map_err(|error| format!("下载 Harness 运行时失败: {error}"))?;
    let total = response.content_length();

    let mut file = tokio::fs::File::create(&archive)
        .await
        .map_err(|error| format!("创建运行时缓存失败: {error}"))?;
    let mut downloaded: u64 = 0;
    let mut last_emit = Instant::now();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("下载 Harness 运行时失败: {error}"))?
    {
        downloaded += chunk.len() as u64;
        file.write_all(&chunk)
            .await
            .map_err(|error| format!("写入运行时缓存失败: {error}"))?;
        if last_emit.elapsed().as_millis() >= 300 {
            last_emit = Instant::now();
            let percent = total
                .map(|t| {
                    if t == 0 {
                        15
                    } else {
                        (15.0 + (downloaded as f64 / t as f64) * 55.0).min(70.0) as u8
                    }
                })
                .unwrap_or(20);
            emit_progress(
                app,
                percent,
                format!("正在下载 DeepSeek Harness 运行时 v{version}..."),
            );
        }
    }
    file.flush().await.map_err(|error| error.to_string())?;
    drop(file);

    let destination = runtime_dir(app);
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    emit_progress(app, 74, "正在解压运行时...");
    if destination.exists() {
        fs::remove_dir_all(&destination).map_err(|error| error.to_string())?;
    }
    let status = std::process::Command::new("tar")
        .arg("-xf")
        .arg(&archive)
        .arg("-C")
        .arg(&app_data)
        .status()
        .map_err(|error| format!("解压 Harness 运行时失败: {error}"))?;
    if !status.success() {
        return Err("解压 Harness 运行时失败".to_string());
    }
    emit_progress(app, 88, "运行时就绪");
    Ok(())
}

fn runtime_node_bin_at(root: &Path) -> PathBuf {
    root.join(if cfg!(windows) {
        "node/node.exe"
    } else {
        "node/bin/node"
    })
}

fn runtime_npm_bin_at(root: &Path) -> PathBuf {
    root.join("node/node_modules/npm/bin/npm-cli.js")
}

fn runtime_dsh_manifest_at(root: &Path) -> PathBuf {
    root.join("node_modules/@deepseek-ai/dsh/package.json")
}

fn validate_runtime_at(root: &Path) -> Result<String, String> {
    let required = [
        runtime_node_bin_at(root),
        root.join("node_modules/@deepseek-ai/dsh/lib/bin.js"),
        root.join("bin/pnpm.cmd"),
        root.join("node_modules/pnpm/package.json"),
        root.join(RUNTIME_MARKER),
    ];
    if let Some(missing) = required.iter().find(|path| !path.is_file()) {
        return Err(format!("更新后的 Harness 运行时缺少 {}", missing.display()));
    }

    let manifest_path = runtime_dsh_manifest_at(root);
    let version = package_version_at(&manifest_path)?;
    let marker =
        fs::read_to_string(root.join(RUNTIME_MARKER)).map_err(|error| error.to_string())?;
    if marker.trim().trim_start_matches('﻿') != version {
        return Err(format!(
            "Harness 运行时版本标记不一致: marker={} package={version}",
            marker.trim()
        ));
    }

    for spec in aligned_peer_packages_from_manifest(&manifest_path, &version) {
        if !spec.starts_with("@deepseek-ai/dsh") {
            continue;
        }
        let suffix = format!("@{version}");
        let package = spec.strip_suffix(&suffix).unwrap_or(&spec);
        let package_manifest = root.join("node_modules").join(package).join("package.json");
        let installed = package_version_at(&package_manifest)
            .map_err(|error| format!("{package} 校验失败: {error}"))?;
        if installed != version {
            return Err(format!(
                "{package} 版本不一致: installed={installed} expected={version}"
            ));
        }
    }

    Ok(version)
}

pub(crate) async fn update_runtime(app: AppHandle) -> Result<(), String> {
    if !valid_runtime(&app) {
        install_runtime(app.clone()).await?;
    }

    let current = runtime_dir(&app);
    let staging = app_data(&app).join("runtime.staging");
    let previous = app_data(&app).join("runtime.previous");
    let failed = app_data(&app).join("runtime.failed");
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(|error| error.to_string())?;
    }
    if failed.exists() {
        let _ = fs::remove_dir_all(&failed);
    }

    emit_progress(&app, 45, "正在创建隔离更新副本...");
    let source = current.clone();
    let destination = staging.clone();
    tauri::async_runtime::spawn_blocking(move || copy_runtime_for_update(&source, &destination))
        .await
        .map_err(|error| format!("创建 Harness 更新副本异常: {error}"))??;

    let install_options = [
        "install",
        "--no-audit",
        "--no-fund",
        "--no-package-lock",
        "--legacy-peer-deps",
        "--fetch-timeout",
        "30000",
        "--fetch-retries",
        "1",
        "--maxsockets",
        "8",
        "--prefix",
    ];

    emit_progress(&app, 55, "正在隔离更新 Harness...");
    let staging_node = runtime_node_bin_at(&staging);
    let staging_npm = runtime_npm_bin_at(&staging);
    let mut command = Command::new(&staging_node);
    command
        .arg(&staging_npm)
        .args(install_options)
        .arg(&staging)
        .arg("@deepseek-ai/dsh@latest")
        .current_dir(&staging);
    configure_runtime_environment_at(&mut command, &app, &staging)?;
    if let Err(error) = run_output_with_timeout(command, Duration::from_secs(300)) {
        let _ = fs::remove_dir_all(&staging);
        return Err(format!("Harness 隔离更新失败，当前版本未受影响: {error}"));
    }

    let manifest_path = runtime_dsh_manifest_at(&staging);
    let version = match package_version_at(&manifest_path) {
        Ok(version) => version,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(format!("读取更新后的 Harness 版本失败: {error}"));
        }
    };

    emit_progress(&app, 68, format!("正在对齐 Harness {version} 依赖..."));
    let mut peer_command = Command::new(&staging_node);
    peer_command
        .arg(&staging_npm)
        .args(install_options)
        .arg(&staging)
        .args(aligned_peer_packages_from_manifest(
            &manifest_path,
            &version,
        ))
        .current_dir(&staging);
    configure_runtime_environment_at(&mut peer_command, &app, &staging)?;
    if let Err(error) = run_output_with_timeout(peer_command, Duration::from_secs(300)) {
        let _ = fs::remove_dir_all(&staging);
        return Err(format!(
            "Harness 依赖隔离更新失败，当前版本未受影响: {error}"
        ));
    }

    fs::write(staging.join(RUNTIME_MARKER), &version).map_err(|error| error.to_string())?;
    if let Err(error) = validate_runtime_at(&staging) {
        let _ = fs::remove_dir_all(&staging);
        return Err(format!("Harness 更新校验失败，当前版本未受影响: {error}"));
    }

    emit_progress(&app, 84, "正在原子切换 Harness 运行时...");
    if previous.exists() {
        fs::remove_dir_all(&previous)
            .map_err(|error| format!("无法清理上一版 Harness 备份，未执行切换: {error}"))?;
    }
    fs::rename(&current, &previous)
        .map_err(|error| format!("无法备份当前 Harness，未执行切换: {error}"))?;
    if let Err(error) = fs::rename(&staging, &current) {
        let restore = fs::rename(&previous, &current);
        return Err(match restore {
            Ok(()) => format!("切换新版 Harness 失败，已恢复旧版本: {error}"),
            Err(restore_error) => {
                format!("切换新版 Harness 失败且自动恢复失败: {error}; restore={restore_error}")
            }
        });
    }

    emit_progress(&app, 90, format!("Harness {version} 已更新并保留回滚副本"));
    Ok(())
}

pub(crate) fn rollback_runtime(app: &AppHandle) -> Result<bool, String> {
    let current = runtime_dir(app);
    let previous = app_data(app).join("runtime.previous");
    if !previous.is_dir() {
        return Ok(false);
    }

    let failed = app_data(app).join("runtime.failed");
    if failed.exists() {
        fs::remove_dir_all(&failed).map_err(|error| error.to_string())?;
    }
    if current.exists() {
        fs::rename(&current, &failed)
            .map_err(|error| format!("保存失败的 Harness 运行时失败: {error}"))?;
    }
    if let Err(error) = fs::rename(&previous, &current) {
        if failed.exists() {
            let _ = fs::rename(&failed, &current);
        }
        return Err(format!("恢复上一版 Harness 失败: {error}"));
    }
    Ok(true)
}

fn powershell_single_quoted(value: &str) -> String {
    value.replace('\'', "''")
}

fn harness_match_paths(app: &AppHandle) -> Option<(String, String)> {
    let entry = dsh_entry(app).to_string_lossy().replace('\\', "/");
    let patch = no_browser_patch_path(app)
        .ok()?
        .to_string_lossy()
        .replace('\\', "/");
    Some((
        powershell_single_quoted(&entry),
        powershell_single_quoted(&patch),
    ))
}

pub(crate) fn harness_process_running(app: &AppHandle) -> bool {
    #[cfg(windows)]
    {
        let Some((entry, patch)) = harness_match_paths(app) else {
            return false;
        };
        let script = format!(
            r#"$entry = '{entry}'
$patch = '{patch}'
$match = Get-CimInstance Win32_Process -Filter "Name='node.exe'" -ErrorAction SilentlyContinue |
    Where-Object {{
        $line = $_.CommandLine
        $normalized = if ($line) {{ $line.Replace('\', '/') }} else {{ '' }}
        $line -and
        $normalized.Contains($entry) -and
        $normalized.Contains($patch) -and
        $line.Contains('--profile web') -and
        $line.Contains('--no-open')
    }} |
    Select-Object -First 1
if ($match) {{ Write-Output '1' }}"#
        );
        let mut command = Command::new("powershell.exe");
        command.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        hidden(&mut command);
        command
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim() == "1")
            .unwrap_or(false)
    }

    #[cfg(not(windows))]
    {
        let _ = app;
        HARNESS_PORT.load(Ordering::SeqCst) > 0
    }
}

pub(crate) fn harness_child_port(pid: u32) -> Option<u16> {
    #[cfg(windows)]
    {
        let script = format!(
            "$listener = Get-NetTCPConnection -State Listen -OwningProcess {pid} -ErrorAction SilentlyContinue | Where-Object {{ $_.LocalAddress -eq '127.0.0.1' -or $_.LocalAddress -eq '::1' }} | Select-Object -First 1; if ($listener) {{ Write-Output $listener.LocalPort }}"
        );
        let mut command = Command::new("powershell.exe");
        command.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        hidden(&mut command);
        let port = command
            .output()
            .ok()
            .and_then(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .parse::<u16>()
                    .ok()
            })
            .filter(|port| *port > 0)?;
        remember_harness_port(port);
        Some(port)
    }

    #[cfg(not(windows))]
    {
        let _ = pid;
        let port = HARNESS_PORT.load(Ordering::SeqCst);
        (port > 0).then_some(port)
    }
}

pub(crate) fn harness_port(app: &AppHandle) -> Option<u16> {
    #[cfg(windows)]
    {
        let (entry, patch) = harness_match_paths(app)?;
        let script = format!(
            r#"$entry = '{entry}'
$patch = '{patch}'
$process = Get-CimInstance Win32_Process -Filter "Name='node.exe'" -ErrorAction SilentlyContinue |
    Where-Object {{
        $line = $_.CommandLine
        $normalized = if ($line) {{ $line.Replace('\', '/') }} else {{ '' }}
        $line -and
        $normalized.Contains($entry) -and
        $normalized.Contains($patch) -and
        $line.Contains('--profile web') -and
        $line.Contains('--no-open')
    }} |
    Select-Object -First 1
if ($process) {{
    $listener = Get-NetTCPConnection -State Listen -OwningProcess $process.ProcessId -ErrorAction SilentlyContinue |
        Where-Object {{ $_.LocalAddress -eq '127.0.0.1' -or $_.LocalAddress -eq '::1' }} |
        Select-Object -First 1
    if ($listener) {{ Write-Output $listener.LocalPort }}
}}"#
        );
        let mut command = Command::new("powershell.exe");
        command.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        hidden(&mut command);
        let port = command
            .output()
            .ok()
            .and_then(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .parse::<u16>()
                    .ok()
            })
            .filter(|port| *port > 0)?;
        remember_harness_port(port);
        Some(port)
    }

    #[cfg(not(windows))]
    {
        let _ = app;
        let port = HARNESS_PORT.load(Ordering::SeqCst);
        (port > 0).then_some(port)
    }
}

pub(crate) fn harness_base_url(app: &AppHandle) -> Option<String> {
    harness_port(app).map(|port| format!("http://127.0.0.1:{port}/"))
}

pub(crate) fn stop_harness_service(app: &AppHandle) {
    #[cfg(windows)]
    {
        let Some((entry, patch)) = harness_match_paths(app) else {
            return;
        };
        let script = format!(
            r#"$entry = '{entry}'
$patch = '{patch}'
Get-CimInstance Win32_Process -Filter "Name='node.exe'" -ErrorAction SilentlyContinue |
    Where-Object {{
        $line = $_.CommandLine
        $normalized = if ($line) {{ $line.Replace('\', '/') }} else {{ '' }}
        $line -and
        $normalized.Contains($entry) -and
        $normalized.Contains($patch) -and
        $line.Contains('--profile web') -and
        $line.Contains('--no-open')
    }} |
    Select-Object -ExpandProperty ProcessId -Unique |
    Where-Object {{ $_ -and $_ -ne $PID }} |
    ForEach-Object {{ Stop-Process -Id $_ -Force -ErrorAction SilentlyContinue }}"#
        );
        let mut command = Command::new("powershell.exe");
        command.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        hidden(&mut command);
        let _ = command.output();
    }

    #[cfg(not(windows))]
    {
        let _ = app;
    }

    clear_harness_endpoint();
}

pub(crate) async fn healthy_on_port(port: u16) -> bool {
    let response = match reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/"))
        .timeout(Duration::from_secs(2))
        .send()
        .await
    {
        Ok(response) => response,
        Err(_) => return false,
    };

    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return response
            .text()
            .await
            .map(|body| body.contains("dsh web authentication required"))
            .unwrap_or(false);
    }

    status.is_success() || status.is_redirection()
}

pub(crate) async fn healthy(app: &AppHandle) -> bool {
    match harness_port(app) {
        Some(port) => healthy_on_port(port).await,
        None => false,
    }
}

pub(crate) fn harness_auth_cookie(app: &AppHandle) -> Option<String> {
    let port = harness_port(app)?;
    let authority = format!("127.0.0.1:{port}");
    let cred_path = dsh_home(app).ok()?.join(".credentials.yaml");
    if !cred_path.exists() {
        return None;
    }
    let script = r#"
const fs = require("fs");
const crypto = require("crypto");
const credPath = process.argv[1];
try {
  const content = fs.readFileSync(credPath, "utf8");
  const match = content.match(/client-connection\/browser-session:[\s\S]*?secret:\s*([^\s]+)/);
  if (!match) process.exit(1);
  function b64u(b) { return b.toString("base64").replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, ""); }
  function unb64u(s) { return Buffer.from(s.replaceAll("-", "+").replaceAll("_", "/") + "=".repeat((4 - s.length % 4) % 4), "base64"); }
  const authority = process.argv[2];
if (!authority) process.exit(1);
  const secret = unb64u(match[1]);
  const name = "dsh-auth-" + b64u(crypto.createHash("sha256").update(authority).digest());
  const now = Date.now();
  const payload = { version: 1, authority, issuedAt: now, expiresAt: now + 30 * 86400 * 1000 };
  const body = b64u(Buffer.from(JSON.stringify(payload)));
  const sig = b64u(crypto.createHmac("sha256", secret).update(body).digest());
  process.stdout.write(`${name}=v1.${body}.${sig}`);
} catch {
  process.exit(1);
}
"#;
    let node = node_bin(app);
    let mut command = Command::new(node);
    hidden(&mut command);
    command.arg("-e").arg(script).arg(cred_path).arg(authority);
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let cookie = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if cookie.is_empty() {
        None
    } else {
        Some(cookie)
    }
}

fn no_browser_patch_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(dsh_home(app)?.join("deepx-no-open.yml"))
}

pub(crate) fn write_no_browser_patch(app: &AppHandle) -> Result<PathBuf, String> {
    let path = no_browser_patch_path(app)?;
    fs::create_dir_all(path.parent().unwrap()).map_err(|error| error.to_string())?;
    fs::write(
        &path,
        r#"- id: web-runtime
  config:
    openBrowser: false
    printUrl: true
    surfaceContext: true
    trustedHosts: []"#,
    )
    .map_err(|error| error.to_string())?;
    Ok(path)
}

const OFFICIAL_BUNDLES: [&str; 3] = [
    "@deepseek-ai/dsh-base",
    "@deepseek-ai/dsh-web-app",
    "@deepseek-ai/dsh-headless",
];

pub(crate) fn migrate_private_plugins(app: &AppHandle) -> Result<bool, String> {
    let legacy_profile = legacy_dsh_home(app).join("profiles/web");
    let legacy_manifest_path = legacy_profile.join("package.json");
    if !legacy_manifest_path.is_file() {
        return Ok(false);
    }

    let shared_profile = profile_dir(app)?;
    let marker = app_data(app).join(".deepx-profile-migrated.json");
    if marker.is_file() {
        return Ok(false);
    }

    fs::create_dir_all(&shared_profile).map_err(|error| error.to_string())?;
    let legacy_manifest: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&legacy_manifest_path).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("旧插件配置无效: {error}"))?;

    let shared_manifest_path = shared_profile.join("package.json");
    let mut shared_manifest: serde_json::Value = if shared_manifest_path.is_file() {
        serde_json::from_str(
            &fs::read_to_string(&shared_manifest_path).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("Harness 插件配置无效: {error}"))?
    } else {
        serde_json::json!({
            "name": "dsh-profile-web",
            "private": true,
            "dependencies": {},
            "dsh": { "profile": { "bundles": [] } }
        })
    };
    if !shared_manifest.is_object() {
        return Err("Harness 插件配置必须是 JSON 对象".to_string());
    }

    let legacy_dependencies = legacy_manifest
        .pointer("/dependencies")
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();
    if shared_manifest.get("dependencies").is_none() {
        shared_manifest["dependencies"] = serde_json::json!({});
    }
    let shared_dependencies = shared_manifest
        .as_object_mut()
        .expect("shared manifest must be an object")
        .entry("dependencies")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or("Harness 插件配置中的 dependencies 无效")?;

    let missing = legacy_dependencies
        .iter()
        .filter(|(name, _)| !OFFICIAL_BUNDLES.contains(&name.as_str()))
        .filter(|(name, _)| !shared_dependencies.contains_key(*name))
        .map(|(name, spec)| (name.clone(), spec.clone()))
        .collect::<Vec<_>>();
    for (name, spec) in &missing {
        shared_dependencies.insert(name.clone(), spec.clone());
    }

    if let Some(legacy_bundles) = legacy_manifest
        .pointer("/dsh/profile/bundles")
        .and_then(|value| value.as_array())
    {
        let shared_root = shared_manifest
            .as_object_mut()
            .expect("shared manifest must be an object");
        let dsh = shared_root
            .entry("dsh")
            .or_insert_with(|| serde_json::json!({}));
        if !dsh.is_object() {
            return Err("Harness 插件配置中的 dsh 无效".to_string());
        }
        let profile = dsh
            .as_object_mut()
            .expect("checked dsh object")
            .entry("profile")
            .or_insert_with(|| serde_json::json!({}));
        if !profile.is_object() {
            return Err("Harness 插件配置中的 dsh.profile 无效".to_string());
        }
        let shared_bundles = profile
            .as_object_mut()
            .expect("checked profile object")
            .entry("bundles")
            .or_insert_with(|| serde_json::json!([]));
        if !shared_bundles.is_array() {
            return Err("Harness 插件配置中的 bundles 无效".to_string());
        }
        let shared_bundles = shared_bundles
            .as_array_mut()
            .expect("checked bundles array");
        for bundle in legacy_bundles {
            if !shared_bundles.contains(bundle) {
                shared_bundles.push(bundle.clone());
            }
        }
    }

    fs::write(
        &shared_manifest_path,
        serde_json::to_vec_pretty(&shared_manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;

    if !missing.is_empty() {
        let mut command = Command::new(node_bin(app));
        configure_runtime_environment(&mut command, app)?;
        command.arg(dsh_entry(app));
        command.args(["plugin", "--profile", "web", "add"]);
        for (name, spec) in &missing {
            command.arg(format!("{name}@{spec}"));
        }
        run_output(command).map_err(|error| format!("迁移私有插件失败: {error}"))?;
    }

    let marker_payload = serde_json::json!({ "migrated": true });
    let marker_file = fs::File::create(&marker).map_err(|error| error.to_string())?;
    serde_json::to_writer_pretty(marker_file, &marker_payload)
        .map_err(|error| error.to_string())?;
    Ok(true)
}

pub(crate) fn ensure_legacy_preset_compatibility(app: &AppHandle) -> Result<(), String> {
    let base = runtime_dir(app).join("node_modules/@deepseek-ai/dsh-agent-presets/presets");
    let standard = base.join("standard");
    let code = base.join("code");
    if standard.is_dir() && !code.exists() {
        let _ = copy_directory(&standard, &code);
    } else if code.is_dir() && !standard.exists() {
        let _ = copy_directory(&code, &standard);
    }
    if let Ok(home) = dsh_home(app) {
        let user_presets = home.join(".agent-presets");
        let user_standard = user_presets.join("standard");
        let user_code = user_presets.join("code");

        if user_standard.is_dir() && !user_code.exists() {
            let _ = copy_directory(&user_standard, &user_code);
        } else if user_code.is_dir() && !user_standard.exists() {
            let _ = copy_directory(&user_code, &user_standard);
        }

        if standard.is_dir() {
            if !user_code.exists() {
                let _ = copy_directory(&standard, &user_code);
            }
            if !user_standard.exists() {
                let _ = copy_directory(&standard, &user_standard);
            }
        } else if code.is_dir() {
            if !user_standard.exists() {
                let _ = copy_directory(&code, &user_standard);
            }
            if !user_code.exists() {
                let _ = copy_directory(&code, &user_code);
            }
        }
    }
    Ok(())
}

pub(crate) fn ensure_cross_harness_compatibility(app: &AppHandle) -> Result<usize, String> {
    let home = match app.path().home_dir() {
        Ok(home) => home,
        Err(_) => return Ok(0),
    };
    let codex_skills = home.join(".codex/skills");
    if !codex_skills.is_dir() {
        return Ok(0);
    }
    let agents_skills = home.join(".agents/skills");
    if let Err(e) = fs::create_dir_all(&agents_skills) {
        return Err(format!("创建技能目录失败: {e}"));
    }
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(&codex_skills) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            if file_name == ".system" {
                continue;
            }
            let source = entry.path();
            if !source.is_dir() {
                continue;
            }
            let target = agents_skills.join(&file_name);
            if target.symlink_metadata().is_err() {
                #[cfg(windows)]
                let success = if std::os::windows::fs::symlink_dir(&source, &target).is_ok() {
                    true
                } else {
                    copy_directory(&source, &target).is_ok()
                };
                #[cfg(not(windows))]
                let success = if std::os::unix::fs::symlink(&source, &target).is_ok() {
                    true
                } else {
                    copy_directory(&source, &target).is_ok()
                };
                if success {
                    count += 1;
                }
            }
        }
    }
    Ok(count)
}
