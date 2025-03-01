# HTTP Keepalive

## HTTP Traffic Generator with Rust

1. Update urls.txt with the required URLs, expected return string, num of tasks and num of requests in each task
2. Http-keepalive-rust urls.txt
3. The loop is running X TCP connections and in each connection is running Y HTTP requests (keepalive) with a specific interval
4. Each HTTP request is appended with a parameter with the unique value of process_id and request_id to track in logs or sniffers
5. The results are saved in a log file
6. Summary of errors per subprocess on program exit


