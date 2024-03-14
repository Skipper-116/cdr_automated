use std::collections::HashMap;
use mysql::prelude::*;

pub fn extract_property_values(data: &[Vec<String>], property_names: &[&str]) -> HashMap<&str, String> {
    let mut property_values: HashMap<&str, String> = HashMap::new();

    for row in data {
        if let Some(property_name) = row.get(0) {
            if property_names.contains(&property_name.as_str()) {
                if let Some(property_value) = row.get(1) {
                    property_values.insert(property_name.as_str(), property_value.clone());
                }
            }
        }
    }

    property_values
}

pub fn insert_data(conn: &mut mysql::Conn, table_name: &str, data: &[Vec<String>]) -> Result<(), Box<dyn std::error::Error>> {
    const BATCH_SIZE: usize = 1000;

    let mut batch = Vec::with_capacity(BATCH_SIZE);
    let mut transaction = conn.start_transaction(mysql::TxOpts::default())?;

    for row in data {
        let placeholders = (0..row.len()).map(|_| "?").collect::<Vec<_>>().join(", ");
        let query = format!("INSERT INTO `{}` VALUES ({})", table_name, placeholders);

        transaction.exec_batch(query, row.iter().map(|v| v as &dyn ToValue))?;

        batch.push(transaction.save_point_vec()?);

        if batch.len() >= BATCH_SIZE {
            transaction.commit_savepoints(batch.drain(..))?;
        }
    }

    if !batch.is_empty() {
        transaction.commit_savepoints(batch)?;
    }

    transaction.commit()?;

    Ok(())
}

pub fn expected_property_values() -> HashMap<&'static str, String> {
    let mut expected_values = HashMap::new();
    expected_values.insert("property_name_1", "expected_value_1".to_string());
    expected_values.insert("property_name_2", "expected_value_2".to_string());
    // Add more expected property values here
    expected_values
}