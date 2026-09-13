use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    sync::{
        mpsc::{sync_channel, RecvTimeoutError, SyncSender},
        Mutex, OnceLock,
    },
    time::Duration,
};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WindowEvent};

const MIN_WIDTH: u32 = 860;
const MIN_HEIGHT: u32 = 600;
const SAVE_DEBOUNCE: Duration = Duration::from_millis(450);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
struct SavedWindowState {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    maximized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WorkArea {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

static WINDOW_STATE: OnceLock<Mutex<Option<SavedWindowState>>> = OnceLock::new();
static SAVE_SIGNAL: OnceLock<SyncSender<()>> = OnceLock::new();

fn state_cache() -> &'static Mutex<Option<SavedWindowState>> {
    WINDOW_STATE.get_or_init(|| Mutex::new(None))
}

fn state_path(app: &AppHandle) -> Option<PathBuf> {
    Some(app.path().app_data_dir().ok()?.join("window-state.json"))
}

fn work_areas(window: &WebviewWindow) -> Vec<WorkArea> {
    let mut result = Vec::new();
    if let Ok(Some(primary)) = window.primary_monitor() {
        let area = primary.work_area();
        result.push(WorkArea {
            x: area.position.x,
            y: area.position.y,
            width: area.size.width,
            height: area.size.height,
        });
    }
    if let Ok(monitors) = window.available_monitors() {
        for monitor in monitors {
            let area = monitor.work_area();
            let candidate = WorkArea {
                x: area.position.x,
                y: area.position.y,
                width: area.size.width,
                height: area.size.height,
            };
            if !result.contains(&candidate) {
                result.push(candidate);
            }
        }
    }
    result
}

fn intersection_area(state: SavedWindowState, area: WorkArea) -> u64 {
    let left = i64::from(state.x).max(i64::from(area.x));
    let top = i64::from(state.y).max(i64::from(area.y));
    let right = (i64::from(state.x) + i64::from(state.width))
        .min(i64::from(area.x) + i64::from(area.width));
    let bottom = (i64::from(state.y) + i64::from(state.height))
        .min(i64::from(area.y) + i64::from(area.height));
    if right <= left || bottom <= top {
        return 0;
    }
    ((right - left) as u64) * ((bottom - top) as u64)
}

fn normalize_state(mut state: SavedWindowState, areas: &[WorkArea]) -> SavedWindowState {
    state.width = state.width.max(MIN_WIDTH);
    state.height = state.height.max(MIN_HEIGHT);
    let Some(target) = areas
        .iter()
        .copied()
        .max_by_key(|area| intersection_area(state, *area))
        .filter(|area| intersection_area(state, *area) > 0)
        .or_else(|| areas.first().copied())
    else {
        return state;
    };

    state.width = state.width.min(target.width.max(MIN_WIDTH));
    state.height = state.height.min(target.height.max(MIN_HEIGHT));

    let had_visible_overlap = intersection_area(state, target) > 0;
    if had_visible_overlap {
        let max_x = i64::from(target.x) + i64::from(target.width.saturating_sub(state.width));
        let max_y = i64::from(target.y) + i64::from(target.height.saturating_sub(state.height));
        state.x = i64::from(state.x).clamp(i64::from(target.x), max_x) as i32;
        state.y = i64::from(state.y).clamp(i64::from(target.y), max_y) as i32;
    } else {
        state.x = i64::from(target.x)
            .saturating_add(i64::from(target.width.saturating_sub(state.width) / 2))
            as i32;
        state.y = i64::from(target.y)
            .saturating_add(i64::from(target.height.saturating_sub(state.height) / 2))
            as i32;
    }
    state
}

fn capture_window(window: &WebviewWindow) -> Option<SavedWindowState> {
    let maximized = window.is_maximized().ok()?;
    if maximized {
        if let Ok(cache) = state_cache().lock() {
            if let Some(mut state) = *cache {
                state.maximized = true;
                return Some(state);
            }
        }
    }
    let position = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    Some(SavedWindowState {
        x: position.x,
        y: position.y,
        width: size.width.max(MIN_WIDTH),
        height: size.height.max(MIN_HEIGHT),
        maximized,
    })
}

fn update_cache(window: &WebviewWindow) {
    let Some(state) = capture_window(window) else {
        return;
    };
    if let Ok(mut cache) = state_cache().lock() {
        *cache = Some(state);
    }
}

fn persist_cached(app: &AppHandle) {
    let Some(path) = state_path(app) else {
        return;
    };
    let state = state_cache().lock().ok().and_then(|cache| *cache);
    let Some(state) = state else {
        return;
    };
    let Ok(json) = serde_json::to_vec_pretty(&state) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let temp = path.with_extension("json.tmp");
    if fs::write(&temp, json).is_ok() {
        let _ = fs::remove_file(&path);
        let _ = fs::rename(&temp, &path);
    }
}

fn signal_save() {
    if let Some(sender) = SAVE_SIGNAL.get() {
        let _ = sender.try_send(());
    }
}

fn start_save_worker(app: AppHandle) {
    let (sender, receiver) = sync_channel::<()>(1);
    if SAVE_SIGNAL.set(sender).is_err() {
        return;
    }
    std::thread::spawn(move || loop {
        if receiver.recv().is_err() {
            break;
        }
        loop {
            match receiver.recv_timeout(SAVE_DEBOUNCE) {
                Ok(()) => continue,
                Err(RecvTimeoutError::Timeout) => {
                    persist_cached(&app);
                    break;
                }
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    });
}

pub(crate) fn initialize(app: &AppHandle, window: &WebviewWindow) {
    start_save_worker(app.clone());
    let restored = state_path(app)
        .and_then(|path| fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice::<SavedWindowState>(&bytes).ok())
        .map(|state| normalize_state(state, &work_areas(window)));

    if let Some(state) = restored {
        let _ = window.set_size(PhysicalSize::new(state.width, state.height));
        let _ = window.set_position(PhysicalPosition::new(state.x, state.y));
        if state.maximized {
            let _ = window.maximize();
        }
        if let Ok(mut cache) = state_cache().lock() {
            *cache = Some(state);
        }
    } else {
        update_cache(window);
    }
}

pub(crate) fn handle_event(window: &WebviewWindow, event: &WindowEvent) {
    match event {
        WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
            update_cache(window);
            signal_save();
        }
        WindowEvent::CloseRequested { .. } => {
            update_cache(window);
            persist_cached(window.app_handle());
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_state, SavedWindowState, WorkArea, MIN_HEIGHT, MIN_WIDTH};

    #[test]
    fn offscreen_state_moves_back_to_primary_work_area() {
        let state = SavedWindowState {
            x: 5000,
            y: 4000,
            width: 1200,
            height: 800,
            maximized: false,
        };
        let normalized = normalize_state(
            state,
            &[WorkArea {
                x: 0,
                y: 0,
                width: 1920,
                height: 1040,
            }],
        );
        assert_eq!(normalized.x, 360);
        assert_eq!(normalized.y, 120);
        assert_eq!(normalized.width, 1200);
        assert_eq!(normalized.height, 800);
    }

    #[test]
    fn oversized_state_is_clamped_and_minimum_size_is_preserved() {
        let oversized = SavedWindowState {
            x: -200,
            y: -100,
            width: 9000,
            height: 7000,
            maximized: true,
        };
        let area = WorkArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1040,
        };
        let normalized = normalize_state(oversized, &[area]);
        assert_eq!(normalized.x, 0);
        assert_eq!(normalized.y, 0);
        assert_eq!(normalized.width, 1920);
        assert_eq!(normalized.height, 1040);
        assert!(normalized.maximized);

        let tiny = SavedWindowState {
            x: 20,
            y: 30,
            width: 100,
            height: 100,
            maximized: false,
        };
        let normalized = normalize_state(tiny, &[area]);
        assert_eq!(normalized.width, MIN_WIDTH);
        assert_eq!(normalized.height, MIN_HEIGHT);
    }
}
