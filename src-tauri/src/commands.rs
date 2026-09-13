use crate::{
    clear_harness_endpoint, configure_runtime_environment, dsh_entry, dsh_home, emit_progress,
    ensure_cross_harness_compatibility, ensure_legacy_preset_compatibility,
    ensure_profile_store_compatibility, harness_auth_cookie, harness_base_url, harness_child_port,
    harness_package_manifest, harness_port, harness_process_running, healthy, healthy_on_port,
    hidden, install_runtime, marketplace_installed, marketplace_version, migrate_private_plugins,
    node_bin, profile_dir, remember_harness_launch_url, repair_marketplace_metadata,
    rollback_runtime, run_output_with_timeout, runtime_dir, seed_bundled_marketplace,
    stop_harness_service, take_harness_boot_url, update_runtime, valid_runtime,
    write_no_browser_patch,
};
use serde::Serialize;
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, Url, WebviewWindow};
use tauri_plugin_opener::OpenerExt;
use tokio::io::AsyncWriteExt;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum HarnessLifecycleState {
    Stopped = 0,
    Starting = 1,
    Healthy = 2,
    Recovering = 3,
    Updating = 4,
    RollingBack = 5,
    Failed = 6,
}

impl HarnessLifecycleState {
    fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Starting,
            2 => Self::Healthy,
            3 => Self::Recovering,
            4 => Self::Updating,
            5 => Self::RollingBack,
            6 => Self::Failed,
            _ => Self::Stopped,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Stopped => "stopped",
            Self::Starting => "starting",
            Self::Healthy => "healthy",
            Self::Recovering => "recovering",
            Self::Updating => "updating",
            Self::RollingBack => "rolling_back",
            Self::Failed => "failed",
        }
    }
}

static HARNESS_MAINTENANCE_DEPTH: AtomicUsize = AtomicUsize::new(0);
static HARNESS_RECOVERY_ACTIVE: AtomicBool = AtomicBool::new(false);
static HARNESS_LAUNCH_ACTIVE: AtomicBool = AtomicBool::new(false);
static HARNESS_ROUTE_RESTORE_PENDING: AtomicBool = AtomicBool::new(false);
static HARNESS_LIFECYCLE_STATE: AtomicU8 = AtomicU8::new(HarnessLifecycleState::Stopped as u8);

struct MaintenanceGuard;

impl Drop for MaintenanceGuard {
    fn drop(&mut self) {
        HARNESS_MAINTENANCE_DEPTH.fetch_sub(1, Ordering::SeqCst);
    }
}

fn maintenance_guard() -> MaintenanceGuard {
    HARNESS_MAINTENANCE_DEPTH.fetch_add(1, Ordering::SeqCst);
    MaintenanceGuard
}

