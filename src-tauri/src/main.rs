// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use tauri::Manager;
use tauri::Window;
use tokio::io::{self, Error, ErrorKind};
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio::time::{self, Duration};

async fn construct_use_stream(
    time_out: u64,
    per: String,
    use_proxy: bool,
    socks5_url: String,
) -> io::Result<TcpStream> {
    if use_proxy {
        match time::timeout(
            Duration::from_millis(time_out),
            TcpStream::connect(socks5_url),
        )
        .await
        {
            Ok(Ok(mut s)) => {
                let Some((addr, port_str)) = per.split_once(':') else {
                    return Err(Error::new(ErrorKind::Other, "invalid peer address"));
                };
                let port = port_str
                    .parse::<u16>()
                    .map_err(|_| Error::new(ErrorKind::Other, "invalid port"))?;
                async_socks5::connect(&mut s, (addr.to_owned(), port), None)
                    .await
                    .map(|_| s)
                    .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))
            }
            Ok(Err(e)) => Err(e),
            Err(e) => Err(Error::new(ErrorKind::Other, e.to_string())),
        }
    } else {
        time::timeout(Duration::from_millis(time_out), TcpStream::connect(per))
            .await
            .unwrap_or_else(|e| Err(Error::new(ErrorKind::Other, e.to_string())))
    }
}

async fn test_connection_speed(
    pers: Vec<String>,
    time_out: u64,
    interval_time: u64,
    use_proxy: bool,
    socks5_url: String,
    window: Window,
    canceller: Arc<Mutex<UnboundedReceiver<bool>>>,
) {
    let mut join_set = JoinSet::new();
    let mut index = 0u32;
    for per in pers {
        let window = window.clone();
        let socks5_url = socks5_url.clone();
        join_set.spawn(async move {
            let mut use_time_vec: Vec<u128> = Vec::new();
            let mut total_count = 0u64;
            loop {
                total_count += 1;
                let now = time::Instant::now();
                match construct_use_stream(time_out, per.clone(), use_proxy, socks5_url.clone())
                    .await
                {
                    Ok(_) => {
                        let elapse = now.elapsed().as_millis();
                        use_time_vec.push(elapse);
                        let success_count = use_time_vec.len() as u128;
                        let total: u128 = use_time_vec.iter().sum();
                        let average_time = total / success_count;
                        let min_time = *use_time_vec.iter().min().unwrap();
                        let packet_loss = (((total_count as f64 - success_count as f64)
                            / total_count as f64
                            * 1000.0)
                            .round()
                            / 10.0);
                        let j = serde_json::json!({
                            "ip": per,
                            "index": index,
                            "success": true,
                            "msg": {
                                "latency": elapse,
                                "count": success_count,
                                "total": total_count,
                                "average": average_time,
                                "min": min_time,
                                "packet_loss": packet_loss
                            }
                        });
                        window.emit("per-result", j.to_string()).unwrap();
                    }
                    Err(e) => {
                        let success_count = use_time_vec.len() as u64;
                        let packet_loss = (((total_count - success_count) as f64
                            / total_count as f64
                            * 1000.0)
                            .round()
                            / 10.0);
                        let j = serde_json::json!({
                            "ip": per,
                            "index": index,
                            "success": false,
                            "msg": {
                                "error": e.to_string(),
                                "count": success_count,
                                "total": total_count,
                                "packet_loss": packet_loss
                            }
                        });
                        window.emit("per-result", j.to_string()).unwrap();
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(interval_time)).await;
            }
        });
        index += 1;
    }
    while let Some(_) = canceller.lock().await.recv().await {
        break;
    }
    join_set.shutdown().await;
    window.emit("complete", "").unwrap();
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_window("main").unwrap();
            let window_copy = window.clone();
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<bool>();
            let rx = Arc::new(tokio::sync::Mutex::new(rx));
            window.listen_global("cancel-all", move |_| {
                tx.send(true).unwrap();
            });
            window.listen_global("test-pers", move |event| {
                let payload = event.payload().unwrap_or("");
                if payload != "" {
                    match serde_json::from_str::<serde_json::Value>(payload) {
                        Ok(json) => {
                            let pers = json
                                .get("pers")
                                .and_then(|v| v.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .map(|item| item.as_str().unwrap_or("").to_string())
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default();
                            let time_out = json
                                .get("time_out")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(5000);
                            let use_proxy = json
                                .get("use_proxy")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            let socks5_url = json
                                .get("socks5_url")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_owned();
                            let interval_time = json
                                .get("interval")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(3000);
                            let window = window_copy.clone();
                            if pers.is_empty() {
                                window_copy.emit("reset", "空数据").unwrap();
                                return;
                            }
                            if use_proxy && socks5_url.is_empty() {
                                window_copy
                                    .emit("reset", "填写完整的socks5代理地址")
                                    .unwrap();
                                return;
                            }
                            let rx = rx.clone();
                            std::thread::spawn(move || {
                                tokio::runtime::Builder::new_multi_thread()
                                    .enable_all()
                                    .build()
                                    .unwrap()
                                    .block_on(async {
                                        test_connection_speed(
                                            pers,
                                            time_out,
                                            interval_time,
                                            use_proxy,
                                            socks5_url,
                                            window,
                                            rx,
                                        )
                                        .await;
                                    });
                            });
                        }
                        Err(e) => {
                            window_copy.emit("reset", e.to_string()).unwrap();
                        }
                    }
                } else {
                    window_copy.emit("reset", "传入无效的参数").unwrap();
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
