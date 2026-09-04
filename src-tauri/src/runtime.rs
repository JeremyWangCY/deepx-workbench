use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub percentage: u8,
    pub detail: String,
}

const RUNTIME_MARKER: &str = ".deepx-runtime-ready";

const REQUIRED_DSH_PEERS: [&str; 19] = [
    "@deepseek-ai/cordis-plugin-group",
    "@deepseek-ai/dsh-anonymous-user-id",
    "@deepseek-ai/dsh-atomic-write",
    "@deepseek-ai/dsh-authorization",
    "@deepseek-ai/dsh-bash-local",
    "@deepseek-ai/dsh-code-runtime",
    "@deepseek-ai/dsh-compaction",
    "@deepseek-ai/dsh-fs",
    "@deepseek-ai/dsh-invariants",
    "@deepseek-ai/dsh-output-retention",
    "@deepseek-ai/dsh-sandbox",
    "@deepseek-ai/dsh-scope",
    "@deepseek-ai/dsh-session-telemetry",
    "@deepseek-ai/dsh-session-title-llm",
    "@deepseek-ai/dsh-shell",
    "@deepseek-ai/dsh-spill",
    "@deepseek-ai/dsh-subagent-in-process-driver",
    "@deepseek-ai/dsh-timeout",
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

fn dsh_home(app: &AppHandle) -> Result<PathBuf, String> {
    let home = app.path().home_dir().map_err(|error| error.to_string())?;
    Ok(home.join(".dsh"))
}

fn legacy_dsh_home(app: &AppHandle) -> PathBuf {
    app_data(app).join("dsh")
}

pub(crate) fn profile_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(dsh_home(app)?.join("profiles/web"))
}

pub(crate) fn npm_bin(app: &AppHandle) -> PathBuf {
    node_dir(app).join("node_modules/npm/bin/npm-cli.js")
}

pub(crate) fn pnpm_package_manifest(app: &AppHandle) -> PathBuf {
    runtime_dir(app).join("node_modules/pnpm/package.json")
}

pub(crate) fn pnpm_cmd(app: &AppHandle) -> PathBuf {
    runtime_dir(app).join("bin/pnpm.cmd")
}

pub(crate) fn configure_runtime_environment(
    command: &mut Command,
    app: &AppHandle,
) -> Result<(), String> {
    let mut paths = vec![runtime_dir(app).join("bin"), node_dir(app)];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    command.env(
        "PATH",
        std::env::join_paths(paths).map_err(|error| error.to_string())?,
    );
    command.env("COREPACK_HOME", runtime_dir(app).join("corepack"));
    command.env("npm_config_node_linker", "hoisted");
    Ok(())
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

fn harness_version(app: &AppHandle) -> Result<String, String> {
    let manifest =
        fs::read_to_string(harness_package_manifest(app)).map_err(|error| error.to_string())?;
    serde_json::from_str::<serde_json::Value>(&manifest)
        .map_err(|error| error.to_string())?
        .get("version")
        .and_then(|value| value.as_str())
        .map(str::to_owned)
        .ok_or_else(|| "Harness 版本信息无效".to_string())
}

fn aligned_peer_packages(version: &str) -> Vec<String> {
    REQUIRED_DSH_PEERS
        .iter()
        .map(|package| {
            if *package == "@deepseek-ai/cordis-plugin-group" {
                (*package).to_string()
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

pub(crate) async fn install_runtime(app: AppHandle) -> Result<(), String> {
    if valid_runtime(&app) {
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

pub(crate) async fn update_runtime(app: AppHandle) -> Result<(), String> {
    if !valid_runtime(&app) {
        install_runtime(app.clone()).await?;
    }

    emit_progress(&app, 52, "正在更新 Harness...");
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
    let mut command = Command::new(node_bin(&app));
    command
        .arg(npm_bin(&app))
        .args(install_options)
        .arg(runtime_dir(&app))
        .arg("@deepseek-ai/dsh@latest")
        .current_dir(runtime_dir(&app));
    run_output_with_timeout(command, Duration::from_secs(300))
        .map_err(|error| format!("Harness 更新失败: {error}"))?;

    let version = harness_version(&app)?;
    let mut peer_command = Command::new(node_bin(&app));
    peer_command
        .arg(npm_bin(&app))
        .args(install_options)
        .arg(runtime_dir(&app))
        .args(aligned_peer_packages(&version))
        .current_dir(runtime_dir(&app));
    run_output_with_timeout(peer_command, Duration::from_secs(300))
        .map_err(|error| format!("Harness 依赖更新失败: {error}"))?;
    fs::write(runtime_marker(&app), version).map_err(|error| error.to_string())?;
    emit_progress(&app, 90, "Harness 已更新");
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
        .map(|response| {
            let status = response.status();
            status.is_success()
                || status.is_redirection()
                || status == reqwest::StatusCode::UNAUTHORIZED
        })
        .unwrap_or(false)
}

pub(crate) fn harness_auth_cookie(app: &AppHandle) -> Option<String> {
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
  const authority = "127.0.0.1:3080";
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
    command.arg("-e").arg(script).arg(cred_path);
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