struct AtomicFlagGuard(&'static AtomicBool);

impl Drop for AtomicFlagGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn try_acquire_flag(flag: &'static AtomicBool) -> Option<AtomicFlagGuard> {
    flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .ok()
        .map(|_| AtomicFlagGuard(flag))
}

fn current_harness_state() -> HarnessLifecycleState {
    HarnessLifecycleState::from_u8(HARNESS_LIFECYCLE_STATE.load(Ordering::SeqCst))
}

fn set_harness_state(app: &AppHandle, state: HarnessLifecycleState) {
    let previous =
        HarnessLifecycleState::from_u8(HARNESS_LIFECYCLE_STATE.swap(state as u8, Ordering::SeqCst));
    if previous != state {
        append_supervisor_log(
            app,
            format!("STATE {} -> {}", previous.as_str(), state.as_str()),
        );
        let _ = app.emit("harness-lifecycle", state.as_str());
    }
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn append_supervisor_log(app: &AppHandle, detail: impl AsRef<str>) {
    const MAX_SUPERVISOR_LOG_BYTES: u64 = 2 * 1024 * 1024;
    let path = runtime_dir(app).join("harness-supervisor.log");
    if fs::metadata(&path)
        .map(|metadata| metadata.len() >= MAX_SUPERVISOR_LOG_BYTES)
        .unwrap_or(false)
    {
        let previous = runtime_dir(app).join("harness-supervisor.previous.log");
        let _ = fs::remove_file(&previous);
        let _ = fs::rename(&path, previous);
    }
    if let Ok(mut log) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(log, "{} {}", unix_millis(), detail.as_ref());
    }
}

fn rotate_harness_log(log_path: &Path) {
    const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;
    let should_rotate = fs::metadata(log_path)
        .map(|metadata| metadata.len() >= MAX_LOG_BYTES)
        .unwrap_or(false);
    if !should_rotate {
        return;
    }
    let previous = log_path.with_file_name("harness-startup.previous.log");
    let _ = fs::remove_file(&previous);
    let _ = fs::rename(log_path, previous);
}

fn redact_harness_line(line: &str) -> String {
    let mut redacted = line.to_string();
    let needle = "token=";
    let mut search_from = 0;
    loop {
        let lowercase = redacted.to_ascii_lowercase();
        let Some(relative) = lowercase[search_from..].find(needle) else {
            break;
        };
        let value_start = search_from + relative + needle.len();
        let value_end = redacted[value_start..]
            .find(|ch: char| ch.is_whitespace() || matches!(ch, '&' | '"' | '\'' | ')' | ']' | '}'))
            .map(|offset| value_start + offset)
            .unwrap_or(redacted.len());
        if value_end <= value_start {
            break;
        }
        redacted.replace_range(value_start..value_end, "<redacted>");
        search_from = value_start + "<redacted>".len();
    }
    redacted
}

fn sanitize_harness_log_file(path: &Path) {
    let Ok(original) = fs::read_to_string(path) else {
        return;
    };
    let sanitized = original
        .lines()
        .map(redact_harness_line)
        .collect::<Vec<_>>()
        .join("\n");
    if sanitized != original.trim_end_matches(['\r', '\n']) {
        let suffix = if original.ends_with('\n') { "\n" } else { "" };
        let _ = fs::write(path, format!("{sanitized}{suffix}"));
    }
}

fn capture_harness_launch_url(line: &str) {
    let Some(start) = line.find("http://127.0.0.1:") else {
        return;
    };
    let candidate = &line[start..];
    let end = candidate
        .find(|ch: char| ch.is_whitespace() || matches!(ch, '"' | '\'' | ')' | ']' | '}'))
        .unwrap_or(candidate.len());
    let url = &candidate[..end];
    let _ = remember_harness_launch_url(url);
}

fn pipe_harness_output<R>(app: AppHandle, reader: R, log_path: PathBuf, stream_name: &'static str)
where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || {
        let mut log = match fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            Ok(log) => log,
            Err(error) => {
                append_supervisor_log(
                    &app,
                    format!("LOG_PIPE_OPEN_ERROR stream={stream_name} error={error}"),
                );
                return;
            }
        };
        for line in BufReader::new(reader).lines() {
            match line {
                Ok(line) => {
                    capture_harness_launch_url(&line);
                    let _ = writeln!(log, "{}", redact_harness_line(&line));
                    let _ = log.flush();
                }
                Err(error) => {
                    append_supervisor_log(
                        &app,
                        format!("LOG_PIPE_READ_ERROR stream={stream_name} error={error}"),
                    );
                    break;
                }
            }
        }
    });
}

fn watch_harness_exit(app: AppHandle, mut child: Child) {
    let pid = child.id();
    std::thread::spawn(move || {
        let status = child.wait();
        match status {
            Ok(status) => {
                append_supervisor_log(&app, format!("EXIT pid={pid} code={:?}", status.code()))
            }
            Err(error) => {
                append_supervisor_log(&app, format!("EXIT_WAIT_ERROR pid={pid} error={error}"))
            }
        }
        clear_harness_endpoint();
        if HARNESS_MAINTENANCE_DEPTH.load(Ordering::SeqCst) == 0 {
            set_harness_state(&app, HarnessLifecycleState::Stopped);
            tauri::async_runtime::spawn(async move {
                recover_harness(app).await;
            });
        }
    });
}

