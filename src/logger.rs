use std::fs::{self, OpenOptions};
use std::io::Write;
use chrono::Local;

pub fn log(level: &str, message: &str) {
    let now = Local::now();
    let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let log_line = format!("{} [{}] {}\n", timestamp, level, message);

    let mut log_path = std::env::current_dir().unwrap_or_default();
    log_path.push("log");

    if !log_path.exists() {
        if let Err(e) = fs::create_dir_all(&log_path) {
            eprintln!("无法创建日志目录: {}", e);
            return;
        }
    }

    let date_str = now.format("%Y-%m-%d").to_string();
    log_path.push(format!("{}.log", date_str));

    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        Ok(mut file) => {
            let _ = file.write_all(log_line.as_bytes());
        }
        Err(e) => {
            eprintln!("打开日志文件失败: {}", e);
        }
    }

    if level == "ERROR" {
        let _ = std::io::stderr().write_all(log_line.as_bytes());
    } else {
        let _ = std::io::stdout().write_all(log_line.as_bytes());
    }
}

#[macro_export]
macro_rules! ds_info {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        crate::logger::log("INFO", &message);
    }}
}

#[macro_export]
macro_rules! ds_error {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        crate::logger::log("ERROR", &message);
    }}
}

#[macro_export]
macro_rules! ds_warn {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        crate::logger::log("WARN", &message);
    }};
}