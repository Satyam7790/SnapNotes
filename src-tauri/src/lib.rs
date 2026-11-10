mod database;
mod file_manager;

use database::{Workspace, DB};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{collections::HashSet, path::{Path, PathBuf}, sync::Mutex, time::Duration};
use tauri::{Emitter, Manager, RunEvent, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

static WATCHER: Lazy<Mutex<Option<notify::RecommendedWatcher>>> = Lazy::new(|| Mutex::new(None));
static SEEN_FILES: Lazy<Mutex<HashSet<PathBuf>>> = Lazy::new(|| Mutex::new(HashSet::new()));
static PENDING_DOCK_FILES: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[derive(Deserialize)]
struct SaveInput { id: String, title: String, document: serde_json::Value }

#[tauri::command]
fn list_workspaces(query: Option<String>) -> Result<Vec<Workspace>, String> {
    DB.lock().map_err(|_| "Storage is unavailable".to_string())?.list(query.as_deref()).map_err(|_| "Could not load workspaces".to_string())
}
#[tauri::command]
fn get_workspace(id: String) -> Result<Workspace, String> {
    DB.lock().map_err(|_| "Storage is unavailable".to_string())?.get(&id).map_err(|_| "Could not open workspace".to_string())?
        .ok_or_else(|| "This workspace could not be found".to_string())
}
#[tauri::command]
fn save_workspace(input: SaveInput) -> Result<Workspace, String> {
    DB.lock().map_err(|_| "Storage is unavailable".to_string())?.save(&input.id, &input.title, &input.document).map_err(|_| "Could not save workspace. Your changes are still open; try again.".to_string())
}
#[tauri::command]
fn create_workspace(title: String, image_paths: Vec<String>) -> Result<Workspace, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let mut content = Vec::new();
    for source in image_paths {
        let path = file_manager::import_image(&source, &id).map_err(|_| format!("Could not import image: {}", Path::new(&source).file_name().unwrap_or_default().to_string_lossy()))?;
        content.push(serde_json::json!({"type":"image", "attrs":{"src":path, "alt":"Screenshot", "title":null, "width":null, "height":null}}));
        content.push(serde_json::json!({"type":"paragraph"}));
    }
    let doc = serde_json::json!({"type":"doc", "content":content});
    DB.lock().map_err(|_| "Storage is unavailable".to_string())?.save(&id, &title, &doc).map_err(|_| "Could not create workspace".to_string())
}
#[tauri::command]
fn add_image(path: String, workspace_id: String) -> Result<String, String> {
    file_manager::import_image(&path, &workspace_id).map_err(|_| "Could not import this image".to_string())
}
#[tauri::command]
fn delete_workspace(id: String) -> Result<(), String> {
    DB.lock().map_err(|_| "Storage is unavailable".to_string())?.delete(&id).map_err(|_| "Could not delete workspace".to_string())?;
    let _ = file_manager::delete_workspace_assets(&id); Ok(())
}
#[tauri::command]
fn rename_workspace(id: String, title: String) -> Result<Workspace, String> {
    let current = DB.lock().map_err(|_| "Storage is unavailable".to_string())?.get(&id).map_err(|_| "Could not open workspace".to_string())?.ok_or("Workspace not found")?;
    DB.lock().map_err(|_| "Storage is unavailable".to_string())?.save(&id, &title, &current.document).map_err(|_| "Could not rename workspace".to_string())
}
#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let path = app.path().app_config_dir().map_err(|_| "Settings unavailable".to_string())?.join("settings.json");
    let mut settings: serde_json::Value = std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(serde_json::json!({"watch_dir":""}));
    if settings["watch_dir"].as_str().unwrap_or("").is_empty() { if let Ok(desktop) = app.path().desktop_dir() { settings["watch_dir"] = desktop.to_string_lossy().to_string().into(); } }
    Ok(settings)
}
#[tauri::command]
fn show_capture(app: tauri::AppHandle) { show_capture_window(&app); }
#[tauri::command]
fn show_main(app: tauri::AppHandle) { show_main_window(&app); }
#[tauri::command]
fn take_pending_dock_images() -> Vec<String> {
    PENDING_DOCK_FILES.lock().map(|mut files| files.drain(..).collect()).unwrap_or_default()
}
#[tauri::command]
fn set_watch_directory(app: tauri::AppHandle, path: String) -> Result<(), String> {
    start_watcher(&app, PathBuf::from(&path))?;
    let config = app.path().app_config_dir().map_err(|_| "Could not access settings".to_string())?;
    std::fs::create_dir_all(&config).map_err(|_| "Could not save settings".to_string())?;
    std::fs::write(config.join("settings.json"), serde_json::json!({"watch_dir":path}).to_string()).map_err(|_| "Could not save settings".to_string())
}