async fn recover_harness(app: AppHandle) {
    if HARNESS_MAINTENANCE_DEPTH.load(Ordering::SeqCst) > 0 || !valid_runtime(&app) {
        return;
    }
    let Some(_recovery_guard) = try_acquire_flag(&HARNESS_RECOVERY_ACTIVE) else {
        return;
    };
    set_harness_state(&app, HarnessLifecycleState::Recovering);

    const BACKOFF_SECONDS: [u64; 5] = [1, 2, 5, 10, 30];
    for (index, delay) in BACKOFF_SECONDS.into_iter().enumerate() {
        if healthy(&app).await {
            set_harness_state(&app, HarnessLifecycleState::Healthy);
            return;
        }
        if HARNESS_MAINTENANCE_DEPTH.load(Ordering::SeqCst) > 0 {
            append_supervisor_log(&app, "RECOVERY_ABORT maintenance");
            return;
        }

        append_supervisor_log(
            &app,
            format!("RECOVERY_WAIT attempt={} delay={}s", index + 1, delay),
        );
        tokio::time::sleep(Duration::from_secs(delay)).await;
        if HARNESS_MAINTENANCE_DEPTH.load(Ordering::SeqCst) > 0 {
            append_supervisor_log(&app, "RECOVERY_ABORT maintenance");
            return;
        }

        match launch_harness(app.clone()).await {
            Ok(()) => {
                if healthy(&app).await {
                    append_supervisor_log(&app, format!("RECOVERY_OK attempt={}", index + 1));
                    set_harness_state(&app, HarnessLifecycleState::Healthy);
                    let _ = show_harness(app.clone()).await;
                    return;
                }
            }
            Err(error) => append_supervisor_log(
                &app,
                format!("RECOVERY_FAIL attempt={} error={error}", index + 1),
            ),
        }
    }

    append_supervisor_log(&app, "RECOVERY_GAVE_UP attempts=5");
    set_harness_state(&app, HarnessLifecycleState::Failed);
}

pub(crate) fn start_harness_watchdog(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(12)).await;
        let mut consecutive_failures = 0_u8;
        loop {
            if HARNESS_MAINTENANCE_DEPTH.load(Ordering::SeqCst) > 0
                || !valid_runtime(&app)
                || healthy(&app).await
            {
                consecutive_failures = 0;
            } else {
                consecutive_failures = consecutive_failures.saturating_add(1);
                if consecutive_failures >= 2 {
                    append_supervisor_log(&app, "WATCHDOG_UNHEALTHY consecutive=2");
                    recover_harness(app.clone()).await;
                    consecutive_failures = 0;
                }
            }
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    });
}

#[derive(Debug, Serialize)]
pub struct RuntimeStatus {
    pub ready: bool,
    pub service_running: bool,
    pub endpoint: Option<String>,
    pub port: Option<u16>,
    pub auth_cookie: Option<String>,
    pub lifecycle_state: String,
    pub deepx_version: String,
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

fn harness_route_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("harness-route.txt"))
}

fn sanitize_harness_route(route: &str) -> Option<String> {
    if route.is_empty()
        || route.len() > 4096
        || !route.starts_with('/')
        || route.chars().any(char::is_control)
    {
        return None;
    }

    let mut parsed = Url::parse(&format!("http://127.0.0.1{route}")).ok()?;
    if parsed.host_str() != Some("127.0.0.1") {
        return None;
    }

    let query_pairs = parsed
        .query_pairs()
        .filter(|(key, _)| !key.eq_ignore_ascii_case("token"))
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    parsed.set_query(None);
    if !query_pairs.is_empty() {
        let mut query = parsed.query_pairs_mut();
        for (key, value) in query_pairs {
            query.append_pair(&key, &value);
        }
    }

    let mut sanitized = parsed.path().to_string();
    if let Some(query) = parsed.query() {
        sanitized.push('?');
        sanitized.push_str(query);
    }
    if let Some(fragment) = parsed.fragment() {
        sanitized.push('#');
        sanitized.push_str(fragment);
    }
    Some(sanitized)
}

fn remembered_harness_route(app: &AppHandle) -> Option<String> {
    let route = fs::read_to_string(harness_route_path(app).ok()?).ok()?;
    sanitize_harness_route(route.trim())
}

