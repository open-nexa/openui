use futures_util::{Stream, StreamExt};
use http::HeaderValue;
use http_body_util::{BodyExt, Full};
use iroh::endpoint::{RecvStream, SendStream};
use iroh::Endpoint;
use nexapipe::EndpointTicket;
use std::str::FromStr;
use tauri::Manager;

pub mod tokiort;

type ClientBuilder = hyper::client::conn::http1::Builder;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .register_asynchronous_uri_scheme_protocol("iroh", move |app, request, responder| {
            println!("Request: {:?}", request);
            let endpoint = app.app_handle().state::<Endpoint>().inner().clone();
            let uri = format!("http://10.0.0.156:8080{}",   request.uri().path());
            let method = request.method().clone();

            tauri::async_runtime::spawn(async move {
                let ticket = match EndpointTicket::from_str(
                    "endpointaby6obscwxujzhwleuyls7jy6mzfimc6ztbz2n2re3mettdg7th3gaiaf5uhi5dqom5c6l3von3tcljrfzzgk3dbpexg4mbonfzg62bnmnqw4ylspexgs4tpnaxgy2lonmxc6",
                ) {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!("Failed to parse ticket: {}", e);
                        return;
                    }
                };

                let conn = match endpoint.connect(ticket, &nexapipe::ALPN.to_vec()).await {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Failed to connect: {}", e);
                        return;
                    }
                };

                let (send, recv) = match conn.open_bi().await {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("Failed to open bi-stream: {}", e);
                        return;
                    }
                };

                let io = tokiort::TokioIo::new(send, recv);
                let (mut sender, conn) = match ClientBuilder::new()
                    .preserve_header_case(true)
                    .title_case_headers(true)
                    .handshake(io)
                    .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Handshake failed: {}", e);
                        return;
                    }
                };

                tokio::task::spawn(async move {
                    if let Err(err) = conn.await {
                        println!("Connection failed: {:?}", err);
                    }
                });

                let body = request.body().to_vec();
                let body = hyper::body::Bytes::from(body);
                let body = Full::new(body);
                let mut req = match hyper::Request::builder()
                    .uri(&uri) 
                    .method(method)
                    .body(body)
                {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Failed to build request: {}", e);
                        return;
                    }
                };
                let  headers =req.headers_mut();
                headers.insert("host", HeaderValue::from_static("10.0.0.156:8080"));

                for (key, val) in request.headers().iter() {
                    headers.insert(key.clone(), val.clone());
                } 

                for (key, val) in req.headers().iter() {
                    println!("{}: {:?}", key, val);
                } 


                println!("Request: {:?}", req);

                match sender.send_request(req).await {
                    Ok(resp) => {
                        let status = resp.status();
                        let body = resp.into_body();
                        let body_bytes = match body.collect().await {
                            Ok(c) => c.to_bytes().to_vec(),
                            Err(e) => {
                                eprintln!("Failed to collect response body: {}", e);
                                return;
                            }
                        };
                        let response = http::Response::builder()
                            .status(status)
                            // .extension(body_bytes)
                            .body(body_bytes)
                            .unwrap();
                        responder.respond(response);
                    }
                    Err(e) => {
                        eprintln!("Failed to send request: {}", e);
                    }
                }
            });
        })
        .setup(|app| {
            let handle = app.handle().clone();

            // 1. 初始化 Iroh Endpoint (这里使用临时节点内存模式)
            // 在实际应用中，你可能需要持久化存储 SecretKey
            tauri::async_runtime::block_on(async move {
                let endpoint = Endpoint::builder().bind().await.unwrap();

                let my_node_id = endpoint.id();
                println!("Iroh Node ID: {}", my_node_id);

                // 3. 将 Endpoint 注入 Tauri 状态供协议处理器使用
                handle.manage(endpoint);
            });
            // 4. 注册 iroh:// 自定义协议
            let app_handle = app.handle().clone();

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
