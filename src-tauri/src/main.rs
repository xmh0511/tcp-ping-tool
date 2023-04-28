// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

use tauri::Window;
use tokio::net::TcpStream;
use tokio::time::{self, Duration};
use tokio::io::{self,Error,ErrorKind};
use tokio::task::{JoinSet};
use tokio::sync::mpsc::{UnboundedReceiver};
use tokio::sync::Mutex;
async fn construct_use_stream(time_out:u64,per:String,use_proxy:bool,socks5_url:String)-> io::Result<TcpStream>{
    if use_proxy{
		match time::timeout(Duration::from_millis(time_out), TcpStream::connect(socks5_url.clone())).await{
			Ok(r)=>{
				if let Ok(mut s) = r{
					let per = per.clone();
					let Some((addr,port)) = per.split_once(":") else{
						return Err(Error::new(ErrorKind::Other, "invalid peer address"));
					};
					let (addr, port) = (addr.to_owned(),{
						if let Ok(port) = u16::from_str_radix(port, 10){
							port
						} else{
							return Err(Error::new(ErrorKind::Other, "invalid port"));
						}
					});
					match async_socks5::connect(& mut s, (addr,port), None).await{
						Ok(_)=>{
					        return Ok(s);
						}
						Err(e)=>{
							return  Err(Error::new(ErrorKind::Other, e.to_string()));
						}
					}
				}else{
					return  Err(Error::new(ErrorKind::Other, r.err().unwrap().to_string()));
				}
			}
			Err(e)=>{
				return  Err(Error::new(ErrorKind::Other, e.to_string()));
			}
		};
	}else{
		TcpStream::connect(per.clone()).await
	}
}
async fn test_connection_speed(pers: Vec<String>, time_out:u64, interval_time:u64 ,use_proxy:bool,socks5_url:String, window: Window,canceller:Arc<Mutex<UnboundedReceiver<bool>>>) {
	let mut join_set = JoinSet::new();
	let mut index = 0u32;
    for per in pers {
        let window = window.clone();
		let socks5_url = socks5_url.clone();
        join_set.spawn(async move {
			let mut use_time_vec = Vec::new();
			//let mut count = 0u64;
			loop{
				let now = time::Instant::now();
				match time::timeout(Duration::from_millis(time_out),construct_use_stream(time_out,per.clone(),use_proxy,socks5_url.clone())).await {
					Ok(d) => {
						if let Ok(_) = d {
							//count+=1;
							let elapse = now.elapsed().as_millis();
							use_time_vec.push(elapse);
							let total:u128 = use_time_vec.iter().sum();
							let count: u128 = use_time_vec.len() as u128;
							let average_time = total / count;
							//println!("{} ms",now.elapsed().as_millis());
							//(per, now.elapsed().as_millis().to_string())
							let j = serde_json::json!({
								"ip":per,
								"index":index,
								"success":true,
								"msg":{
									"latency":elapse,
									"count":count,
									"average":average_time
								}
							});
							window.emit("per-result", j.to_string()).unwrap();
						} else {
							//println!("{d:?}");
							let j = serde_json::json!({
								"ip":per,
								"index":index,
								"success":false,
								"msg":{
									"error": format!("{}",d.err().unwrap().to_string())
								}
							});
							window.emit("per-result", j.to_string()).unwrap();
						}
					}
					Err(e) => {
						//println!("time_out: {e:?}");
						let j = serde_json::json!({
							"ip":per,
							"index":index,
							"success":false,
							"msg":{
								"error": format!("{}",e.to_string())
							}
						});
						window.emit("per-result", j.to_string()).unwrap();
					}
				}
				tokio::time::sleep(std::time::Duration::from_millis(interval_time)).await;
			}
        });
		index+=1;
    }
	tokio::spawn(async move {
		while let Some(_) = canceller.lock().await.recv().await{
			break;
		}
		join_set.shutdown().await;
		window.emit("complete", "").unwrap();
	});
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_window("main").unwrap();
            let window_copy = window.clone();
			let (tx,rx) = tokio::sync::mpsc::unbounded_channel::<bool>();
			let rx = Arc::new(tokio::sync::Mutex::new(rx));
			window.listen_global("cancel-all", move |_|{
				tx.send(true).unwrap();
			});
            window.listen_global("test-pers", move |event| {
                let payload = event.payload().unwrap_or("");
                if payload != "" {
                    //app.emit_all("speed-result", "abc");
                    //println!("payload === {payload}");
                    match serde_json::from_str::<serde_json::Value>(payload) {
                        Ok(json) => {
                            let pers = {
                                match json.get("pers") {
                                    Some(r) => {
										match r.as_array(){
											Some(arr)=>{
												arr.iter().map(|item|{
													item.as_str().unwrap_or("").to_string()
												}).collect()
											}
											None=>{
												Vec::new()
											}
										}
									},
                                    None => {
										Vec::new()
									},
                                }
                            };
							let time_out = {
								match json.get("time_out"){
									Some(time)=>{
										time.as_u64().unwrap_or(5000)
									}
									None=>{
										5000
									}
								}
							};
							let use_proxy = {
								match json.get("use_proxy"){
									Some(use_proxy)=>{
										use_proxy.as_bool().unwrap_or(false)
									}
									None=>{
										false
									}
								}
							};
							let socks5_url = {
								match json.get("socks5_url"){
									Some(socks5_url)=>{
										socks5_url.as_str().unwrap_or("").to_owned()
									}
									None=>{"".to_owned()}
								}
							};
							let interval_time = {
								match json.get("interval"){
									Some(time)=>{
										time.as_u64().unwrap_or(3000)
									}
									None=>{3000}
								}
							};
							let window = window_copy.clone();
							//println!("{pers:?},{time_out}");
							if pers.is_empty(){
								window_copy.emit("reset", "空数据").unwrap();
								return ;
							}
							//println!("use_proxy: {use_proxy},socks5_url:{}",socks5_url.is_empty());
							if use_proxy && socks5_url.is_empty(){
								//println!("invocation -----------");
								window_copy.emit("reset", "填写完整的socks5代理地址").unwrap();
								return ;
							}
							let rx = rx.clone();
							tokio::spawn(async move {
								test_connection_speed(pers,time_out,interval_time,use_proxy,socks5_url, window,rx).await;
							});
                        }
                        Err(e) => {
							window_copy.emit("reset", e.to_string()).unwrap();
						}
                    }
                }else{
					window_copy.emit("reset", "传入无效的参数").unwrap();
				}
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