fn caller_matches_harness(webview: &WebviewWindow, port: u16) -> bool {
    webview.url().ok().is_some_and(|url| {
        url.scheme() == "http" && url.host_str() == Some("127.0.0.1") && url.port() == Some(port)
    })
}

#[tauri::command]
pub fn remember_harness_route(
    app: AppHandle,
    webview: WebviewWindow,
    route: String,
) -> Result<(), String> {
    let Some(port) = harness_port(&app) else {
        return Ok(());
    };
    if !caller_matches_harness(&webview, port) {
        return Ok(());
    }
    let Some(route) = sanitize_harness_route(&route) else {
        return Ok(());
    };
    let path = harness_route_path(&app)?;
    if fs::read_to_string(&path).ok().as_deref() == Some(route.as_str()) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(path, route).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn take_harness_restore_route(app: AppHandle, webview: WebviewWindow) -> Option<String> {
    let port = harness_port(&app)?;
    if !caller_matches_harness(&webview, port) {
        return None;
    }
    if !HARNESS_ROUTE_RESTORE_PENDING.swap(false, Ordering::SeqCst) {
        return None;
    }
    remembered_harness_route(&app).filter(|route| route != "/")
}

fn sanitize_diagnostic_text(app: &AppHandle, text: &str) -> String {
    let mut lines = text
        .lines()
        .rev()
        .take(10_000)
        .map(redact_harness_line)
        .collect::<Vec<_>>();
    lines.reverse();
    let mut sanitized = lines.join("\n");
    if let Ok(home) = app.path().home_dir() {
        let native = home.to_string_lossy().to_string();
        let forward = native.replace('\\', "/");
        if !native.is_empty() {
            sanitized = sanitized.replace(&native, "<user-home>");
        }
        if forward != native && !forward.is_empty() {
            sanitized = sanitized.replace(&forward, "<user-home>");
        }
    }
    sanitized
}

fn export_diagnostic_bundle(app: &AppHandle) -> Result<PathBuf, String> {
    let downloads = app
        .path()
        .download_dir()
        .or_else(|_| app.path().app_cache_dir())
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&downloads).map_err(|error| error.to_string())?;
    let bundle = downloads.join(format!(
        "DeepX-diagnostics-v{}-{}.zip",
        app.package_info().version,
        unix_millis() / 1000
    ));
    let file = fs::File::create(&bundle).map_err(|error| error.to_string())?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    let endpoint = harness_port(app).map(|port| format!("127.0.0.1:{port}"));
    let summary = serde_json::json!({
        "deepx_version": app.package_info().version.to_string(),
        "harness_version": package_version(harness_package_manifest(app)),
        "lifecycle_state": current_harness_state().as_str(),
        "endpoint": endpoint,
        "runtime_ready": valid_runtime(app),
        "platform": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "generated_at_unix_ms": unix_millis(),
    });
    zip.start_file("summary.json", options)
        .map_err(|error| error.to_string())?;
    zip.write_all(
        serde_json::to_string_pretty(&summary)
            .map_err(|error| error.to_string())?
            .as_bytes(),
    )
    .map_err(|error| error.to_string())?;

    let runtime = runtime_dir(app);
    let sources = [
        (
            "harness-supervisor.log",
            runtime.join("harness-supervisor.log"),
        ),
        (
            "harness-supervisor.previous.log",
            runtime.join("harness-supervisor.previous.log"),
        ),
        ("harness-startup.log", runtime.join("harness-startup.log")),
        (
            "harness-startup.previous.log",
            runtime.join("harness-startup.previous.log"),
        ),
    ];

    for (name, path) in sources {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        zip.start_file(name, options)
            .map_err(|error| error.to_string())?;
        zip.write_all(sanitize_diagnostic_text(app, &text).as_bytes())
            .map_err(|error| error.to_string())?;
    }
    zip.finish().map_err(|error| error.to_string())?;
    Ok(bundle)
}