fn is_image(path: &Path) -> bool { path.extension().and_then(|e| e.to_str()).map(|e| matches!(e.to_lowercase().as_str(), "png"|"jpg"|"jpeg"|"webp")).unwrap_or(false) }
fn start_watcher(app: &tauri::AppHandle, dir: PathBuf) -> Result<(), String> {
    if !dir.is_dir() { return Err("That folder cannot be accessed. Choose an existing folder.".into()); }
    let handle = app.clone();
    let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
        if let Ok(event) = result {
            if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                for path in event.paths {
                    if !is_image(&path) { continue; }
                    let app = handle.clone();
                    std::thread::spawn(move || {
                        let mut last_size = 0;
                        let mut stable = 0;
                        for _ in 0..20 {
                            std::thread::sleep(Duration::from_millis(250));
                            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                            if size > 0 && size == last_size { stable += 1; } else { stable = 0; }
                            last_size = size;
                            if stable >= 2 {
                                if let Ok(mut seen) = SEEN_FILES.lock() {
                                    if seen.insert(path.clone()) { let _ = app.emit("screenshot-detected", path.to_string_lossy().to_string()); }
                                }
                                break;
                            }
                        }
                    });
                }
            }
        }
    }).map_err(|_| "Could not watch that folder".to_string())?;
    watcher.watch(&dir, RecursiveMode::NonRecursive).map_err(|_| "Could not watch that folder".to_string())?;
    *WATCHER.lock().map_err(|_| "Watcher is unavailable".to_string())? = Some(watcher);
    Ok(())
}
fn show_capture_window(app: &tauri::AppHandle) { if let Some(w) = app.get_webview_window("capture") { let _ = w.show(); let _ = w.set_focus(); } }
fn show_main_window(app: &tauri::AppHandle) { if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.set_focus(); } }

pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init()).plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().with_handler(|app, shortcut, event| {
            let capture = Shortcut::new(Some(Modifiers::ALT | Modifiers::SHIFT), Code::KeyS);
            if shortcut == &capture && event.state() == ShortcutState::Pressed { show_capture_window(app); }
        }).build())
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data)?;
            DB.lock().map_err(|e| format!("Database initialization failed: {e}"))?.open(data.join("database.sqlite"))?;
            file_manager::init(data.join("assets"))?;
            let shortcut = Shortcut::new(Some(Modifiers::ALT | Modifiers::SHIFT), Code::KeyS);
            app.global_shortcut().register(shortcut)?;
            let handle = app.handle().clone();
            let open = tauri::menu::MenuItem::with_id(app, "open", "Open Quickspace", true, None::<&str>)?;
            let capture = tauri::menu::MenuItem::with_id(app, "capture", "Open capture popup", true, None::<&str>)?;
            let create = tauri::menu::MenuItem::with_id(app, "create", "New workspace", true, None::<&str>)?;
            let settings = tauri::menu::MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit = tauri::menu::MenuItem::with_id(app, "quit", "Quit Quickspace", true, None::<&str>)?;
            let menu = tauri::menu::Menu::with_items(app, &[&open, &capture, &create, &settings, &quit])?;
            let tray_icon = tauri::tray::TrayIconBuilder::new().menu(&menu).on_menu_event(move |app, event| match event.id().as_ref() {
                "open" => show_main_window(app), "capture" => show_capture_window(app), "create" => { show_main_window(app); let _ = app.emit("new-workspace", ()); },
                "settings" => { show_main_window(app); let _ = app.emit("open-settings", ()); }, "quit" => app.exit(0), _ => {}
            }).build(app)?;
            let _ = tray_icon;
            let settings_path = app.path().app_config_dir()?.join("settings.json");
            let configured = std::fs::read_to_string(settings_path).ok().and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok()).and_then(|v| v["watch_dir"].as_str().map(str::to_owned)).filter(|s| !s.is_empty());
            let watch_dir = configured.map(PathBuf::from).or_else(|| app.path().desktop_dir().ok());
            if let Some(dir) = watch_dir { let _ = start_watcher(&handle, dir); }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![list_workspaces,get_workspace,save_workspace,create_workspace,add_image,delete_workspace,rename_workspace,get_settings,set_watch_directory,show_capture,show_main,take_pending_dock_images]);
    let app = builder.on_window_event(|window, event| { if window.label() == "main" { if let WindowEvent::CloseRequested { api, .. } = event { api.prevent_close(); let _ = window.hide(); } } }).build(tauri::generate_context!()).expect("error while building Quickspace");
    app.run(|app, event| {
        #[cfg(target_os = "macos")]
        if let RunEvent::Opened { urls } = event {
            let paths: Vec<String> = urls.into_iter()
                .filter_map(|url| url.to_file_path().ok())
                .filter(|path| is_image(path))
                .map(|path| path.to_string_lossy().into_owned())
                .collect();
            if !paths.is_empty() {
                if let Ok(mut pending) = PENDING_DOCK_FILES.lock() { pending.extend(paths); }
                show_capture_window(app);
                let _ = app.emit("dock-files-opened", ());
            }
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (app, event);
    });
}
