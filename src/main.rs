use std::env;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::thread;

mod parse;
mod db;
mod utils;

use parse::parse_table_data;
use db::{extract_property_values, insert_data, expected_property_values};
use utils::{watch_folder, move_to_failed_folder, load_env};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    load_env()?;

    // Get the folder to watch
    let watch_folder = env::var("WATCH_FOLDER")?.parse::<PathBuf>()?;

    // Get the transactional tables
    let transactional_tables: Vec<String> = env::var("TRANSACTIONAL_TABLES")?
        .split(',')
        .map(|s| s.to_string())
        .collect();

    // Create a channel for sending file paths
    let (tx, rx) = channel();

    // Start watching the folder for new files
    let _watcher = watch_folder(watch_folder, tx)?;

    // Spawn worker threads
    let mut threads = vec![];
    for _ in 0..64 {
        let rx = rx.clone();
        let transactional_tables = transactional_tables.clone();
        let thread = thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(file_path) => {
                        process_file(file_path, &transactional_tables);
                    }
                    Err(_) => break,
                }
            }
        });
        threads.push(thread);
    }

    // Wait for all threads to finish
    for thread in threads {
        thread.join().unwrap();
    }

    Ok(())
}

fn process_file(file_path: PathBuf, transactional_tables: &[String]) {
    let file = match std::fs::File::open(&file_path) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("Error opening file: {}", err);
            return;
        }
    };

    let start = std::time::Instant::now();
    let data = parse_table_data(file);
    let duration = start.elapsed();
    println!("Parsing took: {:?}", duration);

    if let Some(global_properties_data) = data.get("global_properties") {
        let property_names = &["property_name_1", "property_name_2"];
        let property_values = extract_property_values(global_properties_data, property_names);

        if property_values == expected_property_values() {
            let mut conn = match mysql::Conn::new(mysql::Opts::from_env()) {
                Ok(conn) => conn,
                Err(err) => {
                    eprintln!("Error connecting to MySQL: {}", err);
                    return;
                }
            };

            for (table_name, table_data) in data {
                if transactional_tables.contains(&table_name) {
                    // Append the site_id column and create a composite primary key
                    let mut new_table_data = Vec::new();
                    for row in table_data {
                        let mut new_row = row.clone();
                        new_row.insert(0, "site_id_value".to_string());
                        new_table_data.push(new_row);
                    }

                    if let Err(err) = insert_data(&mut conn, &table_name, &new_table_data) {
                        eprintln!("Error inserting data into {}: {}", table_name, err);
                    }
                } else {
                    if let Err(err) = insert_data(&mut conn, &table_name, &table_data) {
                        eprintln!("Error inserting data into {}: {}", table_name, err);
                    }
                }
            }
        } else {
            if let Err(err) = move_to_failed_folder(&property_values) {
                eprintln!("Error moving file to failed folder: {}", err);
            }
        }
    } else {
        eprintln!("The global_properties table was not found in the dump file.");
    }
}