#[tauri::command]
pub fn window_action(app: AppHandle, action: String) -> Result<Option<usize>, String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "主窗口不存在".to_string())?;
    match action.as_str() {
        "minimize" => {
            window.minimize().map_err(|error| error.to_string())?;
            Ok(None)
        }
        "toggle_maximize" => {
            let maximized = window.is_maximized().map_err(|error| error.to_string())?;
            if maximized {
                window.unmaximize().map_err(|error| error.to_string())?;
            } else {
                window.maximize().map_err(|error| error.to_string())?;
            }
            Ok(None)
        }
        "close" => {
            window.close().map_err(|error| error.to_string())?;
            Ok(None)
        }
        "start_dragging" => {
            window.start_dragging().map_err(|error| error.to_string())?;
            Ok(None)
        }
        "open_devtools" | "toggle_devtools" => {
            if window.is_devtools_open() {
                window.close_devtools();
            } else {
                window.open_devtools();
            }
            Ok(None)
        }
        "open_dsh_home" => {
            let path = dsh_home(&app)?;
            let _ = fs::create_dir_all(&path);
            open_path_in_explorer(&app, &path);
            Ok(None)
        }
        "open_plugins_dir" => {
            let path = profile_dir(&app)?;
            let _ = fs::create_dir_all(&path);
            open_path_in_explorer(&app, &path);
            Ok(None)
        }
        "open_skills_dir" => {
            let home = app.path().home_dir().map_err(|error| error.to_string())?;
            let path = home.join(".agents/skills");
            let _ = fs::create_dir_all(&path);
            open_path_in_explorer(&app, &path);
            Ok(None)
        }
        "open_log_file" => {
            let path = runtime_dir(&app).join("harness-startup.log");
            if !path.exists() {
                let _ = fs::write(&path, "");
            }
            open_path_in_explorer(&app, &path);
            Ok(None)
        }
        "repair_plugins" => {
            repair_marketplace_metadata(&app)?;
            Ok(None)
        }
        "migrate_codex_skills" => {
            let count = ensure_cross_harness_compatibility(&app)?;
            Ok(Some(count))
        }
        "export_diagnostics" => {
            let path = export_diagnostic_bundle(&app)?;
            if let Some(parent) = path.parent() {
                open_path_in_explorer(&app, parent);
            }
            Ok(None)
        }
        _ => Err(format!("不支持的窗口操作: {action}")),
    }
}

fn open_path_in_explorer(app: &AppHandle, path: &std::path::Path) {
    if app
        .opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .is_err()
    {
        #[cfg(windows)]
        {
            let _ = Command::new("explorer").arg(path).spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("open").arg(path).spawn();
        }
        #[cfg(all(not(windows), not(target_os = "macos")))]
        {
            let _ = Command::new("xdg-open").arg(path).spawn();
        }
    }
}

