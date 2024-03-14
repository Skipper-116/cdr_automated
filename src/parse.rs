use std::collections::HashMap;
use std::io::BufReader;
use std::sync::Arc;
use std::thread;
use gz::bufread::GzDecoder;
use rayon::prelude::*;

pub fn parse_table_data<T: std::io::Read + Send + Sync + 'static>(
    mut file: T,
) -> HashMap<String, Vec<Vec<String>>> {
    let mut data: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    let mut current_table = String::new();
    let mut current_row = Vec::new();

    let buffer = Arc::new(BufReader::new(file));
    let mut handles = Vec::new();

    // Split the file into chunks and process them in parallel
    for chunk in buffer.bytes().chunks(1024 * 1024 * 100) {
        let buffer_clone = Arc::clone(&buffer);
        let handle = thread::spawn(move || {
            let gz_decoder = GzDecoder::new(&buffer_clone);
            let lines = gz_decoder.lines();

            let mut chunk_data: HashMap<String, Vec<Vec<String>>> = HashMap::new();
            let mut chunk_current_table = String::new();
            let mut chunk_current_row = Vec::new();

            for line in lines {
                let line = line.unwrap();

                if line.starts_with("CREATE TABLE `") {
                    // Start of a new table
                    if !chunk_current_row.is_empty() {
                        chunk_data
                            .entry(chunk_current_table.clone())
                            .or_insert_with(Vec::new)
                            .push(chunk_current_row.clone());
                        chunk_current_row.clear();
                    }
                    chunk_current_table = line.split('`').nth(1).unwrap().to_string();
                } else if line.starts_with(")") {
                    // End of table data
                    if !chunk_current_row.is_empty() {
                        chunk_data
                            .entry(chunk_current_table.clone())
                            .or_insert_with(Vec::new)
                            .push(chunk_current_row.clone());
                        chunk_current_row.clear();
                    }
                } else if line.starts_with("(") {
                    // New row
                    if !chunk_current_row.is_empty() {
                        chunk_data
                            .entry(chunk_current_table.clone())
                            .or_insert_with(Vec::new)
                            .push(chunk_current_row.clone());
                        chunk_current_row.clear();
                    }
                    let row_values: Vec<_> = line[1..line.len() - 1]
                        .split(',')
                        .map(|value| value.trim_matches('\'').to_string())
                        .collect();
                    chunk_current_row.extend_from_slice(&row_values);
                } else {
                    // Continuation of the previous row
                    let row_values: Vec<_> = line
                        .split(',')
                        .map(|value| value.trim_matches('\'').to_string())
                        .collect();
                    chunk_current_row.extend_from_slice(&row_values);
                }
            }

            chunk_data
        });
        handles.push(handle);
    }

    // Collect the results from the threads
    let chunk_results: Vec<_> = handles
        .into_par_iter()
        .map(|handle| handle.join().unwrap())
        .collect();

    // Merge the chunk results
    for chunk_data in chunk_results {
        for (table_name, table_data) in chunk_data {
            data.entry(table_name)
                .or_insert_with(Vec::new)
                .extend(table_data);
        }
    }

    data
}