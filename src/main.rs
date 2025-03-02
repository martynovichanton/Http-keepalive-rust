use std::{env, io};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, Write};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tokio::{task, signal};
use futures::future::join_all;
use reqwest::Client;
use chrono::Local;

const SLEEP_TIME: u64 = 1;
const TEMP_LOGS_DIR: &str = "templogs";
const PARAM_NAME: &str = "generator";
const LOG_FILE: &str = "log.txt";

pub async fn http_keepalive(url_pair: String, process_id: usize, client: Arc<Client>) {
    let url_pairs: Vec<&str> = url_pair.split("---").collect();
    let url: &str = url_pairs[0];
    let expected_string: &str = url_pairs[1];
    let start_idx: usize = url_pairs[3].parse().unwrap_or(0);
    let end_idx: usize = url_pairs[4].parse().unwrap_or(0);

    let log_file_path: String = format!("{}/log-{}.txt", TEMP_LOGS_DIR, process_id);
    let mut count_errors: i32 = 0;
    let mut ids_errors: Vec<String> = vec![];

    fs::create_dir_all(TEMP_LOGS_DIR).ok();

    for i in start_idx..end_idx {
        let param_value: String = format!("{}-{}", process_id, i);
        let now: String = Local::now().format("%Y-%m-%d-%H-%M-%S").to_string();

        let start_time: std::time::Instant = std::time::Instant::now();
        let updated_url: &str = &format!("{}?{}={}", url, PARAM_NAME, param_value)[..];
        match client.get(updated_url).send().await {
            Ok(response) => {
                let status: reqwest::StatusCode = response.status();
                let text: String = response.text().await.unwrap_or_else(|_| "Failed to read response".to_string());
                let elapsed_time: f32 = start_time.elapsed().as_secs_f32();

                let log_entry: String = format!(
                    "{} - {} - elapsed_time: {:.3} - \n{} {}\n",
                    now, updated_url, elapsed_time, status, text
                );

                if !text.contains(expected_string) {
                    count_errors += 1;
                    ids_errors.push(param_value.clone());
                }

                append_to_file(&log_file_path, &log_entry);
                println!("{}", log_entry);
            }
            Err(e) => {
                let log_entry: String = format!("EXCEPTION: {}\n", e);
                count_errors += 1;
                ids_errors.push(param_value.clone());
                append_to_file(&log_file_path, &log_entry);
                println!("{}", log_entry);
            }
        }
        sleep(Duration::from_secs(SLEEP_TIME)).await;
    }

    let summary: String = format!("Process ended {} {}\nErrors number: {}/{} Errors IDs: {:?}\n", url, process_id, count_errors, end_idx, ids_errors);
    append_to_file(&log_file_path, &summary);
    println!("{}", summary);
}

fn append_to_file(path: &str, content: &str) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(content.as_bytes());
    }
}

fn merge_logs() {
    let mut log_file: fs::File = OpenOptions::new().create(true).append(true).open(LOG_FILE).expect("Failed to open log file");
    if let Ok(entries) = fs::read_dir(TEMP_LOGS_DIR) {
        for entry in entries.filter_map(Result::ok) {
            if let Ok(contents) = fs::read_to_string(entry.path()) {
                let _ = log_file.write_all(contents.as_bytes());
            }
        }
        let _ = fs::remove_dir_all(TEMP_LOGS_DIR);
    }
}

#[tokio::main]
async fn main() {
    // urls file content
    // url---expected_string---num_processes---start_idx---end_idx
    
    let args: Vec<String> = env::args().collect();
    if args.len() == 2 {
        let filename: &String = &args[1];
        let file: File = File::open(filename).expect("Error opening file");
        let reader: io::BufReader<File> = io::BufReader::new(file);
        let urls: Vec<String> = reader.lines().filter_map(Result::ok).collect();

        let client: Arc<Client> = Arc::new(Client::new());

        // compact loop
        let mut tasks: Vec<_> = Vec::new();
        for url_pair in urls {
            let url_pairs: Vec<&str> = url_pair.split("---").collect();
            let num_processes: usize = url_pairs[2].parse().unwrap_or(0);
            tasks = (0..num_processes)
            .map(|i| {
                let client: Arc<Client> = Arc::clone(&client);
                task::spawn(http_keepalive(url_pair.clone(), i, client))
            })
            .collect(); 
        }

        // explicit loop
        // let mut tasks:Vec<task::JoinHandle<()>>= Vec::new();
        // for url_pair in urls {
        //     let url_pairs: Vec<&str> = url_pair.split("---").collect();
        //     let num_processes: usize = url_pairs[2].parse().unwrap_or(0);
        //     for i in 0..num_processes {
        //         let url_pair: String = url_pair.clone();
        //         let client: Arc<Client> = Arc::clone(&client);
        //         let task: task::JoinHandle<()> = tokio::spawn(async move {
        //             http_keepalive(url_pair, i, client).await;
        //         });
        //         tasks.push(task);
        //     }
        // }

        let ctrl_c = async {
            signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
            println!("\nCtrl+C detected! Merging logs before exit...");
            merge_logs();
            std::process::exit(0);
        };

        tokio::select! {
            _ = join_all(tasks) => {
                println!("All processes completed");
            }
            _ = ctrl_c => {}
        }

        merge_logs();
        println!("Loop ended");
    } else {
        println!("[*] Usage: {} [urls_file]", args[0]);
    }
}