#[tauri::command]
pub async fn runtime_status(app: AppHandle, webview: WebviewWindow) -> RuntimeStatus {
    let port = harness_port(&app);
    let service_running = match port {
        Some(port) => healthy_on_port(port).await,
        None => false,
    };
    let auth_cookie = port
        .filter(|port| caller_matches_harness(&webview, *port))
        .and_then(|_| harness_auth_cookie(&app));
    let mut lifecycle = current_harness_state();
    if service_running
        && matches!(
            lifecycle,
            HarnessLifecycleState::Stopped | HarnessLifecycleState::Failed
        )
    {
        set_harness_state(&app, HarnessLifecycleState::Healthy);
        lifecycle = HarnessLifecycleState::Healthy;
    }
    RuntimeStatus {
        ready: valid_runtime(&app),
        service_running,
        endpoint: port.map(|port| format!("127.0.0.1:{port}")),
        port,
        auth_cookie,
        lifecycle_state: lifecycle.as_str().to_string(),
        deepx_version: app.package_info().version.to_string(),
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

async fn fetch_npm_version(
    client: &reqwest::Client,
    url: &str,
    timeout_secs: u64,
) -> Option<String> {
    let response = client
        .get(url)
        .timeout(Duration::from_secs(timeout_secs))
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    response
        .json::<serde_json::Value>()
        .await
        .ok()?
        .get("version")
        .and_then(|value| value.as_str())
        .map(str::to_owned)
}

async fn npm_latest_release(client: &reqwest::Client, package: &str) -> Option<String> {
    let encoded = package.replace('/', "%2f");
    let primary_url = format!("https://registry.npmjs.org/{encoded}/latest");
    if let Some(version) = fetch_npm_version(client, &primary_url, 6).await {
        return Some(version);
    }
    let fallback_url = format!("https://registry.npmmirror.com/{encoded}/latest");
    fetch_npm_version(client, &fallback_url, 10).await
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

async fn wait_for_harness(child: &mut Child, log_path: &Path) -> Result<(), String> {
    for _ in 0..180 {
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
        if let Some(port) = harness_child_port(child.id()) {
            if healthy_on_port(port).await {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let _ = child.kill();
    Err("Harness 启动超时".to_string())
}

async fn stop_current_harness(app: &AppHandle) -> Result<(), String> {
    stop_harness_service(app);
    for _ in 0..20 {
        if !harness_process_running(app) {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    Err("旧的 Harness 服务停止超时".to_string())
}

async fn wait_for_concurrent_launch(app: &AppHandle) -> Result<(), String> {
    for _ in 0..60 {
        if healthy(app).await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Err("Harness 正在由另一个任务启动，但等待超时".to_string())
}

#[tauri::command]
pub async fn launch_harness(app: AppHandle) -> Result<(), String> {
    let Some(_launch_guard) = try_acquire_flag(&HARNESS_LAUNCH_ACTIVE) else {
        return wait_for_concurrent_launch(&app).await;
    };

    let migrated = migrate_private_plugins(&app)?;
    let _migration_guard = migrated.then(maintenance_guard);
    let _ = ensure_legacy_preset_compatibility(&app);
    let _ = ensure_profile_store_compatibility(&app);

    let service_healthy = healthy(&app).await;
    if service_healthy {
        if !migrated {
            set_harness_state(&app, HarnessLifecycleState::Healthy);
            return Ok(());
        }
        stop_current_harness(&app).await?;
    } else if harness_process_running(&app) {
        append_supervisor_log(&app, "OWNED_PROCESS_UNHEALTHY stopping before restart");
        stop_current_harness(&app).await?;
    }
    if migrated {
        repair_marketplace_metadata(&app)?;
    }
    let no_browser_patch = write_no_browser_patch(&app)?;
    if !valid_runtime(&app) {
        set_harness_state(&app, HarnessLifecycleState::Failed);
        return Err("Harness 尚未安装".to_string());
    }
    set_harness_state(&app, HarnessLifecycleState::Starting);

    let mut command = Command::new(node_bin(&app));
    command.arg(dsh_entry(&app)).args([
        "--profile",
        "web",
        "--patch",
        &no_browser_patch.to_string_lossy(),
        "--no-open",
        "--port",
        "0",
    ]);
    let log_path = runtime_dir(&app).join("harness-startup.log");
    let previous_log_path = runtime_dir(&app).join("harness-startup.previous.log");
    sanitize_harness_log_file(&log_path);
    sanitize_harness_log_file(&previous_log_path);
    rotate_harness_log(&log_path);
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|error| error.to_string())?;
    let _ = writeln!(
        log,
        "\n=== DeepX Harness start ts={} deepx={} ===",
        unix_millis(),
        app.package_info().version
    );
    let _ = log.flush();
    drop(log);
    hidden(&mut command);
    configure_runtime_environment(&mut command, &app)?;
    let mut child = match command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            set_harness_state(&app, HarnessLifecycleState::Failed);
            return Err(error.to_string());
        }
    };

    let pid = child.id();
    if let Some(stdout) = child.stdout.take() {
        pipe_harness_output(app.clone(), stdout, log_path.clone(), "stdout");
    }
    if let Some(stderr) = child.stderr.take() {
        pipe_harness_output(app.clone(), stderr, log_path.clone(), "stderr");
    }
    append_supervisor_log(&app, format!("SPAWN pid={pid} port=dynamic"));
    match wait_for_harness(&mut child, &log_path).await {
        Ok(()) => {
            let port = harness_child_port(pid).unwrap_or_default();
            append_supervisor_log(&app, format!("HEALTHY pid={pid} port={port}"));
            set_harness_state(&app, HarnessLifecycleState::Healthy);
            watch_harness_exit(app.clone(), child);
            Ok(())
        }
        Err(error) => {
            append_supervisor_log(&app, format!("START_FAILED pid={pid} error={error}"));
            set_harness_state(&app, HarnessLifecycleState::Failed);
            Err(error)
        }
    }
}

fn get_harness_url(app: &AppHandle) -> Result<String, String> {
    let port = harness_port(app).ok_or_else(|| "Harness 尚未监听本地端口".to_string())?;
    if let Some(url) = take_harness_boot_url() {
        if Url::parse(&url).ok().and_then(|parsed| parsed.port()) == Some(port) {
            return Ok(url);
        }
    }
    harness_base_url(app).ok_or_else(|| "Harness endpoint 不可用".to_string())
}

async fn navigate_to_harness(app: AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("主窗口不存在")?;
    let target = get_harness_url(&app)?;
    HARNESS_ROUTE_RESTORE_PENDING.store(true, Ordering::SeqCst);
    window
        .navigate(Url::parse(&target).map_err(|error| error.to_string())?)
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

/// Diagnostic probe: the toolbar script reports its page state on every
/// watchdog tick. Writes to the onload log only while the probe flag file
/// exists, so shipping builds stay silent.
#[tauri::command]
pub fn toolbar_probe(app: AppHandle, diag: String) {
    let Ok(app_data) = app.path().app_data_dir() else {
        return;
    };
    let flag = app_data.join(".deepx-probe.flag");
    if !flag.is_file() {
        return;
    }
    let Ok(log_dir) = app.path().app_log_dir() else {
        return;
    };
    let _ = fs::create_dir_all(&log_dir);
    if let Ok(mut log) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_dir.join("webview-onload.log"))
    {
        let _ = writeln!(log, "PROBE {}", diag);
    }
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
        let installer = cache.join(format!("deepx-workbench-update-{tag}.exe"));
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
            drop(file);
            let _ = fs::remove_file(&installer);
            return Err(format!("保存 DeepX 更新失败: {error}"));
        }
        if let Err(error) = file.sync_all().await {
            drop(file);
            let _ = fs::remove_file(&installer);
            return Err(format!("保存 DeepX 更新失败: {error}"));
        }
        drop(file);
        tokio::time::sleep(Duration::from_millis(150)).await;

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
        let mut spawn_result = Command::new(&installer)
            // /S silent install (skips the remove-previous dialog that hangs
            // GUI mode when previous installs are half-broken), /R relaunches
            // the freshly installed app when the install finishes.
            .args(["/S", "/R"])
            .spawn();

        if let Err(ref e) = spawn_result {
            if e.raw_os_error() == Some(32) {
                for _ in 0..5 {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    spawn_result = Command::new(&installer).args(["/S", "/R"]).spawn();
                    if spawn_result.is_ok() {
                        break;
                    }
                }
            }
        }
        spawn_result.map_err(|error| format!("启动 DeepX 更新安装器失败: {error}"))?;
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
    let _maintenance = maintenance_guard();
    install_runtime(app.clone()).await?;
    if !marketplace_installed(&app) {
        emit_progress(&app, 92, "正在准备插件市场...");
        install_marketplace(app.clone()).await?;
    }
    emit_progress(&app, 94, "正在启动...");
    if !healthy(&app).await {
        launch_harness(app.clone()).await?;
    }
    show_harness(app).await
}

#[tauri::command]
pub async fn update_harness(app: AppHandle) -> Result<(), String> {
    let _maintenance = maintenance_guard();
    set_harness_state(&app, HarnessLifecycleState::Updating);
    emit_progress(&app, 25, "正在更新...");

    if let Err(error) = stop_current_harness(&app).await {
        set_harness_state(&app, HarnessLifecycleState::Failed);
        return Err(error);
    }
    if let Err(error) = update_runtime(app.clone()).await {
        set_harness_state(&app, HarnessLifecycleState::Failed);
        return Err(error);
    }
    emit_progress(&app, 94, "正在验证新版 Harness 启动...");

    match launch_harness(app.clone()).await {
        Ok(()) => {
            set_harness_state(&app, HarnessLifecycleState::Healthy);
            show_harness(app).await
        }
        Err(update_error) => {
            append_supervisor_log(
                &app,
                format!("UPDATE_START_FAILED error={update_error}; rolling back"),
            );
            set_harness_state(&app, HarnessLifecycleState::RollingBack);
            emit_progress(&app, 72, "新版 Harness 启动失败，正在自动回滚...");
            match rollback_runtime(&app) {
                Ok(true) => match launch_harness(app.clone()).await {
                    Ok(()) => {
                        let _ = show_harness(app.clone()).await;
                        append_supervisor_log(&app, "UPDATE_ROLLBACK_OK");
                        set_harness_state(&app, HarnessLifecycleState::Healthy);
                        Err(format!(
                            "新版 Harness 未通过启动验证，DeepX 已自动恢复上一版。原始错误：{update_error}"
                        ))
                    }
                    Err(rollback_launch_error) => {
                        set_harness_state(&app, HarnessLifecycleState::Failed);
                        Err(format!(
                            "新版 Harness 启动失败，已恢复上一版文件，但旧版重新启动也失败。新版错误：{update_error}；旧版错误：{rollback_launch_error}"
                        ))
                    }
                },
                Ok(false) => {
                    set_harness_state(&app, HarnessLifecycleState::Failed);
                    Err(format!(
                        "新版 Harness 启动失败，且没有可用的上一版回滚副本：{update_error}"
                    ))
                }
                Err(rollback_error) => {
                    set_harness_state(&app, HarnessLifecycleState::Failed);
                    Err(format!(
                        "新版 Harness 启动失败且自动回滚失败。新版错误：{update_error}；回滚错误：{rollback_error}"
                    ))
                }
            }
        }
    }
}

#[tauri::command]
pub async fn restart_harness(app: AppHandle) -> Result<(), String> {
    let _maintenance = maintenance_guard();
    set_harness_state(&app, HarnessLifecycleState::Starting);
    emit_progress(&app, 15, "正在重启 Harness...");
    if let Err(error) = stop_current_harness(&app).await {
        set_harness_state(&app, HarnessLifecycleState::Failed);
        return Err(error);
    }
    emit_progress(&app, 70, "正在启动...");
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
    let _maintenance = maintenance_guard();
    emit_progress(&app, 15, "正在准备插件市场...");
    if !valid_runtime(&app) {
        install_runtime(app.clone()).await?;
    }
    let migrated = migrate_private_plugins(&app)?;
    if migrated && healthy(&app).await {
        emit_progress(&app, 35, "正在切换到共享插件目录...");
        stop_current_harness(&app).await?;
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
    if !healthy(&app).await {
        launch_harness(app.clone()).await?;
        show_harness(app.clone()).await?;
    }
    emit_progress(&app, 100, "插件市场已就绪");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{redact_harness_line, sanitize_harness_route};

    #[test]
    fn harness_logs_redact_token_values() {
        let line = "dsh web: http://127.0.0.1:51650/?token=secret-value&mode=web";
        let redacted = redact_harness_line(line);
        assert_eq!(
            redacted,
            "dsh web: http://127.0.0.1:51650/?token=<redacted>&mode=web"
        );
        assert!(!redacted.contains("secret-value"));

        let mixed_case = "dsh web: http://127.0.0.1:51650/?Token=another-secret&mode=web";
        let redacted = redact_harness_line(mixed_case);
        assert_eq!(
            redacted,
            "dsh web: http://127.0.0.1:51650/?Token=<redacted>&mode=web"
        );
        assert!(!redacted.contains("another-secret"));
    }

    #[test]
    fn remembered_routes_never_persist_auth_tokens() {
        assert_eq!(
            sanitize_harness_route("/sessions/abc?view=trace&token=secret#step"),
            Some("/sessions/abc?view=trace#step".to_string())
        );
        assert_eq!(
            sanitize_harness_route("/sessions/abc?Token=secret&view=trace"),
            Some("/sessions/abc?view=trace".to_string())
        );
        assert_eq!(sanitize_harness_route("https://example.com/"), None);
    }
}
