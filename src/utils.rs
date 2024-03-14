use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use notify::{watcher, RecursiveMode, Watcher};

pub fn watch_folder(folder: PathBuf, tx: Sender<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let (mut watcher, rx) = watcher(RecursiveMode::NonRecursive).unwrap();

    watcher.watch(folder, RecursiveMode::NonRecursive)?;

    loop {
        match rx.recv() {
            Ok(event) => {
                if let Ok(path) = event.path {
                    let file_name = path.file_name().unwrap().to_string_lossy();
                    if file_name.starts_with("openmrs_") && file_name.ends_with(".sql.gz") {
                        tx.send(path).unwrap();
                    }
                }
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }
}

pub fn move_to_failed_folder(property_values: &HashMap<&str, String>) -> Result<(), Box<dyn std::error::Error>> {
    let failed_folder = "path/to/failed/folder";
    let reason_file = "path/to/reason.txt";

    let mut reason = String::new();
    for (property_name, property_value) in property_values {
        reason.push_str(&format!("{}: {}\n", property_name, property_value));
    }

    std::fs::create_dir_all(failed_folder)?;
    std::fs::rename("path/to/dump.sql.gz", format!("{}/dump.sql.gz", failed_folder))?;
    std::fs::write(reason_file, reason)?;

    Ok(())
}

pub fn load_env() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    for (key, value) in env::vars() {
        println!("{}: {}", key, value);
    }
    Ok(())
}