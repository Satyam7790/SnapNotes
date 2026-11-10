use once_cell::sync::Lazy;
use std::{fs, path::{Path,PathBuf}, sync::Mutex};
use uuid::Uuid;
static ASSETS: Lazy<Mutex<Option<PathBuf>>> = Lazy::new(|| Mutex::new(None));
pub fn init(path:PathBuf)->Result<(),std::io::Error>{fs::create_dir_all(&path)?;*ASSETS.lock().map_err(|_|std::io::Error::other("asset storage unavailable"))?=Some(path);Ok(())}
pub fn import_image(source:&str,workspace:&str)->Result<String,String>{
    let src=Path::new(source); let ext=src.extension().and_then(|s|s.to_str()).unwrap_or("").to_lowercase();
    if !matches!(ext.as_str(),"png"|"jpg"|"jpeg"|"webp"){return Err("Unsupported image format".into())}
    let base=ASSETS.lock().map_err(|_|"Storage unavailable")?.clone().ok_or("Storage unavailable")?;
    let dir=base.join(workspace);fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
    let name=format!("{}.{}",Uuid::new_v4(),ext);let dest=dir.join(name);fs::copy(src,&dest).map_err(|e|e.to_string())?;
    Ok(dest.to_string_lossy().to_string())
}
pub fn delete_workspace_assets(workspace:&str)->Result<(),std::io::Error>{let base=ASSETS.lock().map_err(|_|std::io::Error::other("asset storage unavailable"))?.clone().ok_or_else(||std::io::Error::other("asset storage unavailable"))?;let dir=base.join(workspace);if dir.exists(){fs::remove_dir_all(dir)?}Ok(())}
