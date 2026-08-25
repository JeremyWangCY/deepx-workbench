use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum UpdateChannel {
    Latest,
    Next,
}

impl UpdateChannel {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Latest => "latest",
            Self::Next => "next",
        }
    }
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

fn update_channel_path(app: &AppHandle) -> PathBuf {
    app_data(app).join("update-channel.json")
}

pub(crate) fn update_channel(app: &AppHandle) -> UpdateChannel {
    fs::read_to_string(update_channel_path(app))
        .ok()
        .and_then(|text| serde_json::from_str::<UpdateChannel>(&text).ok())
        .unwrap_or(UpdateChannel::Latest)
}

pub(crate) fn set_update_channel(app: &AppHandle, channel: UpdateChannel) -> Result<(), String> {
    let path = update_channel_path(app);
    fs::create_dir_all(path.parent().unwrap()).map_err(|error| error.to_string())?;
    let payload = serde_json::to_vec(&channel).map_err(|error| error.to_string())?;
    fs::write(path, payload).map_err(|error| error.to_string())
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
    let profile = match profile_dir(app) {
        Ok(path) => path,
        Err(_) => return false,
    };
    let manifest = profile.join("package.json");
    let has_dependency = fs::read_to_string(manifest)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| value.get("dependencies")?.get("dshmarket").cloned())
        .is_some();
    has_dependency
        && profile
            .join("node_modules/dshmarket/package.json")
            .is_file()
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
        .arg(format!(
            "@deepseek-ai/dsh@{}",
            update_channel(&app).as_str()
        ))
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
