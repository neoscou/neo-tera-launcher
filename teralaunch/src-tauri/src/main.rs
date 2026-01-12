#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Standard library imports
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant, SystemTime};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

// Debug logging helper
fn debug_log(msg: &str) {
  let log_path = std::env::current_exe()
    .ok()
    .and_then(|p| p.parent().map(|p| p.join("debug.log")))
    .unwrap_or_else(|| PathBuf::from("debug.log"));
  
  if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
    use std::io::Write;
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let _ = writeln!(file, "[{}] {}", timestamp, msg);
  }
  println!("{}", msg);
}

// Clear debug log at startup to prevent file from growing indefinitely
fn clear_debug_log() {
  let log_path = std::env::current_exe()
    .ok()
    .and_then(|p| p.parent().map(|p| p.join("debug.log")))
    .unwrap_or_else(|| PathBuf::from("debug.log"));
  
  // Delete the file if it exists
  let _ = std::fs::remove_file(&log_path);
}

// Third-party imports
use dotenv::dotenv;
use log::{LevelFilter, error, info, warn};
use tokio::sync::{watch, Mutex, mpsc};
use rayon::prelude::*;
use tokio::runtime::Runtime;
use serde::{Deserialize, Serialize};
use serde_json::{json};
use tauri::{Manager};
use tauri::api::dialog::FileDialogBuilder;
use teralib::{get_game_status_receiver, run_game, reset_global_state};
use teralib::config::get_config_value;
use reqwest::Client;
use lazy_static::lazy_static;
use ini::Ini;
use sha2::{Sha256, Digest};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use walkdir::WalkDir;
use reqwest::cookie::Jar;
use reqwest::cookie::CookieStore;
use url::Url;

// Struct definitions
#[derive(Serialize, Deserialize)]
struct LoginResponse {
  #[serde(rename = "Return")]
  return_value: bool,
  #[serde(rename = "ReturnCode")]
  return_code: i32,
  #[serde(rename = "Msg")]
  msg: String,
  #[serde(rename = "CharacterCount")]
  character_count: String,
  #[serde(rename = "Permission")]
  permission: i32,
  #[serde(rename = "Privilege")]
  privilege: i32,
  #[serde(rename = "UserNo")]
  user_no: i32,
  #[serde(rename = "UserName")]
  user_name: String,
  #[serde(rename = "AuthKey")]
  auth_key: String,
}

#[derive(Serialize)]
struct AuthInfo {
  character_count: String,
  permission: i32,
  privilege: i32,
  user_no: i32,
  user_name: String,
  auth_key: String,
}

struct GlobalAuthInfo {
  character_count: String,
  user_no: i32,
  user_name: String,
  auth_key: String,
}

lazy_static! {
  static ref GLOBAL_AUTH_INFO: RwLock<GlobalAuthInfo> = RwLock::new(GlobalAuthInfo {
    character_count: String::new(),
    user_no: 0,
    user_name: String::new(),
    auth_key: String::new(),
  });

  static ref AUTHENTICATED_CLIENT: Mutex<Option<Client>> = Mutex::new(None);

  // Client for registration flow - maintains session cookies between register and verify calls
  static ref REGISTRATION_CLIENT: Mutex<Option<Client>> = Mutex::new(None);

  static ref LAUNCHER_BASE_URL: String = get_config_value("LAUNCHER_ACTION_URL");

  static ref GLOBAL_ACTS_MAP: RwLock<HashMap<String, String>> = RwLock::new(HashMap::new());
  static ref GLOBAL_PAGES_MAP: RwLock<HashMap<String, String>> = RwLock::new(HashMap::new());
}

// Struct for the initial /launcher/LoginAction response
#[derive(Deserialize)]
struct InitialLoginResponse {
  #[serde(rename = "Return")]
  return_value: bool,
  #[serde(rename = "Msg")]
  msg: String,
  #[serde(rename = "ReturnCode")]
  return_code: i32,
}

// Struct for the /launcher/GetAccountInfoAction response
#[derive(Deserialize)]
struct AccountInfoResponse {
  #[serde(rename = "UserNo")]
  user_no: i32,
  #[serde(rename = "UserName")]
  user_name: String,
  #[serde(rename = "Permission")]
  permission: i32,
  #[serde(rename = "Privilege")]
  privilege: i32,
  #[serde(rename = "Banned", default)] 
  banned: bool,
}

// Struct for the /launcher/GetAuthKeyAction response
#[derive(Deserialize)]
struct AuthKeyResponse {
  #[serde(rename = "AuthKey")]
  auth_key: String,
}

// Struct for the /launcher/GetCharacterCountAction response
#[derive(Deserialize)]
struct CharCountResponse {
  #[serde(rename = "CharacterCount")]
  character_count: String,
}

// Struct for the /launcher/GetMaintenanceStatusAction response
#[derive(Deserialize, Debug, Clone, Serialize)]
struct MaintenanceResponse {
  #[serde(rename = "Return")]
  return_value: bool,
  #[serde(rename = "ReturnCode")]
  return_code: i32,
  #[serde(rename = "Msg")]
  msg: String,
  #[serde(rename = "StartTime")]
  start_time: Option<u64>,
  #[serde(rename = "EndTime")]
  end_time: Option<u64>,
}

// This struct combines all info into the format the frontend expects (same as old LoginResponse)
#[derive(Serialize)]
struct CombinedLoginResponse {
  #[serde(rename = "Return")]
  return_value: bool,
  #[serde(rename = "ReturnCode")]
  return_code: i32,
  #[serde(rename = "Msg")]
  msg: String,
  #[serde(rename = "CharacterCount")]
  character_count: String,
  #[serde(rename = "Permission")]
  permission: i32,
  #[serde(rename = "Privilege")]
  privilege: i32,
  #[serde(rename = "UserNo")]
  user_no: i32,
  #[serde(rename = "UserName")]
  user_name: String,
  #[serde(rename = "AuthKey")]
  auth_key: String,
  #[serde(rename = "Banned")]
  banned: bool,

  #[serde(rename = "ActsMap", skip_serializing_if = "Option::is_none")]
    acts_map: Option<serde_json::Value>,
    #[serde(rename = "PagesMap", skip_serializing_if = "Option::is_none")]
    pages_map: Option<serde_json::Value>,

  session_cookie: Option<String>,
}

/* const CONFIG: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config/config.json"));

lazy_static::lazy_static! {
  static ref CONFIG_JSON: Value = serde_json::from_str(CONFIG).expect("Failed to parse config");
} */


