// Common system functions

use std::path::PathBuf;
use std::env;
use std::fs;
use chrono::prelude::*;

/// Return a system temp directory, based on the TEMP environment variable, adding a subfolder and timestamp
pub fn get_temp_dir(app_name: &str) -> PathBuf {
    let temp_dir = env::var("TEMP").unwrap_or_else(|_| "".to_string());
    let mut path = PathBuf::from(&temp_dir);

    let utc: DateTime<Utc> = Utc::now();
    let timestamp = utc.format("%Y-%m-%d-%H-%M-%S").to_string();

    path.push(format!("{}-{}", app_name, timestamp));

    // Creates the new directory, ignoring the error if the directory already exists
    let _ = fs::create_dir_all(&path);

    path
}