#[derive(Debug, Serialize, Deserialize, Clone)]
struct FileInfo {
  path: String,
  hash: String,
  size: u64,
  url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DirectoryInfo {
  path: String,
  hash: String,
  file_count: usize,
  total_size: u64,
}

// Launcher self-update structs
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LauncherUpdateManifest {
  version: String,
  download_url: String,
  changelog: Option<String>,
  hash: Option<String>,
  file_name: String,
  release_date: Option<String>,
  mandatory: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
struct LauncherUpdateInfo {
  update_available: bool,
  current_version: String,
  latest_version: String,
  download_url: Option<String>,
  changelog: Option<String>,
  mandatory: bool,
}

#[derive(Clone, Serialize)]
struct LauncherUpdateProgress {
  progress: f64,
  downloaded_bytes: u64,
  total_bytes: u64,
  speed: f64,
  status: String,
}

#[derive(Clone, Serialize)]
struct ProgressPayload {
  file_name: String,
  progress: f64,
  speed: f64,
  downloaded_bytes: u64,
  total_bytes: u64,
  total_files: usize,
  elapsed_time: f64,
  current_file_index: usize,
}

#[derive(Clone, Serialize)]
struct FileCheckProgress {
  current_file: String,
  progress: f64,
  current_count: usize,
  total_files: usize,
  elapsed_time: f64,
  files_to_update: usize,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
struct CachedFileInfo {
  hash: String,
  last_modified: SystemTime,
}

struct GameState {
  status_receiver: Arc<Mutex<watch::Receiver<bool>>>,
  is_launching: Arc<Mutex<bool>>,
}


//static INIT: Once = Once::new();


lazy_static! {
  static ref HASH_CACHE: Mutex<HashMap<String, CachedFileInfo>> = Mutex::new(HashMap::new());
}


/* fn get_config_value(key: &str) -> String {
  CONFIG_JSON[key].as_str().expect(&format!("{} must be set in config.json", key)).to_string()
} */

fn is_ignored(path: &Path, game_path: &Path, ignored_paths: &HashSet<&str>) -> bool {
  let relative_path = path.strip_prefix(game_path).unwrap().to_str().unwrap().replace("\\", "/");

  // Ignore files at the root
  if relative_path.chars().filter(|&c| c == '/').count() == 0 {
    return true;
  }

  // Check if the path is in the list of ignored paths
  for ignored_path in ignored_paths {
    if relative_path.starts_with(ignored_path) {
      return true;
    }
  }

  false
}

async fn get_server_hash_file() -> Result<serde_json::Value, String> {
  let url = get_hash_file_url();
  debug_log(&format!("DEBUG: Fetching hash file from: {}", url));
  let client = reqwest::Client::new();
  let res = client
    .get(&url)
    .header("Cache-Control", "no-cache, no-store, must-revalidate")
    .header("Pragma", "no-cache")
    .header("Expires", "0")
    .send().await
    .map_err(|e| e.to_string())?;
  debug_log(&format!("DEBUG: Got response, status: {}", res.status()));
  
  // Read as text first to handle BOM
  let text = res.text().await.map_err(|e| {
    debug_log(&format!("ERROR: Failed to read response text: {}", e));
    e.to_string()
  })?;
  
  // Strip BOM if present
  let text = text.trim_start_matches('\u{FEFF}');
  
  debug_log(&format!("DEBUG: Response text length: {} chars", text.len()));
  
  let json: serde_json::Value = serde_json::from_str(text).map_err(|e| {
    debug_log(&format!("ERROR: Failed to parse JSON: {}", e));
    debug_log(&format!("First 200 chars: {}", &text[..text.len().min(200)]));
    e.to_string()
  })?;
  
  debug_log("DEBUG: JSON parsed successfully");
  
  // Debug: Check DataCenter entry
  if let Some(files) = json["files"].as_array() {
    debug_log(&format!("DEBUG: Hash file contains {} files", files.len()));
    if let Some(dc_entry) = files.iter().find(|f| f["path"].as_str() == Some("S1Game/S1Data/DataCenter_Final_EUR.dat")) {
      debug_log(&format!("DEBUG: DataCenter in hash file - Hash: {}, Size: {}", 
        dc_entry["hash"].as_str().unwrap_or("MISSING"),
        dc_entry["size"].as_u64().unwrap_or(0)));
    } else {
      debug_log("DEBUG: DataCenter entry NOT FOUND in hash file!");
    }
  } else {
    debug_log("ERROR: 'files' field is not an array or missing!");
  }
  
  Ok(json)
}


fn calculate_file_hash<P: AsRef<Path>>(path: P) -> Result<String, String> {
  let mut file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
  let mut hasher = Sha256::new();
  let mut buffer = [0; 1024];

  loop {
    let bytes_read = file.read(&mut buffer).map_err(|e| format!("Failed to read file: {}", e))?;
    if bytes_read == 0 {
      break;
    }
    hasher.update(&buffer[..bytes_read]);
  }

  let result = hasher.finalize();
  Ok(format!("{:x}", result))
}

fn get_cache_file_path() -> Result<PathBuf, String> {
  let mut path = std::env::current_exe().map_err(|e| e.to_string())?;
  path.pop();
  path.push("file_cache.json");
  Ok(path)
}

fn save_cache_to_disk(cache: &HashMap<String, CachedFileInfo>) -> Result<(), String> {
  let cache_path = get_cache_file_path()?;
  let serialized = serde_json::to_string(cache).map_err(|e| e.to_string())?;
  let mut file = File::create(cache_path).map_err(|e| e.to_string())?;
  file.write_all(serialized.as_bytes()).map_err(|e| e.to_string())?;
  Ok(())
}

fn load_cache_from_disk() -> Result<HashMap<String, CachedFileInfo>, String> {
  let cache_path = get_cache_file_path()?;
  let mut file = File::open(cache_path).map_err(|e| e.to_string())?;
  let mut contents = String::new();
  file.read_to_string(&mut contents).map_err(|e| e.to_string())?;
  let cache: HashMap<String, CachedFileInfo> = serde_json::from_str(&contents).map_err(|e| e.to_string())?;
  Ok(cache)
}


fn get_hash_file_url() -> String {
  get_config_value("HASH_FILE_URL")
}

fn get_files_server_url() -> String {
  get_config_value("FILE_SERVER_URL")
}

fn get_launcher_update_url() -> String {
  get_config_value("LAUNCHER_UPDATE_URL")
}

fn get_current_launcher_version() -> String {
  env!("CARGO_PKG_VERSION").to_string()
}

fn compare_versions(current: &str, latest: &str) -> std::cmp::Ordering {
  // Parse version strings like "0.0.6" or "1.1.0.0" 
  let current_parts: Vec<u32> = current.split('.').filter_map(|s| s.parse().ok()).collect();
  let latest_parts: Vec<u32> = latest.split('.').filter_map(|s| s.parse().ok()).collect();
  
  let max_len = current_parts.len().max(latest_parts.len());
  
  for i in 0..max_len {
    let current_num = current_parts.get(i).copied().unwrap_or(0);
    let latest_num = latest_parts.get(i).copied().unwrap_or(0);
    
    match current_num.cmp(&latest_num) {
      std::cmp::Ordering::Less => return std::cmp::Ordering::Less,
      std::cmp::Ordering::Greater => return std::cmp::Ordering::Greater,
      std::cmp::Ordering::Equal => continue,
    }
  }
  
  std::cmp::Ordering::Equal
}

fn find_config_file() -> Option<PathBuf> {
  let current_dir = env::current_dir().ok()?;
  let config_in_current = current_dir.join("tera_config.ini");
  if config_in_current.exists() {
    return Some(config_in_current);
  }

  let parent_dir = current_dir.parent()?;
  let config_in_parent = parent_dir.join("tera_config.ini");
  if config_in_parent.exists() {
    return Some(config_in_parent);
  }

  if let Ok(exe_path) = env::current_exe() {
    if let Some(exe_dir) = exe_path.parent() {
      let config_in_exe_dir = exe_dir.join("tera_config.ini");
      if config_in_exe_dir.exists() {
        return Some(config_in_exe_dir);
      }
    }
  }

  None
}

fn load_config() -> Result<(PathBuf, String), String> {
  let config_path = find_config_file().ok_or("Config file not found")?;
  let conf = Ini::load_from_file(&config_path).map_err(|e|
    format!("Failed to load config: {}", e)
  )?;

  let section = conf.section(Some("game")).ok_or("Game section not found in config")?;

  let game_path = section.get("path").ok_or("Game path not found in config")?;

  let game_path = PathBuf::from(game_path);

  let game_lang = section.get("lang").ok_or("Game language not found in config")?.to_string();

  Ok((game_path, game_lang))
}

/* fn save_config(game_path: &Path, game_lang: &str) -> Result<(), String> {
  let config_path = find_config_file().ok_or("Config file not found")?;
  let mut conf = Ini::new();

  conf.with_section(Some("game")).set("path", game_path.to_str().ok_or("Invalid game path")?);
  conf.with_section(Some("game")).set("lang", game_lang);

  let mut file = std::fs::File
    ::create(&config_path)
    .map_err(|e| format!("Failed to create config file: {}", e))?;

  conf.write_to(&mut file).map_err(|e| format!("Failed to write config: {}", e))?;

  Ok(())
} */

async fn get_maintenance_status() -> Result<MaintenanceResponse, String> { 
  let client = reqwest::Client::new();
  let base_url = &*LAUNCHER_BASE_URL; 
  let maintenance_url = format!("{}/launcher/GetMaintenanceStatusAction", base_url);

  let res = client
    .get(&maintenance_url)
    .send().await
    .map_err(|e| format!("Failed to connect to maintenance server: {}", e))?;

  if !res.status().is_success() {
    return Err(format!("Maintenance check request failed with status: {}", res.status()));
  }

  let maintenance_body: MaintenanceResponse = res
    .json()
    .await
    .map_err(|e| format!("Failed to parse maintenance response: {}", e))?;

  if !maintenance_body.return_value {
    return Err(format!("Maintenance check API error: {}", maintenance_body.msg));
  }

  Ok(maintenance_body)
}

#[tauri::command]
async fn check_maintenance_and_notify(window: tauri::Window) -> Result<bool, String> {
  match get_maintenance_status().await {
    Ok(response) => {
      let is_maintenance = response.start_time.is_some() || response.end_time.is_some();

      if is_maintenance {
        // Emit the event with full maintenance details for the modal
        let payload = serde_json::to_value(&response)
          .unwrap_or(json!({"msg": "Active maintenance"}));

        if let Err(e) = window.emit("maintenance_active", payload) {
          error!("Failed to emit maintenance_active event: {:?}", e);
        }
      }

      // Return 'true' if maintenance is active, 'false' otherwise
      Ok(is_maintenance)
    }
    Err(e) => {
      error!("Error checking maintenance status: {:?}", e);
      // Return a specific error so the frontend can handle it as a network issue
      Err(format!("ERROR_NETWORK_CHECK: {}", e))
    }
  }
}


#[tauri::command]
async fn generate_hash_file(window: tauri::Window) -> Result<String, String> {
  let start_time = Instant::now();

  let game_path = get_game_path().map_err(|e| e.to_string())?;
  info!("Game path: {:?}", game_path);
  let output_path = game_path.join("hash-file.json");
  info!("Output path: {:?}", output_path);

  // Only scan the S1Game folder for hash generation
  let s1game_path = game_path.join("S1Game");
  if !s1game_path.exists() {
    return Err("S1Game folder not found in game directory".to_string());
  }
  info!("Scanning S1Game folder: {:?}", s1game_path);

  // List of files and directories to ignore (relative to S1Game)
  let ignored_paths: HashSet<&str> = [
    "GuildFlagUpload",
    "GuildLogoUpload",
    "ImageCache",
    "Logs",
    "Screenshots",
    "Config/S1Engine.ini",
    "Config/S1Game.ini",
    "Config/S1Input.ini",
    "Config/S1Lightmass.ini",
    "Config/S1Option.ini",
    "Config/S1SystemSettings.ini",
    "Config/S1TBASettings.ini",
    "Config/S1UI.ini",
  ].iter().cloned().collect();

  let total_files = WalkDir::new(&s1game_path)
    .into_iter()
    .filter_map(|e| e.ok())
    .filter(|e| e.file_type().is_file())
    .filter(|e| !is_ignored(e.path(), &s1game_path, &ignored_paths))
    .count();
  info!("Total files to process: {}", total_files);

  let progress_bar = ProgressBar::new(total_files as u64);
  let progress_style = ProgressStyle::default_bar()
    .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
    .map_err(|e| e.to_string())?
    .progress_chars("##-");
  progress_bar.set_style(progress_style);

  let processed_files = AtomicU64::new(0);
  let total_size = AtomicU64::new(0);
  let files = Arc::new(Mutex::new(Vec::new()));

  let result: Result<(), String> = WalkDir::new(&s1game_path)
    .into_iter()
    .par_bridge()
    .try_for_each(|entry| -> Result<(), String> {
      let entry = entry.map_err(|e| e.to_string())?;
      let path = entry.path();
      if path.is_file() && !is_ignored(path, &s1game_path, &ignored_paths) {
        // Get path relative to S1Game folder, then prefix with "S1Game/" for the full relative path
        let s1game_relative = path.strip_prefix(&s1game_path).unwrap().to_str().unwrap().replace("\\", "/");
        let relative_path = format!("S1Game/{}", s1game_relative);
        info!("Processing file: {}", relative_path);

        let contents = std::fs::read(path).map_err(|e| e.to_string())?;
        let mut hasher = Sha256::new();
        hasher.update(&contents);
        let hash = format!("{:x}", hasher.finalize());
        let size = contents.len() as u64;
        let file_server_url = get_config_value("FILE_SERVER_URL");
        let url = format!("{}/{}", file_server_url, relative_path);

        files.blocking_lock().push(FileInfo {
          path: relative_path.clone(),
          hash,
          size,
          url,
        });

        total_size.fetch_add(size, Ordering::Relaxed);
        let current_processed = processed_files.fetch_add(1, Ordering::Relaxed) + 1;
        progress_bar.set_position(current_processed);

        let progress = (current_processed as f64 / total_files as f64) * 100.0;
        window.emit("hash_file_progress", json!({
          "current_file": relative_path,
          "progress": progress,
          "processed_files": current_processed,
          "total_files": total_files,
          "total_size": total_size.load(Ordering::Relaxed)
        })).map_err(|e| e.to_string())?;
      }
      Ok(())
    });

  if let Err(e) = result {
    error!("Error during file processing: {:?}", e);
    return Err(e);
  }

  progress_bar.finish_with_message("File processing completed");

  // Generate directory hashes for fast comparison
  info!("Generating directory hashes");
  let files_vec = files.lock().await.clone();
  
  // Group files by directory
  let mut dir_files: std::collections::HashMap<String, Vec<&FileInfo>> = std::collections::HashMap::new();
  for file in &files_vec {
    // Get the parent directory path
    let dir_path = if let Some(pos) = file.path.rfind('/') {
      file.path[..pos].to_string()
    } else {
      "S1Game".to_string()
    };
    dir_files.entry(dir_path).or_insert_with(Vec::new).push(file);
  }
  
  // Create directory hashes by combining all file hashes in each directory
  let mut directories: Vec<DirectoryInfo> = Vec::new();
  for (dir_path, dir_file_list) in &dir_files {
    // Sort files by path for consistent hash
    let mut sorted_files: Vec<&FileInfo> = dir_file_list.clone();
    sorted_files.sort_by(|a, b| a.path.cmp(&b.path));
    
    // Combine all file hashes to create directory hash
    let mut hasher = Sha256::new();
    let mut total_size: u64 = 0;
    for file in &sorted_files {
      hasher.update(file.hash.as_bytes());
      total_size += file.size;
    }
    let dir_hash = format!("{:x}", hasher.finalize());
    
    directories.push(DirectoryInfo {
      path: dir_path.clone(),
      hash: dir_hash,
      file_count: sorted_files.len(),
      total_size,
    });
  }
  
  info!("Generated {} directory hashes", directories.len());

  info!("Generating JSON");
  let json = serde_json::to_string(&json!({
    "directories": directories,
    "files": files_vec
  })).map_err(|e| e.to_string())?;

  info!("Writing hash file");
  let mut file = File::create(&output_path).map_err(|e| e.to_string())?;
  file.write_all(json.as_bytes()).map_err(|e| e.to_string())?;

  let duration = start_time.elapsed();
  let total_processed = processed_files.load(Ordering::Relaxed);
  let total_size = total_size.load(Ordering::Relaxed);
  info!("Hash file generation completed in {:?}", duration);
  info!("Total files processed: {}", total_processed);
  info!("Total size: {} bytes", total_size);

  Ok(format!("Hash file generated successfully. Processed {} files with a total size of {} bytes in {:?}", total_processed, total_size, duration))
}


#[tauri::command]
async fn select_game_folder() -> Result<String, String> {
  let (tx, mut rx) = mpsc::channel(1);

  FileDialogBuilder::new()
    .set_title("Select Tera Game Folder")
    .set_directory("/")
    .pick_folder(move |folder_path| {
      if let Some(path) = folder_path {
        let _ = tx.try_send(path);
      }
    });

  match rx.recv().await {
    Some(path) => Ok(path.to_string_lossy().into_owned()),
    None => Err("Folder selection cancelled or failed".into()),
  }
}


fn get_game_path() -> Result<PathBuf, String> {
  let (game_path, _) = load_config()?;
  Ok(game_path)
}

#[tauri::command]
fn get_game_path_string() -> Result<String, String> {
  let game_path = get_game_path()?;
  Ok(game_path.to_string_lossy().to_string())
}


#[tauri::command]
fn save_game_path_to_config(path: String) -> Result<(), String> {
  // Try to find existing config, or create a new one in the executable directory
  let config_path = match find_config_file() {
    Some(path) => path,
    None => {
      // Config doesn't exist - create it in the executable directory
      let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get executable path: {}", e))?
        .parent()
        .ok_or("Failed to get executable directory")?
        .to_path_buf();
      
      exe_dir.join("tera_config.ini")
    }
  };
  
  // Load existing config or create a new one
  let mut conf = if config_path.exists() {
    Ini::load_from_file(&config_path).map_err(|e|
      format!("Failed to load config: {}", e)
    )?
  } else {
    Ini::new()
  };

  // Set both path and default language
  conf.with_section(Some("game"))
    .set("lang", "EUR")
    .set("path", &path);

  conf.write_to_file(&config_path).map_err(|e| format!("Failed to write config: {}", e))?;

  Ok(())
}

#[tauri::command]
fn get_game_path_from_config() -> Result<String, String> {
  match get_game_path() {
    Ok(game_path) => game_path
      .to_str()
      .ok_or_else(|| "Invalid UTF-8 in game path".to_string())
      .map(|s| s.to_string()),
    Err(e) => {
      if e.contains("Config file not found") {
        Err("tera_config.ini is missing".to_string())
      } else {
        Err(e)
      }
    }
  }
}

#[tauri::command]
async fn check_update_required(window: tauri::Window) -> Result<bool, String> {
  match get_files_to_update(window).await {
    Ok(files) => Ok(!files.is_empty()),
    Err(e) => Err(e),
  }
}

async fn update_file(
  _app_handle: tauri::AppHandle,
  window: tauri::Window,
  client: &reqwest::Client,
  file_info: FileInfo,
  total_files: usize,
  current_file_index: usize,
  total_size: u64,
  downloaded_size: u64,
) -> Result<u64, String> {
  let game_path = get_game_path()?;
  let file_path = game_path.join(&file_info.path);

  if let Some(parent) = file_path.parent() {
    tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
  }

  let start_time = Instant::now();

  // Emit initial progress
  let initial_payload = ProgressPayload {
    file_name: file_info.path.clone(),
    progress: 0.0,
    speed: 0.0,
    downloaded_bytes: downloaded_size,
    total_bytes: total_size,
    total_files,
    elapsed_time: 0.0,
    current_file_index,
  };
  let _ = window.emit("download_progress", &initial_payload);

  let response = client.get(&file_info.url)
    .send()
    .await
    .map_err(|e| format!("Request failed: {}", e))?;
  
  let content_length = response.content_length().unwrap_or(file_info.size);
  
  // Stream the response with incremental hashing for real-time progress
  let mut stream = response.bytes_stream();
  let mut downloaded_bytes: u64 = 0;
  let mut all_bytes = Vec::with_capacity(content_length as usize);
  let mut last_progress_update = Instant::now();
  let mut hasher = Sha256::new(); // Compute hash incrementally as chunks arrive
  
  use futures_util::StreamExt;
  
  while let Some(chunk_result) = stream.next().await {
    let chunk = chunk_result.map_err(|e| format!("Failed to read chunk: {}", e))?;
    downloaded_bytes += chunk.len() as u64;
    hasher.update(&chunk); // Hash each chunk as it arrives
    all_bytes.extend_from_slice(&chunk);
    
    // Emit progress every 100ms to avoid overwhelming the UI
    if last_progress_update.elapsed().as_millis() >= 100 {
      let elapsed = start_time.elapsed();
      let elapsed_secs = elapsed.as_secs_f64();
      let speed = if elapsed_secs > 0.0 { (downloaded_bytes as f64 / elapsed_secs) as u64 } else { 0 };
      let file_progress = if content_length > 0 { (downloaded_bytes as f64 / content_length as f64) * 100.0 } else { 0.0 };
      
      let progress_payload = ProgressPayload {
        file_name: file_info.path.clone(),
        progress: file_progress,
        speed: speed as f64,
        downloaded_bytes: downloaded_size + downloaded_bytes,
        total_bytes: total_size,
        total_files,
        elapsed_time: elapsed_secs,
        current_file_index,
      };
      let _ = window.emit("download_progress", &progress_payload);
      last_progress_update = Instant::now();
    }
  }
  
  let downloaded = all_bytes.len() as u64;

  // Hash is already computed - just finalize it (instant)
  let downloaded_hash = format!("{:x}", hasher.finalize());

  if downloaded_hash.to_lowercase() != file_info.hash.to_lowercase() {
    return Err(format!("Hash mismatch for file: {} (expected: {}, got: {})", file_info.path, file_info.hash, downloaded_hash));
  }

  tokio::fs::write(&file_path, &all_bytes).await.map_err(|e| e.to_string())?;

  let elapsed = start_time.elapsed();
  let elapsed_secs = elapsed.as_secs_f64();
  let speed = if elapsed_secs > 0.0 { (downloaded as f64 / elapsed_secs) as u64 } else { downloaded };

  println!("Downloaded {} ({} bytes) in {:.2}s at {}/s", file_info.path, downloaded, elapsed_secs, format_bytes(speed));

  // Emit completion progress
  let final_payload = ProgressPayload {
    file_name: file_info.path.clone(),
    progress: 100.0,
    speed: speed as f64,
    downloaded_bytes: downloaded_size + downloaded,
    total_bytes: total_size,
    total_files,
    elapsed_time: elapsed.as_secs_f64(),
    current_file_index,
  };
  let _ = window.emit("download_progress", &final_payload);

  // Emit a final event for this file
  let final_progress_payload = ProgressPayload {
    file_name: file_info.path.clone(),
    progress: 100.0,
    speed: 0.0,
    downloaded_bytes: downloaded_size + downloaded,
    total_bytes: total_size,
    total_files,
    elapsed_time: start_time.elapsed().as_secs_f64(),
    current_file_index,
  };
  if let Err(e) = window.emit("download_progress", &final_progress_payload) {
    println!("Failed to emit final download_progress event: {}", e);
  }

  println!("File download completed: {}", file_info.path);

  Ok(downloaded)
}

/// Concurrent version of update_file that uses atomic counters for shared progress tracking
async fn update_file_concurrent(
  _app_handle: tauri::AppHandle,
  window: tauri::Window,
  client: &reqwest::Client,
  file_info: FileInfo,
  total_files: usize,
  current_file_index: usize,
  total_size: u64,
  downloaded_bytes_total: &AtomicU64,
  completed_files: &AtomicUsize,
) -> Result<u64, String> {
  let game_path = get_game_path()?;
  let file_path = game_path.join(&file_info.path);

  if let Some(parent) = file_path.parent() {
    tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
  }

  let start_time = Instant::now();

  let response = client.get(&file_info.url)
    .send()
    .await
    .map_err(|e| format!("Request failed: {}", e))?;
  
  let content_length = response.content_length().unwrap_or(file_info.size);
  
  // Stream the response with incremental hashing for real-time progress
  let mut stream = response.bytes_stream();
  let mut downloaded_bytes: u64 = 0;
  let mut all_bytes = Vec::with_capacity(content_length as usize);
  let mut last_progress_update = Instant::now();
  let mut hasher = Sha256::new();
  
  use futures_util::StreamExt;
  
  while let Some(chunk_result) = stream.next().await {
    let chunk = chunk_result.map_err(|e| format!("Failed to read chunk: {}", e))?;
    let chunk_len = chunk.len() as u64;
    downloaded_bytes += chunk_len;
    hasher.update(&chunk);
    all_bytes.extend_from_slice(&chunk);
    
    // Update global progress atomically
    downloaded_bytes_total.fetch_add(chunk_len, Ordering::SeqCst);
    
    // Emit progress every 100ms
    if last_progress_update.elapsed().as_millis() >= 100 {
      let elapsed = start_time.elapsed();
      let elapsed_secs = elapsed.as_secs_f64();
      let global_downloaded = downloaded_bytes_total.load(Ordering::SeqCst);
      let speed = if elapsed_secs > 0.0 { (downloaded_bytes as f64 / elapsed_secs) as u64 } else { 0 };
      let file_progress = if content_length > 0 { (downloaded_bytes as f64 / content_length as f64) * 100.0 } else { 0.0 };
      
      let progress_payload = ProgressPayload {
        file_name: file_info.path.clone(),
        progress: file_progress,
        speed: speed as f64,
        downloaded_bytes: global_downloaded,
        total_bytes: total_size,
        total_files,
        elapsed_time: elapsed_secs,
        current_file_index,
      };
      let _ = window.emit("download_progress", &progress_payload);
      last_progress_update = Instant::now();
    }
  }
  
  let downloaded = all_bytes.len() as u64;

  // Finalize hash
  let downloaded_hash = format!("{:x}", hasher.finalize());

  if downloaded_hash.to_lowercase() != file_info.hash.to_lowercase() {
    return Err(format!("Hash mismatch for file: {} (expected: {}, got: {})", file_info.path, file_info.hash, downloaded_hash));
  }

  tokio::fs::write(&file_path, &all_bytes).await.map_err(|e| e.to_string())?;

  let completed = completed_files.fetch_add(1, Ordering::SeqCst) + 1;
  let elapsed = start_time.elapsed();
  let elapsed_secs = elapsed.as_secs_f64();
  let speed = if elapsed_secs > 0.0 { (downloaded as f64 / elapsed_secs) as u64 } else { downloaded };

  println!("Downloaded {} ({} bytes) in {:.2}s at {}/s [{}/{}]", 
    file_info.path, downloaded, elapsed_secs, format_bytes(speed), completed, total_files);

  // Emit completion progress
  let global_downloaded = downloaded_bytes_total.load(Ordering::SeqCst);
  let final_payload = ProgressPayload {
    file_name: file_info.path.clone(),
    progress: 100.0,
    speed: speed as f64,
    downloaded_bytes: global_downloaded,
    total_bytes: total_size,
    total_files,
    elapsed_time: elapsed_secs,
    current_file_index,
  };
  let _ = window.emit("download_progress", &final_payload);

  Ok(downloaded)
}

fn format_bytes(bytes: u64) -> String {
  const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
  let mut size = bytes as f64;
  let mut unit_index = 0;

  while size >= 1024.0 && unit_index < UNITS.len() - 1 {
    size /= 1024.0;
    unit_index += 1;
  }

  format!("{:.2} {}", size, UNITS[unit_index])
}

#[tauri::command]
async fn download_all_files(
  app_handle: tauri::AppHandle,
  window: tauri::Window,
  files_to_update: Vec<FileInfo>
) -> Result<Vec<u64>, String> {
  let total_files = files_to_update.len();
  let total_size: u64 = files_to_update.iter().map(|f| f.size).sum();
  
  debug_log(&format!("=== download_all_files CALLED with {} files, {} total bytes ===", total_files, total_size));
  println!("download_all_files: {} files, {} bytes", total_files, total_size);

  if total_files == 0 {
    println!("No files to download");
    if let Err(e) = window.emit("download_complete", ()) {
      eprintln!("Failed to emit download_complete event: {}", e);
    }
    return Ok(vec![]);
  }

  // Create a shared HTTP client for connection reuse
  let client = Arc::new(reqwest::Client::new());
  
  // Shared state for tracking progress across concurrent downloads
  let downloaded_bytes_total = Arc::new(AtomicU64::new(0));
  let completed_files = Arc::new(AtomicUsize::new(0));
  
  // Use a semaphore to limit concurrent downloads (6 at a time)
  let semaphore = Arc::new(tokio::sync::Semaphore::new(6));
  
  // Spawn all downloads concurrently
  let mut handles = Vec::with_capacity(total_files);
  
  for (index, file_info) in files_to_update.into_iter().enumerate() {
    let client = Arc::clone(&client);
    let window = window.clone();
    let app_handle = app_handle.clone();
    let semaphore = Arc::clone(&semaphore);
    let downloaded_bytes_total = Arc::clone(&downloaded_bytes_total);
    let completed_files = Arc::clone(&completed_files);
    
    let handle = tokio::spawn(async move {
      // Acquire semaphore permit (limits concurrent downloads)
      let _permit = semaphore.acquire().await.map_err(|e| e.to_string())?;
      
      let result = update_file_concurrent(
        app_handle,
        window,
        &client,
        file_info,
        total_files,
        index + 1,
        total_size,
        &downloaded_bytes_total,
        &completed_files,
      ).await;
      
      result
    });
    
    handles.push(handle);
  }
  
  // Wait for all downloads to complete
  let mut downloaded_sizes = Vec::with_capacity(total_files);
  debug_log(&format!("Waiting for {} download tasks to complete...", handles.len()));
  
  for (index, handle) in handles.into_iter().enumerate() {
    debug_log(&format!("Awaiting task {} of {}", index + 1, total_files));
    match handle.await {
      Ok(Ok(size)) => {
        debug_log(&format!("Task {} completed successfully ({} bytes)", index + 1, size));
        downloaded_sizes.push(size);
      },
      Ok(Err(e)) => {
        debug_log(&format!("Task {} failed: {}", index + 1, e));
        return Err(e);
      },
      Err(e) => {
        debug_log(&format!("Task {} panicked: {}", index + 1, e));
        return Err(format!("Download task panicked: {}", e));
      },
    }
  }

  debug_log("All download tasks completed!");
  println!("Download complete for {} file(s)", total_files);
  if let Err(e) = window.emit("download_complete", ()) {
    eprintln!("Failed to emit download_complete event: {}", e);
  }

  Ok(downloaded_sizes)
}


#[tauri::command]
async fn get_files_to_update(window: tauri::Window) -> Result<Vec<FileInfo>, String> {
  debug_log("=== Starting get_files_to_update ===");
  println!("Starting get_files_to_update");

  let start_time = Instant::now();
  let server_hash_file = get_server_hash_file().await?;

  let local_game_path = get_game_path()?;
  debug_log(&format!("Local game path: {:?}", local_game_path));
  println!("Local game path: {:?}", local_game_path);

  debug_log("Attempting to read server hash file");
  println!("Attempting to read server hash file");
  let files = server_hash_file["files"].as_array().ok_or("Invalid server hash file format")?;
  let directories = server_hash_file["directories"].as_array();
  
  debug_log(&format!("Server hash file parsed, {} files found", files.len()));
  println!("Server hash file parsed, {} files found", files.len());
  
  // Build a map of files by directory for quick lookup
  let mut files_by_dir: std::collections::HashMap<String, Vec<&serde_json::Value>> = std::collections::HashMap::new();
  for file_info in files {
    let path = file_info["path"].as_str().unwrap_or("");
    let dir_path = if let Some(pos) = path.rfind('/') {
      path[..pos].to_string()
    } else {
      "S1Game".to_string()
    };
    files_by_dir.entry(dir_path).or_insert_with(Vec::new).push(file_info);
  }

  // Check which directories need full file-by-file comparison
  let dirs_needing_check: Arc<std::sync::Mutex<std::collections::HashSet<String>>> = Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
  let verified_dirs: Arc<std::sync::Mutex<std::collections::HashSet<String>>> = Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
  let verified_count = Arc::new(AtomicUsize::new(0));
  
  if let Some(dir_array) = directories {
    println!("Checking {} directories (size verification)", dir_array.len());
    
    let dir_check_start = Instant::now();
    let total_dirs = dir_array.len();
    let processed_dirs = Arc::new(AtomicUsize::new(0));
    
    // Prepare directory info for parallel processing
    let dir_infos: Vec<_> = dir_array.iter().map(|dir_info| {
      let dir_path = dir_info["path"].as_str().unwrap_or("").to_string();
      let file_count = dir_info["file_count"].as_u64().unwrap_or(0) as usize;
      (dir_path, file_count)
    }).collect();
    
    // Process directories in parallel - verify all files exist with correct sizes
    dir_infos.par_iter().for_each(|(dir_path, file_count)| {
      let current = processed_dirs.fetch_add(1, Ordering::SeqCst) + 1;
      if current % 100 == 0 || current == total_dirs {
        println!("  Checking directory {}/{}", current, total_dirs);
      }
      
      // QUICK CHECK: Verify all files exist with correct sizes (no hashing!)
      // This is fast because it only reads file metadata, not file contents
      if let Some(dir_files) = files_by_dir.get(dir_path) {
        let mut all_exist_with_correct_size = true;
        
        for file_info in dir_files {
          let path = file_info["path"].as_str().unwrap_or("");
          let size = file_info["size"].as_u64().unwrap_or(0);
          let local_file_path = local_game_path.join(path);
          
          if !local_file_path.exists() {
            all_exist_with_correct_size = false;
            break;
          }
          
          if let Ok(metadata) = fs::metadata(&local_file_path) {
            if metadata.len() != size {
              all_exist_with_correct_size = false;
              break;
            }
          } else {
            all_exist_with_correct_size = false;
            break;
          }
        }
        
        if all_exist_with_correct_size && dir_files.len() == *file_count {
          // All files exist with correct sizes - directory is verified
          verified_dirs.lock().unwrap().insert(dir_path.clone());
          verified_count.fetch_add(1, Ordering::SeqCst);
        } else {
          // Files missing or size mismatch - need to download
          dirs_needing_check.lock().unwrap().insert(dir_path.clone());
        }
      }
    });
    
    let verified = verified_count.load(Ordering::SeqCst);
    let needs_check = dirs_needing_check.lock().unwrap().len();
    
    debug_log(&format!("Directory check completed in {:?}", dir_check_start.elapsed()));
    debug_log(&format!("  Verified: {}", verified));
    debug_log(&format!("  Directories needing update: {}", needs_check));
    
    // Debug: Check if S1Data is in the list
    let needs_check_list = dirs_needing_check.lock().unwrap();
    if needs_check_list.iter().any(|d| d.contains("S1Data")) {
      debug_log("DEBUG: S1Data is in dirs_needing_check - will check files");
    } else {
      debug_log("DEBUG: S1Data NOT in dirs_needing_check - files will be skipped!");
    }
    drop(needs_check_list);
    
    println!("Directory check completed in {:?}", dir_check_start.elapsed());
    println!("  Verified: {}", verified);
    println!("  Directories needing update: {}", needs_check);
  } else {
    // No directory hashes in server file, check all directories
    println!("No directory hashes found, will check all files");
    let mut needs_check = dirs_needing_check.lock().unwrap();
    for dir_path in files_by_dir.keys() {
      needs_check.insert(dir_path.clone());
    }
  }
  
  let dirs_needing_check = dirs_needing_check.lock().unwrap().clone();

  // Now only check files in directories that need checking
  let files_to_check: Vec<&serde_json::Value> = files.iter()
    .filter(|f| {
      let path = f["path"].as_str().unwrap_or("");
      let dir_path = if let Some(pos) = path.rfind('/') {
        &path[..pos]
      } else {
        "S1Game"
      };
      dirs_needing_check.contains(dir_path)
    })
    .collect();

  debug_log(&format!("Checking {} files in {} directories", files_to_check.len(), dirs_needing_check.len()));
  println!("Checking {} files in {} directories", files_to_check.len(), dirs_needing_check.len());

  if files_to_check.is_empty() {
    // All directories verified, nothing to update!
    println!("All directories verified, no files to update");
    let total_time = start_time.elapsed();
    let _ = window.emit("file_check_completed", json!({
      "total_files": files.len(),
      "files_to_update": 0,
      "total_size": 0,
      "total_time_seconds": total_time.as_secs(),
      "average_time_per_file_ms": 0.0
    }));
    return Ok(vec![]);
  }

  println!("Starting file comparison for {} files", files_to_check.len());
  let _cache = load_cache_from_disk().unwrap_or_else(|_| HashMap::new());
  let cache = Arc::new(RwLock::new(_cache));

  let progress_bar = ProgressBar::new(files_to_check.len() as u64);
  progress_bar.set_style(ProgressStyle::default_bar()
    .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
    .unwrap()
    .progress_chars("##-"));

  let processed_count = Arc::new(AtomicUsize::new(0));
  let files_to_update_count = Arc::new(AtomicUsize::new(0));
  let total_size = Arc::new(AtomicU64::new(0));
  let total_files_to_check = files_to_check.len();

  let files_to_update: Vec<FileInfo> = files_to_check.par_iter().enumerate()
    .filter_map(|(_index, file_info)| {
      let path = file_info["path"].as_str().unwrap_or("");
      let server_hash = file_info["hash"].as_str().unwrap_or("");
      let size = file_info["size"].as_u64().unwrap_or(0);
      let url = file_info["url"].as_str().unwrap_or("").to_string();

      // Debug DataCenter specifically
      if path.contains("DataCenter_Final_EUR") {
        debug_log("DEBUG: Checking DataCenter_Final_EUR.dat");
        debug_log(&format!("  Server hash: {}", server_hash));
        debug_log(&format!("  Server size: {}", size));
      }

      let local_file_path = local_game_path.join(path);

      let current_count = processed_count.fetch_add(1, Ordering::SeqCst) + 1;
      if current_count % 100 == 0 || current_count == total_files_to_check {
        let progress_payload = FileCheckProgress {
          current_file: path.to_string(),
          progress: (current_count as f64 / total_files_to_check as f64) * 100.0,
          current_count,
          total_files: total_files_to_check,
          elapsed_time: start_time.elapsed().as_secs_f64(),
          files_to_update: files_to_update_count.load(Ordering::SeqCst),
        };

        let _ = window.emit("file_check_progress", progress_payload)
          .map_err(|e| {
            println!("Error emitting file_check_progress event: {}", e);
            e.to_string()
          });
      }

      progress_bar.inc(1);

      if !local_file_path.exists() {
        files_to_update_count.fetch_add(1, Ordering::SeqCst);
        total_size.fetch_add(size, Ordering::SeqCst);
        return Some(FileInfo {
          path: path.to_string(),
          hash: server_hash.to_string(),
          size,
          url,
        });
      }

      let metadata = match fs::metadata(&local_file_path) {
        Ok(m) => m,
        Err(_) => {
          files_to_update_count.fetch_add(1, Ordering::SeqCst);
          total_size.fetch_add(size, Ordering::SeqCst);
          return Some(FileInfo {
            path: path.to_string(),
            hash: server_hash.to_string(),
            size,
            url,
          });
        }
      };

      let last_modified = metadata.modified().ok();

      let cache_read = cache.read().unwrap();
      if let Some(cached_info) = cache_read.get(path) {
        if let Some(lm) = last_modified {
          if cached_info.last_modified == lm && cached_info.hash.to_lowercase() == server_hash.to_lowercase() {
            return None;
          }
        }
      }
      drop(cache_read);

      if metadata.len() != size {
        files_to_update_count.fetch_add(1, Ordering::SeqCst);
        total_size.fetch_add(size, Ordering::SeqCst);
        return Some(FileInfo {
          path: path.to_string(),
          hash: server_hash.to_string(),
          size,
          url,
        });
      }

      let local_hash = match calculate_file_hash(&local_file_path) {
        Ok(hash) => hash,
        Err(_) => {
          files_to_update_count.fetch_add(1, Ordering::SeqCst);
          total_size.fetch_add(size, Ordering::SeqCst);
          return Some(FileInfo {
            path: path.to_string(),
            hash: server_hash.to_string(),
            size,
            url,
          });
        }
      };

      let mut cache_write = cache.write().unwrap();
      cache_write.insert(path.to_string(), CachedFileInfo {
        hash: local_hash.clone(),
        last_modified: last_modified.unwrap_or_else(SystemTime::now),
      });
      drop(cache_write);

      // Debug DataCenter hash comparison
      if path.contains("DataCenter_Final_EUR") {
        debug_log("DEBUG: DataCenter hash comparison:");
        debug_log(&format!("  Local hash:  {}", local_hash.to_lowercase()));
        debug_log(&format!("  Server hash: {}", server_hash.to_lowercase()));
        debug_log(&format!("  Match: {}", local_hash.to_lowercase() == server_hash.to_lowercase()));
      }

      if local_hash.to_lowercase() != server_hash.to_lowercase() {
        files_to_update_count.fetch_add(1, Ordering::SeqCst);
        total_size.fetch_add(size, Ordering::SeqCst);
        Some(FileInfo {
          path: path.to_string(),
          hash: server_hash.to_string(),
          size,
          url,
        })
      } else {
        None
      }
    })
    .collect();

  progress_bar.finish_with_message("File comparison completed");

  // Save the updated cache to disk
  let final_cache = cache.read().unwrap();
  if let Err(e) = save_cache_to_disk(&*final_cache) {
    eprintln!("Failed to save cache to disk: {}", e);
  }

  let total_time = start_time.elapsed();
  println!("File comparison completed. Files to update: {}", files_to_update.len());

  // Emit a final event with complete statistics
  let _ = window.emit("file_check_completed", json!({
    "total_files": files.len(),
    "files_to_update": files_to_update.len(),
    "total_size": total_size.load(Ordering::SeqCst),
    "total_time_seconds": total_time.as_secs(),
    "average_time_per_file_ms": (total_time.as_millis() as f64) / (files.len() as f64)
  }));

  Ok(files_to_update)
}


#[tauri::command]
async fn get_game_status(state: tauri::State<'_, GameState>) -> Result<bool, String> {
  let status = state.status_receiver.lock().await.borrow().clone();
  let is_launching = *state.is_launching.lock().await;
  Ok(status || is_launching)
}

#[tauri::command]
async fn handle_launch_game(
  app_handle: tauri::AppHandle,
  state: tauri::State<'_, GameState>
) -> Result<String, String> {
  println!("Total time: {:?}", 3);
  let mut is_launching = state.is_launching.lock().await;
  if *is_launching {
    return Err("Game is already launching".to_string());
  }
  *is_launching = true;

  let is_running = *state.status_receiver.lock().await.borrow();

  if is_running {
    *is_launching = false;
    return Err("Game is already running".to_string());
  }

  // --- Call GetAuthKeyAction to get a FRESH auth key right before launching ---
  // This is critical: the working launcher does this right before game connection
  let base_url = &*LAUNCHER_BASE_URL;
  let auth_key_url = format!("{}/launcher/GetAuthKeyAction", base_url);
  
  // Use the authenticated client that has the session cookie
  let client_guard = AUTHENTICATED_CLIENT.lock().await;
  let client = match client_guard.as_ref() {
    Some(c) => c,
    None => {
      drop(client_guard);
      *is_launching = false;
      return Err("Not authenticated. Please login first.".to_string());
    }
  };
  
  info!("Calling GetAuthKeyAction before game launch...");
  let fresh_auth_key = match client.get(&auth_key_url).send().await {
    Ok(response) => {
      match response.json::<AuthKeyResponse>().await {
        Ok(auth_key_resp) => {
          info!("Got fresh auth key before game launch: {}", auth_key_resp.auth_key);
          auth_key_resp.auth_key
        }
        Err(e) => {
          drop(client_guard);
          *is_launching = false;
          return Err(format!("Failed to parse GetAuthKeyAction response: {}", e));
        }
      }
    }
    Err(e) => {
      drop(client_guard);
      *is_launching = false;
      return Err(format!("Failed to call GetAuthKeyAction: {}", e));
    }
  };
  
  drop(client_guard); // Release the client lock

  // Update GLOBAL_AUTH_INFO with the fresh auth key
  {
    let mut auth_info = GLOBAL_AUTH_INFO.write().unwrap();
    auth_info.auth_key = fresh_auth_key.clone();
  }

  let auth_info = GLOBAL_AUTH_INFO.read().unwrap();
  // CRITICAL: Game expects user_no (numeric ID) as account_name, NOT the string username
  let account_name = auth_info.user_no.to_string();
  let characters_count = auth_info.character_count.clone();
  let ticket = fresh_auth_key; // Use the fresh key we just got
  
  info!("Launching game with account_name (user_no): '{}', ticket: '{}'", account_name, ticket);
  let (game_path, game_lang) = load_config()?;


  let acts_map_clone: HashMap<String, String> = {
      let acts_map_guard = GLOBAL_ACTS_MAP.read().unwrap();
      acts_map_guard.clone()
  };
  let pages_map_clone: HashMap<String, String> = {
      let pages_map_guard = GLOBAL_PAGES_MAP.read().unwrap();
      pages_map_guard.clone()
  };
  info!("Sending actsMap andpagesMap to teralib...");

  let full_game_path = game_path.join("Binaries").join("Tera.exe");

  if !full_game_path.exists() {
    *is_launching = false;
    return Err(format!("Game executable not found at: {:?}", full_game_path));
  }

  let full_game_path_str = full_game_path
    .to_str()
    .ok_or("Invalid path to game executable")?
    .to_string();

  let app_handle_clone = app_handle.clone();
  let is_launching_clone = Arc::clone(&state.is_launching);

  tokio::task::spawn(async move {
    // Emit the game_status_changed event at the start of the launch
    if let Err(e) = app_handle_clone.emit_all("game_status_changed", true) {
      error!("Failed to emit game_status_changed event: {:?}", e);
    }

    info!("run_game reached");
    info!("Account Name being passed to run_game: '{}'", account_name);
    info!("Characters Count: '{}'", characters_count);
    info!("Ticket: '{}'", ticket);
    match
      run_game(
        &account_name,
        &characters_count,
        &ticket,
        &game_lang,
        &full_game_path_str,
        acts_map_clone,
        pages_map_clone,
      ).await
    {
      Ok(exit_status) => {
        let result = format!("Game exited with status: {:?}", exit_status);
        app_handle_clone.emit_all("game_status", &result).unwrap();
        info!("{}", result);
      }
      Err(e) => {
        let error = format!("Error launching game: {:?}", e);
        app_handle_clone.emit_all("game_status", &error).unwrap();
        error!("{}", error);
      }
    }

    info!("Emitting game_ended event");
    if let Err(e) = app_handle_clone.emit_all("game_ended", ()) {
      error!("Failed to emit game_ended event: {:?}", e);
    }

    let mut is_launching = is_launching_clone.lock().await;
    *is_launching = false;
    if let Err(e) = app_handle_clone.emit_all("game_status_changed", false) {
      error!("Failed to emit game_status_changed event: {:?}", e);
    }

    reset_global_state();

    info!("Game launch state reset");
  });

  Ok("Game launch initiated".to_string())
}


#[tauri::command]
fn get_language_from_config() -> Result<String, String> {
  info!("Attempting to read language from config file");
  let (_, game_lang) = load_config()?;
  info!("Language read from config: {}", game_lang);
  Ok(game_lang)
}

#[tauri::command]
fn get_signup_url() -> Result<String, String> {
  let base_url = &*LAUNCHER_BASE_URL;
  Ok(format!("{}/launcher/SignupForm", base_url))
}

#[tauri::command]
async fn launch_toolbox() -> Result<String, String> {
  info!("Launching TeraToolbox");
  
  // Get the game path from config
  let (game_path, _) = load_config()?;
  let toolbox_path = Path::new(&game_path)
    .join("Binaries")
    .join("Toolbox")
    .join("TeraToolbox.exe");
  
  if !toolbox_path.exists() {
    return Err(format!("TeraToolbox not found at: {}", toolbox_path.display()));
  }
  
  // Use PowerShell Start-Process with -Verb RunAs for elevation
  let toolbox_path_str = toolbox_path.to_string_lossy().to_string();
  let working_dir = toolbox_path.parent().unwrap().to_string_lossy().to_string();
  
  match std::process::Command::new("powershell")
    .args(&[
      "-WindowStyle", "Hidden",
      "-Command",
      &format!("Start-Process -FilePath '{}' -WorkingDirectory '{}' -Verb RunAs", toolbox_path_str, working_dir)
    ])
    .spawn() {
      Ok(_) => {
        info!("TeraToolbox launched successfully");
        Ok("TeraToolbox launched".to_string())
      },
      Err(e) => {
        error!("Failed to launch TeraToolbox: {}", e);
        Err(format!("Failed to launch TeraToolbox: {}", e))
      }
    }
}

/// Checks if TeraToolbox is installed.
#[tauri::command]
async fn is_toolbox_installed() -> Result<bool, String> {
  info!("Checking if TeraToolbox is installed");
  
  let (game_path, _) = load_config()?;
  let toolbox_path = Path::new(&game_path)
    .join("Binaries")
    .join("Toolbox")
    .join("TeraToolbox.exe");
  
  Ok(toolbox_path.exists())
}

/// Downloads and installs TeraToolbox.
#[tauri::command]
async fn install_toolbox(window: tauri::Window) -> Result<String, String> {
  use futures_util::StreamExt;
  use zip::ZipArchive;
  
  info!("Installing TeraToolbox");
  
  let (game_path, _) = load_config()?;
  let binaries_path = Path::new(&game_path).join("Binaries");
  let toolbox_path = binaries_path.join("Toolbox");
  
  // Create Binaries directory if it doesn't exist
  std::fs::create_dir_all(&binaries_path)
    .map_err(|e| format!("Failed to create Binaries directory: {}", e))?;
  
  let zip_path = binaries_path.join("Toolbox.zip");
  
  // Toolbox.zip is located in /Neolithic on R2
  let file_server_url = get_files_server_url();
  // Replace TeraDirect with Neolithic for Toolbox path
  let base_url = file_server_url.replace("/TeraDirect", "/Neolithic");
  
  // Add timestamp to bypass CDN/R2 cache
  let timestamp = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_secs();
  let zip_url = format!("{}/Toolbox.zip?t={}", base_url, timestamp);
  
  info!("Downloading Toolbox from: {}", zip_url);
  
  // Download with progress - disable caching to ensure fresh download
  let client = reqwest::Client::new();
  let response = client.get(zip_url)
    .header("Cache-Control", "no-cache, no-store, must-revalidate")
    .header("Pragma", "no-cache")
    .header("Expires", "0")
    .send()
    .await
    .map_err(|e| format!("Failed to download Toolbox: {}", e))?;
  
  let total_size = response.content_length().unwrap_or(0);
  let mut downloaded: u64 = 0;
  let mut file = std::fs::File::create(&zip_path)
    .map_err(|e| format!("Failed to create ZIP file: {}", e))?;
  
  let mut stream = response.bytes_stream();
  
  while let Some(chunk) = stream.next().await {
    let chunk = chunk.map_err(|e| format!("Failed to read chunk: {}", e))?;
    file.write_all(&chunk)
      .map_err(|e| format!("Failed to write to file: {}", e))?;
    
    downloaded += chunk.len() as u64;
    
    let progress = if total_size > 0 {
      ((downloaded as f64 / total_size as f64) * 80.0) as u32 // 80% for download
    } else {
      0
    };
    
    // Emit progress event
    let _ = window.emit("toolbox_install_progress", serde_json::json!({
      "progress": progress,
      "downloaded_bytes": downloaded,
      "total_bytes": total_size,
      "status": "downloading"
    }));
  }
  
  drop(file); // Close the file before extraction
  
  info!("Download complete, extracting...");
  
  // Emit extracting status
  let _ = window.emit("toolbox_install_progress", serde_json::json!({
    "progress": 80,
    "status": "extracting"
  }));
  
  // Extract the ZIP file
  let zip_file = std::fs::File::open(&zip_path)
    .map_err(|e| format!("Failed to open ZIP file: {}", e))?;
  
  let mut archive = ZipArchive::new(zip_file)
    .map_err(|e| format!("Failed to read ZIP archive: {}", e))?;
  
  // Remove existing Toolbox directory if it exists
  if toolbox_path.exists() {
    std::fs::remove_dir_all(&toolbox_path)
      .map_err(|e| format!("Failed to remove existing Toolbox directory: {}", e))?;
  }
  
  // Create Toolbox directory
  std::fs::create_dir_all(&toolbox_path)
    .map_err(|e| format!("Failed to create Toolbox directory: {}", e))?;
  
  // Get total file count for progress calculation
  let total_files = archive.len();
  
  // Extract all files, stripping the top-level folder (e.g., neolithic-TB-main/)
  for i in 0..total_files {
    let mut file = archive.by_index(i)
      .map_err(|e| format!("Failed to read file from archive: {}", e))?;
    
    let outpath = match file.enclosed_name() {
      Some(path) => {
        // Strip the first component (top-level folder) from the path
        let components: Vec<_> = path.components().collect();
        if components.len() > 1 {
          // Skip the first component and join the rest
          let stripped_path: PathBuf = components[1..].iter().collect();
          toolbox_path.join(stripped_path)
        } else {
          // If it's just the top-level folder itself, skip it
          continue;
        }
      },
      None => continue,
    };
    
    if file.name().ends_with('/') {
      std::fs::create_dir_all(&outpath)
        .map_err(|e| format!("Failed to create directory: {}", e))?;
    } else {
      if let Some(p) = outpath.parent() {
        if !p.exists() {
          std::fs::create_dir_all(&p)
            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
        }
      }
      let mut outfile = std::fs::File::create(&outpath)
        .map_err(|e| format!("Failed to create file: {}", e))?;
      std::io::copy(&mut file, &mut outfile)
        .map_err(|e| format!("Failed to extract file: {}", e))?;
    }
    
    // Update progress during extraction (80-100%)
    let extract_progress = 80 + ((i as f64 / total_files as f64) * 20.0) as u32;
    let _ = window.emit("toolbox_install_progress", serde_json::json!({
      "progress": extract_progress,
      "status": "extracting"
    }));
  }
  
  // Clean up ZIP file
  let _ = std::fs::remove_file(&zip_path);
  
  // Emit complete status
  let _ = window.emit("toolbox_install_progress", serde_json::json!({
    "progress": 100,
    "status": "complete"
  }));
  
  info!("TeraToolbox installed successfully");
  Ok("TeraToolbox installed successfully".to_string())
}

/// Checks if TeraToolbox (Electron) is currently running.
#[tauri::command]
fn is_toolbox_running() -> Result<bool, String> {
  #[cfg(windows)]
  {
    use std::process::Command;
    
    let log_msg = |msg: &str| {
      debug_log(&format!("[is_toolbox_running] {}", msg));
    };
    
    log_msg("Starting check...");
    
    let (game_path, _) = load_config()?;
    let toolbox_path = Path::new(&game_path)
      .join("Binaries")
      .join("Toolbox");
    
    // Check if Toolbox directory exists
    if !toolbox_path.exists() {
      log_msg("Toolbox directory does not exist");
      return Ok(false);
    }
    
    log_msg(&format!("Toolbox path: {}", toolbox_path.display()));
    
    // Use a simpler PowerShell command that lists all electron processes with their paths
    let ps_cmd = "Get-Process -Name 'electron' -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Path";
    
    let output = Command::new("powershell")
      .args(&["-Command", ps_cmd])
      .creation_flags(0x08000000) // CREATE_NO_WINDOW
      .output()
      .map_err(|e| format!("Failed to check process: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    log_msg(&format!("Process check output:\n{}", stdout));
    if !stderr.is_empty() {
      log_msg(&format!("Process check error: {}", stderr));
    }
    
    // Check if any of the paths contain our Toolbox directory
    let toolbox_str = toolbox_path.to_string_lossy();
    let is_running = stdout.lines().any(|line| {
      let matches = line.contains(&*toolbox_str) || line.contains("\\Binaries\\Toolbox\\");
      if matches {
        log_msg(&format!("Found Toolbox process: {}", line));
      }
      matches
    });
    
    log_msg(&format!("Result: {}", is_running));
    Ok(is_running)
  }
  
  #[cfg(not(windows))]
  Ok(false)
}

/// Kills all running TeraToolbox (Electron) processes.
#[tauri::command]
fn kill_toolbox_process() -> Result<String, String> {
  #[cfg(windows)]
  {
    use std::process::Command;
    
    let log_msg = |msg: &str| {
      debug_log(&format!("[kill_toolbox_process] {}", msg));
    };
    
    log_msg("Starting kill process...");
    
    // Use WMIC to find electron processes with Toolbox in their executable path
    let wmic_cmd = "wmic process where \"name='electron.exe' and ExecutablePath like '%Binaries\\\\Toolbox%'\" get ProcessId /format:list";
    
    log_msg(&format!("Running WMIC command: {}", wmic_cmd));
    
    let output = Command::new("cmd")
      .args(&["/C", wmic_cmd])
      .creation_flags(0x08000000)
      .output()
      .map_err(|e| {
        log_msg(&format!("Failed to execute WMIC: {}", e));
        format!("Failed to execute WMIC: {}", e)
      })?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    log_msg(&format!("WMIC output:\n{}", stdout));
    if !stderr.is_empty() {
      log_msg(&format!("WMIC stderr:\n{}", stderr));
    }
    
    // Parse PIDs from WMIC output (format: ProcessId=12345)
    let mut pids = Vec::new();
    for line in stdout.lines() {
      let line = line.trim();
      if line.starts_with("ProcessId=") {
        if let Some(pid) = line.strip_prefix("ProcessId=") {
          let pid = pid.trim();
          if !pid.is_empty() && pid.chars().all(|c| c.is_numeric()) {
            log_msg(&format!("Found Toolbox PID: {}", pid));
            pids.push(pid.to_string());
          }
        }
      }
    }
    
    if pids.is_empty() {
      log_msg("No Toolbox processes found");
      return Ok("No processes to kill".to_string());
    }
    
    log_msg(&format!("Total PIDs to kill: {}", pids.len()));
    
    // Kill each process tree
    for pid in pids {
      log_msg(&format!("Killing process tree for PID: {}", pid));
      let kill_cmd = format!("taskkill /F /T /PID {}", pid);
      
      let result = Command::new("cmd")
        .args(&["/C", &kill_cmd])
        .creation_flags(0x08000000)
        .output();
      
      match result {
        Ok(out) => {
          let stdout = String::from_utf8_lossy(&out.stdout);
          let stderr = String::from_utf8_lossy(&out.stderr);
          log_msg(&format!("Taskkill output: {}", stdout));
          if !stderr.is_empty() {
            log_msg(&format!("Taskkill stderr: {}", stderr));
          }
        }
        Err(e) => {
          log_msg(&format!("Failed to execute taskkill: {}", e));
        }
      }
    }
    
    log_msg("Kill commands executed, waiting for processes to terminate...");
    
    // Wait for processes to fully terminate
    std::thread::sleep(std::time::Duration::from_millis(3000));
    
    log_msg("Done waiting, kill process complete");
    Ok("Toolbox processes terminated".to_string())
  }
  
  #[cfg(not(windows))]
  Ok("Not supported on this platform".to_string())
}

/// Uninstalls TeraToolbox by removing the entire Toolbox directory and all contents.
#[tauri::command]
async fn uninstall_toolbox() -> Result<String, String> {
  info!("Uninstalling TeraToolbox");
  
  let (game_path, _) = load_config()?;
  let toolbox_dir = Path::new(&game_path)
    .join("Binaries")
    .join("Toolbox");
  
  if !toolbox_dir.exists() {
    return Err("TeraToolbox is not installed".to_string());
  }
  
  // Try Rust's native removal first
  match std::fs::remove_dir_all(&toolbox_dir) {
    Ok(_) => {
      info!("TeraToolbox uninstalled successfully using native removal");
      return Ok("TeraToolbox uninstalled successfully".to_string());
    }
    Err(e) => {
      warn!("Native removal failed ({}), attempting PowerShell fallback", e);
      // Fallback to PowerShell for locked files
      let toolbox_path_str = toolbox_dir.to_string_lossy().to_string();
      
      match std::process::Command::new("powershell")
        .args(&[
          "-WindowStyle", "Hidden",
          "-Command",
          &format!("Remove-Item -Path '{}' -Recurse -Force -ErrorAction Stop", toolbox_path_str)
        ])
        .output() {
          Ok(output) => {
            if output.status.success() {
              info!("TeraToolbox uninstalled successfully using PowerShell");
              Ok("TeraToolbox uninstalled successfully".to_string())
            } else {
              let error_msg = String::from_utf8_lossy(&output.stderr);
              error!("Failed to uninstall TeraToolbox: {}", error_msg);
              Err(format!("Failed to remove Toolbox folder. Please ensure TeraToolbox is closed and try again.\n\nError: {}", error_msg))
            }
          },
          Err(e) => {
            error!("Failed to execute PowerShell command: {}", e);
            Err(format!("Failed to remove Toolbox folder: {}", e))
          }
        }
    }
  }
}

/// Registers a new user account via the Portal API.
/// 
/// ### Arguments
/// * `username` - The desired username (3-24 alphanumeric characters).
/// * `email` - A valid email address.
/// * `password` - The desired password (8-128 characters).
///
/// ### Returns
/// * `Ok(String)` - JSON containing registration result.
/// * `Err(String)` - A descriptive error message in case of failure.
#[tauri::command]
async fn register(username: String, email: String, password: String) -> Result<String, String> {
  // Build client with cookie store to maintain session for email verification
  let client = Client::builder()
    .cookie_store(true)
    .build()
    .map_err(|e| e.to_string())?;

  let base_url = &*LAUNCHER_BASE_URL;
  
  // Call SignupAction with launcher=true to bypass captcha check when captcha is disabled
  let signup_url = format!("{}/launcher/SignupAction?launcher=true", base_url);

  // Build the form payload using reqwest's form method
  let params = [
    ("login", username.as_str()),
    ("email", email.as_str()),
    ("password", password.as_str()),
  ];

  info!("Attempting to register user: {}", username);

  let signup_res = client
    .post(&signup_url)
    .form(&params)
    .send()
    .await
    .map_err(|e| format!("Registration request failed: {}", e))?;

  if !signup_res.status().is_success() {
    return Err(format!("Registration request failed with status: {}", signup_res.status()));
  }

  let response_text = signup_res
    .text()
    .await
    .map_err(|e| format!("Failed to read registration response: {}", e))?;

  info!("Registration response: {}", response_text);

  // Store the client for use in verification step (maintains session cookies)
  {
    let mut reg_client = REGISTRATION_CLIENT.lock().await;
    *reg_client = Some(client);
  }

  Ok(response_text)
}

/// Verifies the email verification code for registration.
///
/// ### Parameters
/// * `code` - The verification code sent to the user's email.
///
/// ### Returns
/// * `Ok(String)` - JSON containing verification result.
/// * `Err(String)` - A descriptive error message in case of failure.
#[tauri::command]
async fn verify_registration(code: String) -> Result<String, String> {
  // Get the client with session cookies from registration
  let client: Client = {
    let reg_client = REGISTRATION_CLIENT.lock().await;
    reg_client.clone().ok_or("No registration session found. Please register first.")?
  };

  let base_url = &*LAUNCHER_BASE_URL;
  let verify_url = format!("{}/launcher/SignupVerifyAction?launcher=true", base_url);

  let params = [
    ("code", code.as_str()),
  ];

  info!("Attempting to verify registration with code");

  let verify_res = client
    .post(&verify_url)
    .form(&params)
    .send()
    .await
    .map_err(|e| format!("Verification request failed: {}", e))?;

  if !verify_res.status().is_success() {
    return Err(format!("Verification request failed with status: {}", verify_res.status()));
  }

  let response_text = verify_res
    .text()
    .await
    .map_err(|e| format!("Failed to read verification response: {}", e))?;

  info!("Verification response: {}", response_text);

  // Clear the registration client after successful verification
  {
    let mut reg_client = REGISTRATION_CLIENT.lock().await;
    *reg_client = None;
  }

  Ok(response_text)
}

#[tauri::command]
fn save_language_to_config(language: String) -> Result<(), String> {
  info!("Attempting to save language {} to config file", language);
  let config_path = find_config_file().ok_or("Config file not found")?;
  let mut conf = Ini::load_from_file(&config_path).map_err(|e|
    format!("Failed to load config: {}", e)
  )?;

  conf.with_section(Some("game")).set("lang", &language);

  conf.write_to_file(&config_path).map_err(|e| format!("Failed to write config: {}", e))?;

  info!("Language successfully saved to config");
  Ok(())
}

#[tauri::command]
async fn reset_launch_state(state: tauri::State<'_, GameState>) -> Result<(), String> {
  let mut is_launching = state.is_launching.lock().await;
  *is_launching = false;
  Ok(())
}

#[tauri::command]
async fn set_auth_info( 
  auth_key: String, 
  user_name: String, 
  user_no: i32, 
  character_count: String,
  session_cookie: Option<String>, 
) { 
  {
    let mut auth_info = GLOBAL_AUTH_INFO.write().unwrap();
    auth_info.auth_key = auth_key;
    auth_info.user_name = user_name;
    auth_info.user_no = user_no;
    auth_info.character_count = character_count;

    info!("Auth info set from frontend:");
    info!("User Name: {}", auth_info.user_name);
    info!("User No: {}", auth_info.user_no);
    info!("Character Count: {}", auth_info.character_count);
    info!("Auth Key: {}", auth_info.auth_key);
  }

  if let Some(cookie_value) = session_cookie {
    if !cookie_value.is_empty() {
      info!("Rebuilding authenticated client from stored cookie...");
      let base_url = &*LAUNCHER_BASE_URL;
      let url = Url::parse(base_url).expect("Failed to parse LAUNCHER_BASE_URL");
      let host = url.host_str().expect("LAUNCHER_BASE_URL has no host");

      // Build cookie
      let cookie_str = format!("launcher.sid={}; Domain={}; Path=/", cookie_value, host);
      
      let jar = Arc::new(Jar::default());
      jar.add_cookie_str(&cookie_str, &url);

      // Build new client using the cookie jar
      let client = Client::builder()
        .cookie_store(true)
        .cookie_provider(jar)
        .build()
        .expect("Failed to rebuild client");

      // Store client globally
      let mut client_guard = AUTHENTICATED_CLIENT.lock().await; // <-- 6. 'await' is now valid
      *client_guard = Some(client);
      info!("Authenticated client rebuilt successfully.");
    } else {
      info!("No session cookie found to rebuild client.");
    }
  }
}


/// Handles the complete login process for the TERA launcher.
///
/// ### Overview
/// This function:
/// 1. Authenticates the user using their credentials.
/// 2. Retrieves the session cookie and essential account details (account info, auth key, character count).
/// 3. Fetches and parses the main launcher HTML page to extract `ACTS_MAP` and `PAGES_MAP`.
/// 4. Stores these maps globally for future use.
/// 5. Returns a structured JSON response with all relevant login and session data.
///
/// The function communicates with the launcherâ€™s backend endpoints, maintains cookies
/// across requests, and reconstructs necessary URLs dynamically using `LAUNCHER_BASE_URL`.
///
/// ### Arguments
/// * `username` - The user's login name.
/// * `password` - The user's password.
///
/// ### Returns
/// * `Ok(String)` - JSON containing authentication results and user data.
/// * `Err(String)` - A descriptive error message in case of failure.
#[tauri::command]
async fn login(username: String, password: String) -> Result<String, String> {
    // 1. Create an HTTP client with a persistent cookie jar
    let cookie_jar = Arc::new(Jar::default());
    let client = Client::builder()
        .cookie_store(true)
        .cookie_provider(Arc::clone(&cookie_jar))
        .build()
        .map_err(|e| e.to_string())?;

    // --- Step 1: Define Base URL ---
    // The base launcher URL is obtained once from the global constant.
    let base_url = &*LAUNCHER_BASE_URL;
    let login_url = format!("{}/launcher/LoginAction", base_url);

    // --- Step 2: POST to /launcher/LoginAction (Authentication) ---
    let payload = format!("login={}&password={}", username, password);

    let login_res = client
        .post(&login_url)
        .body(payload)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    // --- Parse the login response and extract session cookie ---
    if !login_res.status().is_success() {
        return Err(format!("Login request failed with status: {}", login_res.status()));
    }

    let login_body: InitialLoginResponse = login_res
        .json()
        .await
        .map_err(|e| format!("Failed to parse login response: {}.", e))?;

    if !login_body.return_value {
        return Err(login_body.msg);
    }

    // Parse the cookies to retrieve the session identifier (launcher.sid)
    let login_url_parsed = Url::parse(&login_url)
        .map_err(|e| format!("Failed to parse login URL: {}", e))?;
    let cookie_header_value = cookie_jar.cookies(&login_url_parsed);

    let session_cookie: Option<String> = cookie_header_value
        .and_then(|header_val| header_val.to_str().ok().map(String::from))
        .and_then(|cookie_str| {
            cookie_str.split(';').find_map(|cookie_pair| {
                let cookie_pair = cookie_pair.trim();
                if cookie_pair.starts_with("launcher.sid=") {
                    Some(cookie_pair.trim_start_matches("launcher.sid=").to_string())
                } else {
                    None
                }
            })
        });

    let success_msg = login_body.msg.clone();

    // --- Step 3: Retrieve account data using the authenticated client ---
    // These endpoints depend on the valid session cookie.
    let account_info_url = format!("{}/launcher/GetAccountInfoAction", base_url);
    let account_info: AccountInfoResponse = client
        .get(&account_info_url)
        .send()
        .await
        .map_err(|e| format!("Failed to get account info: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse account info: {}", e))?;

    let auth_key_url = format!("{}/launcher/GetAuthKeyAction", base_url);
    let auth_key: AuthKeyResponse = client
        .get(&auth_key_url)
        .send()
        .await
        .map_err(|e| format!("Failed to get auth key: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse auth key: {}", e))?;

    let char_count_url = format!("{}/launcher/GetCharacterCountAction", base_url);
    let char_count: CharCountResponse = client
        .get(&char_count_url)
        .send()
        .await
        .map_err(|e| format!("Failed to get character count: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse character count: {}", e))?;

    // --- Step 4: GET /launcher/Main to extract ActsMap/PagesMap ---
    // Important: Add locale if the server requires it to serve localized pages.
    let main_url = format!("{}/launcher/Main?locale=en", base_url);

    let main_res = client
        .get(&main_url)
        .send()
        .await
        .map_err(|e| format!("Failed to get launcher main page: {}", e))?;

    if !main_res.status().is_success() {
        return Err(format!(
            "Launcher main page request failed with status: {}",
            main_res.status()
        ));
    }

    let main_html = main_res
        .text()
        .await
        .map_err(|e| format!("Failed to read main page body: {}", e))?;

    // KEY STEP: Pass `base_url` so that `extract_maps_from_html` can rebuild the full URLs in ACTS_MAP.
    let (acts_map, pages_map) = extract_maps_from_html(&main_html, base_url)?;

    // --- Step 5: Save parsed maps into global state ---
    if let Some(map_object) = acts_map.as_object() {
        let mut acts_map_guard = GLOBAL_ACTS_MAP.write().unwrap();
        acts_map_guard.clear();
        for (key, value) in map_object {
            if let Some(url_str) = value.as_str() {
                acts_map_guard.insert(key.clone(), url_str.to_string());
            }
        }
        info!("Saved GLOBAL_ACTS_MAP with {} entries", acts_map_guard.len());
    }

    if let Some(map_object) = pages_map.as_object() {
        let mut pages_map_guard = GLOBAL_PAGES_MAP.write().unwrap();
        pages_map_guard.clear(); // Clear previous map before inserting new values
        for (key, value) in map_object {
            if let Some(url_str) = value.as_str() {
                pages_map_guard.insert(key.clone(), url_str.to_string());
            }
        }
        info!("Saved GLOBAL_PAGES_MAP with {} entries", pages_map_guard.len());
    }

    // --- Step 6: Consolidate and return the final JSON response ---
    let combined_response = CombinedLoginResponse {
        return_value: true,
        return_code: login_body.return_code,
        msg: success_msg,
        character_count: char_count.character_count,
        permission: account_info.permission,
        privilege: account_info.privilege,
        user_no: account_info.user_no,
        user_name: account_info.user_name,
        auth_key: auth_key.auth_key,
        banned: account_info.banned,

        acts_map: Some(acts_map),
        pages_map: Some(pages_map),

        session_cookie: session_cookie,
    };

    // Store the authenticated client globally for subsequent API calls
    let mut client_guard = AUTHENTICATED_CLIENT.lock().await;
    *client_guard = Some(client);

    // Serialize and return the combined response as JSON
    serde_json::to_string(&combined_response)
        .map_err(|e| format!("Failed to serialize final login response: {}", e))
}

#[tauri::command]
async fn handle_logout(state: tauri::State<'_, GameState>) -> Result<(), String> {
  let mut is_launching = state.is_launching.lock().await;
  *is_launching = false;

  // Reset global authentication information
  {
    let mut auth_info = GLOBAL_AUTH_INFO.write().unwrap();
    auth_info.auth_key = String::new();
    auth_info.user_name = String::new();
    auth_info.user_no = 0;
    auth_info.character_count = String::new();
  }

  {
    let mut pages_map = GLOBAL_PAGES_MAP.write().unwrap();
    pages_map.clear();
    info!("GLOBAL_PAGES_MAP cleared.");
  }

  {
    let mut pages_map = GLOBAL_ACTS_MAP.write().unwrap();
    pages_map.clear();
    info!("GLOBAL_ACTS_MAP cleared.");
  }

  let mut client_guard = AUTHENTICATED_CLIENT.lock().await;
  *client_guard = None;

  Ok(())
}

// Modification: We need to access LAUNCHER_BASE_URL inside this function,
// but itâ€™s not a parameter. The solution is to pass it as an argument to the function,
// and update the call in `login` accordingly.
// Move this line if itâ€™s not already at the top of the file.
// use regex::Regex;
fn extract_maps_from_html(
    html: &str,
    base_url: &str
) -> Result<(serde_json::Value, serde_json::Value), String> {
    use regex::Regex;
    
    lazy_static! {
        // Expression to extract the full block of the ACTS_MAP/PAGES_MAP variable (including the braces).
        static ref RE_ACTSMAP: Regex = 
            Regex::new(r"var ACTS_MAP\s*=\s*(\{[\s\S]*?\});").expect("Invalid actsMap regex");
            
        static ref RE_PAGESMAP: Regex = 
            Regex::new(r"var PAGES_MAP\s*=\s*(\{[\s\S]*?\});").expect("Invalid pagesMap regex");

        // KEY FIX FOR ACTS_MAP: Captures the Key (Group 1) and the PATH (Group 2)
        // Pattern looks for: Key: location.protocol + "//HOST:PORT/PATH"
        // G1 (\w+): The numeric key (e.g., 210)
        // G2 (/[^"]*?): The path starting with '/' and ending before the closing quote.
        static ref RE_ACTS_ITEM_PATH: Regex = 
            Regex::new(r#"(\w+):\s*location\.protocol\s*\+\s*"//\S+?(/[^"]*?)",?"#).expect("Invalid actsMap item path regex");


        // --- Regex for PAGES_MAP (General cleanup) ---
        
        // Quote all unquoted keys that are tokens (word or number).
        static ref RE_QUOTE_UNQUOTED_KEYS: Regex = 
            Regex::new(r#"([,\s{])(\w+)(\s*?:)"#).expect("Invalid quote unquoted keys regex");
        
        // Replace JS expressions (if any) with a valid string for PAGES_MAP.
        static ref RE_JS_PROTOCOL_PAGESMAP: Regex = 
            Regex::new(r"(location\.protocol[\s\S]+?)(\}|,)").expect("Invalid pagesMap JS value regex");
            
        // Remove trailing comma.
        static ref RE_TRAILING_COMMA: Regex = 
            Regex::new(r",\s*?\}").expect("Invalid trailing comma regex");
            
        // Whitespace normalization
        static ref RE_NORMALIZE_WHITESPACE: Regex = 
            Regex::new(r"[\r\n\t ]+").expect("Invalid normalize whitespace regex");
    }

    // 1. EXTRACT AND BUILD ACTS_MAP (Manual URL reconstruction)

    let acts_map_raw = RE_ACTSMAP.captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str())
        .ok_or("Could not find ACTS_MAP in HTML")?;
    
    eprintln!("DEBUG: ACTS_MAP Raw Content:\n{}", acts_map_raw); // <- DEBUG 1

    let mut final_acts_map = serde_json::Map::new();

    // Iterate over all (Key: Path) matches in the raw string
    for cap in RE_ACTS_ITEM_PATH.captures_iter(acts_map_raw) {
        // Group 1: Key (e.g., "210")
        // Group 2: Path (e.g., "/tera/ShopAuth?authKey=%s")
        let key = cap.get(1).unwrap().as_str().to_string();
        let path = cap.get(2).unwrap().as_str(); // This path is already just the route
        
        // Rebuild the URL: base_url + path
        let final_url = format!("{}{}", base_url, path);
        
        final_acts_map.insert(key, serde_json::Value::String(final_url));
    }
    
    let acts_map = serde_json::Value::Object(final_acts_map);
    eprintln!("DEBUG: ACTS_MAP Final JSON:\n{}", acts_map.to_string()); // <- DEBUG 2


    // 2. EXTRACT AND CLEAN PAGES_MAP (String cleanup)
    let pages_map_raw = RE_PAGESMAP.captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str())
        .ok_or("Could not find PAGES_MAP in HTML")?;
    
    let mut pages_map_str = pages_map_raw.to_string();
    
    // Apply cleanup: normalization, JS value replacement, quoting keys, trailing comma removal.
    pages_map_str = RE_NORMALIZE_WHITESPACE.replace_all(&pages_map_str, " ").to_string();
    pages_map_str = pages_map_str.trim().replace("{ ", "{").replace(" }", "}");
    
    // Replace JS expressions (Group 1) with a placeholder string (if any exist in PAGES_MAP)
    pages_map_str = RE_JS_PROTOCOL_PAGESMAP.replace_all(&pages_map_str, r#""/JS/EXPRESSION/REMOVED"$2"#).to_string();

    // Quote all unquoted keys.
    pages_map_str = RE_QUOTE_UNQUOTED_KEYS.replace_all(&pages_map_str, r#"$1"$2"$3"#).to_string();

    // Remove trailing comma.
    pages_map_str = RE_TRAILING_COMMA.replace_all(&pages_map_str, "}").to_string();

    eprintln!("DEBUG: PAGES_MAP Cleaned Content Final:\n{}", pages_map_str); // <- DEBUG 3

    // 3. PARSE PAGES_MAP
    let pages_map: serde_json::Value = serde_json::from_str(&pages_map_str)
        .map_err(|e| format!("Failed to parse pagesMap JSON: {}", e))?;

    Ok((acts_map, pages_map))
}

#[tauri::command]
async fn check_server_connection() -> Result<bool, String> {
  // Always return true - skip actual connection check
  // Update check is disabled via UPDATE_CHECK_ENABLED = false in app.js
  Ok(true)
}

#[tauri::command]
fn get_client_version() -> Result<String, String> {
  Ok(get_config_value("CLIENT_VERSION"))
}

#[tauri::command]
async fn get_fresh_account_info() -> Result<String, String> {
  let client_guard = AUTHENTICATED_CLIENT.lock().await;
  if let Some(client) = &*client_guard {
    // We have an authenticated client, fetch fresh data
    
    let base_url = &*LAUNCHER_BASE_URL;

    // --- Step 1: GET /launcher/GetAccountInfoAction ---
    let account_info_url = format!("{}/launcher/GetAccountInfoAction", base_url);
    let account_info: AccountInfoResponse = client
      .get(&account_info_url)
      .send()
      .await
      .map_err(|e| format!("(Re-check) Failed to get account info: {}", e))?
      .json()
      .await
      .map_err(|e| format!("(Re-check) Failed to parse account info: {}", e))?;

    // --- Step 2: Use existing auth key from GLOBAL_AUTH_INFO ---
    // DO NOT call GetAuthKeyAction here! That would generate a new key and 
    // invalidate the one already stored in the database from login.
    let existing_auth_key = {
      let auth_info = GLOBAL_AUTH_INFO.read().unwrap();
      auth_info.auth_key.clone()
    };
    info!("(Re-check) Using existing auth key from login");

    // --- Step 3: GET /launcher/GetCharacterCountAction ---
    let char_count_url = format!("{}/launcher/GetCharacterCountAction", base_url);
    let char_count: CharCountResponse = client
      .get(&char_count_url)
      .send()
      .await
      .map_err(|e| format!("(Re-check) Failed to get char count: {}", e))?
      .json()
      .await
      .map_err(|e| format!("(Re-check) Failed to parse char count: {}", e))?;

    // actsMap & pagesMap refresh
    info!("Refreshing ActsMap and PagesMap for existing session...");
    let main_url = format!("{}/launcher/Main", base_url); 
    
    let main_res = client
      .get(&main_url)
      .send()
      .await
      .map_err(|e| format!("(Re-check) Failed to get launcher main page: {}", e))?;

    if !main_res.status().is_success() {
      return Err(format!("(Re-check) Launcher main page request failed with status: {}", main_res.status()));
    }

    let main_html = main_res.text().await.map_err(|e| format!("(Re-check) Failed to read main page body: {}", e))?;
    let (acts_map, pages_map) = extract_maps_from_html(&main_html, base_url)?;

    if let Some(map_object) = pages_map.as_object() {
        let mut pages_map_guard = GLOBAL_PAGES_MAP.write().unwrap();
        pages_map_guard.clear();
        for (key, value) in map_object {
            if let Some(url_str) = value.as_str() {
                pages_map_guard.insert(key.clone(), url_str.to_string());
            }
        }
        info!("(Re-check) Stored GLOBAL_PAGES_MAP with {} entries", pages_map_guard.len());
    }

    if let Some(map_object) = acts_map.as_object() {
        let mut acts_map_guard = GLOBAL_ACTS_MAP.write().unwrap();
        acts_map_guard.clear();
        for (key, value) in map_object {
            if let Some(url_str) = value.as_str() {
                acts_map_guard.insert(key.clone(), url_str.to_string());
            }
        }
        info!("(Re-check) Stored GLOBAL_ACTS_MAP with {} entries", acts_map_guard.len());
    }

    // --- Step 4: Combine all the data ---
    let combined_response = CombinedLoginResponse {
      return_value: true,
      return_code: 0, // Not a login, so 0 is fine
      msg: "success".to_string(),
      character_count: char_count.character_count,
      permission: account_info.permission,
      privilege: account_info.privilege,
      user_no: account_info.user_no,
      user_name: account_info.user_name,
      auth_key: existing_auth_key, // Use existing auth key from login, not a new one!
      banned: account_info.banned,
      acts_map: None,
      pages_map: None,
      session_cookie: None, 
    };
    
    // --- Step 5: Return the fresh data to JS ---
    serde_json::to_string(&combined_response)
      .map_err(|e| format!("Failed to serialize fresh info: {}", e))

  } else {
    // No client found â€” the user is not logged in or the session was lost.
    Err("User is not authenticated (no client)".to_string())
  }
}


// ==================== LAUNCHER SELF-UPDATE COMMANDS ====================

/// Check if a launcher update is available by fetching the update manifest from the server
#[tauri::command]
async fn check_launcher_update() -> Result<LauncherUpdateInfo, String> {
  let update_url = get_launcher_update_url();
  let current_version = get_current_launcher_version();
  
  println!("Checking for launcher updates at: {}", update_url);
  println!("Current launcher version: {}", current_version);
  
  let client = Client::builder()
    .timeout(Duration::from_secs(30))
    .build()
    .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
  
  let response = client
    .get(&update_url)
    .send()
    .await
    .map_err(|e| format!("Failed to fetch update manifest: {}", e))?;
  
  if !response.status().is_success() {
    return Err(format!("Update manifest request failed with status: {}", response.status()));
  }
  
  let manifest: LauncherUpdateManifest = response
    .json()
    .await
    .map_err(|e| format!("Failed to parse update manifest: {}", e))?;
  
  println!("Latest launcher version available: {}", manifest.version);
  
  // Compare versions
  let update_available = compare_versions(&current_version, &manifest.version) == std::cmp::Ordering::Less;
  
  Ok(LauncherUpdateInfo {
    update_available,
    current_version,
    latest_version: manifest.version.clone(),
    download_url: if update_available { Some(manifest.download_url) } else { None },
    changelog: manifest.changelog,
    mandatory: manifest.mandatory.unwrap_or(false),
  })
}

/// Download the launcher update and prepare it for installation
#[tauri::command]
async fn download_launcher_update(
  window: tauri::Window,
  download_url: String,
) -> Result<String, String> {
  println!("Downloading launcher update from: {}", download_url);
  
  // Get the directory where the current executable is located
  let exe_path = std::env::current_exe().map_err(|e| format!("Failed to get executable path: {}", e))?;
  let exe_dir = exe_path.parent().ok_or("Failed to get executable directory")?;
  
  // Always use a temp filename to avoid conflicts with the running exe
  let file_name = "teralauncher_update.exe".to_string();
  
  let update_path = exe_dir.join(&file_name);
  let update_path_str = update_path.to_string_lossy().to_string();
  
  println!("Saving update to: {}", update_path_str);
  
  let client = Client::builder()
    .timeout(Duration::from_secs(300)) // 5 minute timeout for download
    .build()
    .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
  
  let response = client
    .get(&download_url)
    .send()
    .await
    .map_err(|e| format!("Failed to start download: {}", e))?;
  
  if !response.status().is_success() {
    return Err(format!("Download failed with status: {}", response.status()));
  }
  
  let total_size = response.content_length().unwrap_or(0);
  println!("Total download size: {} bytes", total_size);
  
  // Emit initial progress
  let _ = window.emit("launcher_update_progress", LauncherUpdateProgress {
    progress: 0.0,
    downloaded_bytes: 0,
    total_bytes: total_size,
    speed: 0.0,
    status: "downloading".to_string(),
  });
  
  // Stream the download with progress updates
  let mut stream = response.bytes_stream();
  let mut downloaded_bytes: u64 = 0;
  let mut all_bytes = Vec::with_capacity(total_size as usize);
  let start_time = Instant::now();
  let mut last_progress_update = Instant::now();
  
  while let Some(chunk_result) = stream.next().await {
    let chunk = chunk_result.map_err(|e| format!("Failed to read chunk: {}", e))?;
    downloaded_bytes += chunk.len() as u64;
    all_bytes.extend_from_slice(&chunk);
    
    // Emit progress every 100ms
    if last_progress_update.elapsed().as_millis() >= 100 {
      let elapsed_secs = start_time.elapsed().as_secs_f64();
      let speed = if elapsed_secs > 0.0 { downloaded_bytes as f64 / elapsed_secs } else { 0.0 };
      let progress = if total_size > 0 { (downloaded_bytes as f64 / total_size as f64) * 100.0 } else { 0.0 };
      
      let _ = window.emit("launcher_update_progress", LauncherUpdateProgress {
        progress,
        downloaded_bytes,
        total_bytes: total_size,
        speed,
        status: "downloading".to_string(),
      });
      
      last_progress_update = Instant::now();
    }
  }
  
  // Write the downloaded file
  let _ = window.emit("launcher_update_progress", LauncherUpdateProgress {
    progress: 100.0,
    downloaded_bytes,
    total_bytes: total_size,
    speed: 0.0,
    status: "writing".to_string(),
  });
  
  tokio::fs::write(&update_path, &all_bytes)
    .await
    .map_err(|e| format!("Failed to write update file: {}", e))?;
  
  println!("Launcher update downloaded successfully: {}", update_path_str);
  
  // Emit completion
  let _ = window.emit("launcher_update_progress", LauncherUpdateProgress {
    progress: 100.0,
    downloaded_bytes,
    total_bytes: total_size,
    speed: 0.0,
    status: "complete".to_string(),
  });
  
  Ok(update_path_str)
}

/// Apply the launcher update by launching the new executable and closing this one
#[tauri::command]
async fn apply_launcher_update(
  app_handle: tauri::AppHandle,
  update_path: String,
) -> Result<(), String> {
  println!("Applying launcher update from: {}", update_path);
  
  let update_file = Path::new(&update_path);
  if !update_file.exists() {
    return Err(format!("Update file not found: {}", update_path));
  }
  
  // Get current executable path
  let current_exe = std::env::current_exe().map_err(|e| format!("Failed to get current executable: {}", e))?;
  let current_exe_str = current_exe.to_string_lossy().to_string();
  let exe_dir = current_exe.parent().ok_or("Failed to get executable directory")?;
  
  // Create a PowerShell script that will run hidden:
  // 1. Wait for the current launcher to close
  // 2. Remove old backup if exists
  // 3. Rename current exe to .old
  // 4. Move new exe to original name
  // 5. Launch the new launcher
  // 6. Clean up
  let ps_script_path = exe_dir.join("update_launcher.ps1");
  let exe_name = current_exe.file_name().unwrap().to_string_lossy();
  
  let ps_content = format!(
    r#"
Start-Sleep -Seconds 2

# Wait for the launcher process to exit
while (Get-Process -Name "{}" -ErrorAction SilentlyContinue) {{
    Start-Sleep -Milliseconds 500
}}

# Apply update
$oldPath = "{}.old"
$currentPath = "{}"
$updatePath = "{}"

if (Test-Path $oldPath) {{ Remove-Item -Force $oldPath }}
if (Test-Path $currentPath) {{ Rename-Item -Path $currentPath -NewName "$currentPath.old" -Force }}
Move-Item -Path $updatePath -Destination $currentPath -Force

# Start the updated launcher
Start-Process -FilePath $currentPath

# Cleanup
Start-Sleep -Seconds 2
if (Test-Path $oldPath) {{ Remove-Item -Force $oldPath }}
Remove-Item -Force $MyInvocation.MyCommand.Path
"#,
    exe_name.trim_end_matches(".exe"),
    current_exe_str,
    current_exe_str,
    update_path,
  );
  
  std::fs::write(&ps_script_path, ps_content)
    .map_err(|e| format!("Failed to create update script: {}", e))?;
  
  // Launch PowerShell completely hidden using -WindowStyle Hidden
  std::process::Command::new("powershell")
    .args([
      "-ExecutionPolicy", "Bypass",
      "-WindowStyle", "Hidden",
      "-File", &ps_script_path.to_string_lossy(),
    ])
    .creation_flags(0x08000000) // CREATE_NO_WINDOW flag
    .spawn()
    .map_err(|e| format!("Failed to start update script: {}", e))?;
  
  // Exit the application
  println!("Exiting launcher for update...");
  app_handle.exit(0);
  
  Ok(())
}

/// Get the current launcher version
#[tauri::command]
fn get_launcher_version() -> String {
  get_current_launcher_version()
}

#[tauri::command]
fn log_debug_message(message: String) -> Result<(), String> {
  debug_log(&message);
  Ok(())
}

fn main() {

  // Clear debug log from previous session
  clear_debug_log();

  dotenv().ok();

  let (tera_logger, mut tera_log_receiver) = teralib::setup_logging();

  // Configure only the teralib logger
  log::set_boxed_logger(Box::new(tera_logger)).expect("Failed to set logger");
  log::set_max_level(LevelFilter::Info);

  // Create an asynchronous channel for logs
  let (log_sender, mut log_receiver) = mpsc::channel::<String>(100);

  // Create a Tokio runtime
  let rt = Runtime::new().expect("Failed to create Tokio runtime");

  // Spawn a task to receive logs and send them through the channel
  rt.spawn(async move {
    while let Some(log_message) = tera_log_receiver.recv().await {
      println!("Teralib: {}", log_message);
      if let Err(e) = log_sender.send(log_message).await {
        eprintln!("Failed to send log message: {}", e);
      }
    }
  });


  let game_status_receiver = get_game_status_receiver();
  let game_state = GameState {
    status_receiver: Arc::new(Mutex::new(game_status_receiver)),
    is_launching: Arc::new(Mutex::new(false)),
  };

  tauri::Builder
    ::default()
    .manage(game_state)
    .setup(|app| {
      let window = app.get_window("main").unwrap();
      let app_handle = app.handle();
      println!("Tauri setup started");

      #[cfg(debug_assertions)]
      window.open_devtools();

      // Spawn an asynchronous task to receive logs from the channel and send them to the frontend
      tauri::async_runtime::spawn(async move {
        while let Some(log_message) = log_receiver.recv().await {
          let _ = app_handle.emit_all("log_message", log_message);
        }
      });

      println!("Tauri setup completed");


      Ok(())
    })
    .invoke_handler(
      tauri::generate_handler![
        handle_launch_game,
        get_game_status,
        select_game_folder,
        get_game_path_from_config,
        save_game_path_to_config,
        reset_launch_state,
        login,
        set_auth_info,
        get_language_from_config,
        save_language_to_config,
        get_files_to_update,
        handle_logout,
        generate_hash_file,
        check_server_connection,
        check_update_required,
        download_all_files,
        get_client_version,
        get_game_path_string,
        check_maintenance_and_notify,
        get_fresh_account_info,
        get_signup_url,
        register,
        verify_registration,
        launch_toolbox,
        is_toolbox_installed,
        install_toolbox,
        uninstall_toolbox,
        is_toolbox_running,
        kill_toolbox_process,
        log_debug_message,
        // Launcher self-update commands
        check_launcher_update,
        download_launcher_update,
        apply_launcher_update,
        get_launcher_version,
      ]
    )
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

