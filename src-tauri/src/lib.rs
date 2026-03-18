use futures_util::{Stream, StreamExt};
use http::HeaderValue;
use http_body_util::{BodyExt, Full};
use iroh::endpoint::{RecvStream, SendStream};
use iroh::Endpoint;
use nexapipe::EndpointTicket;
use nexapipe;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
use tauri::Manager;

// 连接存储结构体
struct ConnectionStore {
    connection: Mutex<Option<Arc<dyn Send + Sync + 'static>>>,
}

impl ConnectionStore {
    fn new() -> Self {
        Self {
            connection: Mutex::new(None),
        }
    }
    
    fn set_connection(&self, conn: Arc<dyn Send + Sync + 'static>) {
        *self.connection.lock().unwrap() = Some(conn);
    }
    
    fn get_connection(&self) -> Option<Arc<dyn Send + Sync + 'static>> {
        self.connection.lock().unwrap().clone()
    }
    
    fn reset_connection(&self) {
        *self.connection.lock().unwrap() = None;
    }
}

pub mod tokiort;

type ClientBuilder = hyper::client::conn::http1::Builder;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .register_asynchronous_uri_scheme_protocol("iroh", move |app, request, responder| {
            println!("Request: {:?}", request);
            let endpoint = app.app_handle().state::<Endpoint>().inner().clone();
            let connection_store = app.app_handle().state::<Arc<ConnectionStore>>().inner().clone();
            let uri = request.uri();
            let host = "10.0.0.156:18080";
            let path = uri.path_and_query().unwrap().to_string(); 
            let method = request.method().clone();

            tauri::async_runtime::spawn(async move {
                let ticket = match EndpointTicket::from_str(
                    "endpointacpmyz2js7veawlqdag5udh3t2evk2w2jiy4xycmuodlyyjhrmwkyaiaf5uhi5dqom5c6l3von3tcljrfzzgk3dbpexg4mbonfzg62bnmnqw4ylspexgs4tpnaxgy2lonmxc6",
                ) {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!("Failed to parse ticket: {}", e);
                        return;
                    }
                };

                // 尝试获取现有连接，如果没有则建立新连接
                let (mut send, recv) = match endpoint.connect(ticket.clone(), &nexapipe::ALPN.to_vec()).await {
                    Ok(conn) => {
                        // 存储连接
                        let conn_arc = Arc::new(conn);
                        connection_store.set_connection(conn_arc.clone());
                        // 打开 bi-stream
                        match conn_arc.open_bi().await {
                            Ok(p) => p,
                            Err(e) => {
                                eprintln!("Failed to open bi-stream: {}", e);
                                // 发送错误响应
                                let error_response = http::Response::builder()
                                    .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                                    .body(format!("Failed to open bi-stream: {}", e).as_bytes().to_vec())
                                    .unwrap();
                                responder.respond(error_response);
                                connection_store.reset_connection();
                                return;
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to connect: {}", e);
                        // 发送错误响应
                        let error_response = http::Response::builder()
                            .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                            .body(format!("Failed to connect: {}", e).as_bytes().to_vec())
                            .unwrap();
                        responder.respond(error_response);
                        return;
                    }
                };
                let send_result = send.write_all(&nexapipe::HANDSHAKE).await;
                if let Err(e) = send_result {
                    eprintln!("Failed to send handshake: {}", e);
                    // 发送错误响应
                    let error_response = http::Response::builder()
                        .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                        .body(format!("Failed to send handshake: {}", e).as_bytes().to_vec())
                        .unwrap();
                    responder.respond(error_response);
                    // 重置连接，下次请求会重新建立
                    connection_store.reset_connection();
                    return;
                }

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
                        // 发送错误响应
                        let error_response = http::Response::builder()
                            .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                            .body(format!("Handshake failed: {}", e).as_bytes().to_vec())
                            .unwrap();
                        responder.respond(error_response);
                        // 重置连接，下次请求会重新建立
                        connection_store.reset_connection();
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
                    .uri(path)
                    .method(method)
                    .body(body)
                {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Failed to build request: {}", e);
                        // 发送错误响应
                        let error_response = http::Response::builder()
                            .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                            .body(format!("Failed to build request: {}", e).as_bytes().to_vec())
                            .unwrap();
                        responder.respond(error_response);
                        return;
                    }
                };
                let  headers =req.headers_mut(); 
                headers.insert("host", HeaderValue::from_str(&host).unwrap());

                for (key, val) in request.headers().iter() {
                    headers.insert(key.clone(), val.clone());
                } 

                for (key, val) in req.headers().iter() {
                    println!("{}: {:?}", key, val);
                } 


                println!("Request: {:?}", req);

                match sender.send_request(req).await {
                    Ok(response) => {
                        let status = response.status();
                        let headers = response.headers().clone();
 
                        // 读取响应体
                        let body = match http_body_util::BodyExt::collect(response.into_body()).await {
                            Ok(body) => body.to_bytes().to_vec(),
                            Err(_) => {
                                let response = http::Response::builder()
                                    .status(500)
                                    .body(std::borrow::Cow::Borrowed(&[] as &[u8]))
                                    .unwrap();
                                responder.respond(response);
                                return;
                            }
                        };

                        // 构建响应
                        let mut http_response_builder = http::Response::builder()
                            .status(status)
                            .header("Access-Control-Allow-Origin", "*")
                            .header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
                            .header("Access-Control-Allow-Headers", "Content-Type, Authorization");

                        // 先检查是否有Content-Range头
                        let has_content_range = headers.contains_key(http::header::CONTENT_RANGE);

                        // 遍历并添加所有头部
                        for (key, value) in &headers {
                            // 跳过 CORS 相关的头，使用我们自己设置的
                            if key != &http::header::ACCESS_CONTROL_ALLOW_ORIGIN &&
                               key != &http::header::ACCESS_CONTROL_ALLOW_METHODS &&
                               key != &http::header::ACCESS_CONTROL_ALLOW_HEADERS {
                                http_response_builder = http_response_builder.header(key, value.to_owned());
                            }
                        }

                        // 处理Range请求和部分内容响应
                        if status == http::StatusCode::PARTIAL_CONTENT {
                            // 确保Content-Range头存在
                            if has_content_range {
                                // 返回部分内容响应
                                if let Ok(resp) = http_response_builder.body(std::borrow::Cow::Owned(body)) {
                                    responder.respond(resp);
                                } else {
                                    let response = http::Response::builder()
                                        .status(status)
                                        .body(std::borrow::Cow::Borrowed(&[] as &[u8]))
                                        .unwrap();
                                    responder.respond(response);
                                }
                            } else {
                                // 如果没有Content-Range头，返回完整内容
                                let mut full_response_builder =
                                    http::Response::builder().status(http::StatusCode::OK);
                                for (key, value) in &headers {
                                    full_response_builder = full_response_builder.header(key, value.to_owned());
                                }
                                if let Ok(resp) = full_response_builder.body(std::borrow::Cow::Owned(body)) {
                                    responder.respond(resp);
                                } else {
                                    let response = http::Response::builder()
                                        .status(http::StatusCode::OK)
                                        .body(std::borrow::Cow::Borrowed(&[] as &[u8]))
                                        .unwrap();
                                    responder.respond(response);
                                }
                            }
                        } else {
                            // 返回完整内容响应
                            if let Ok(resp) = http_response_builder.body(std::borrow::Cow::Owned(body)) {
                                responder.respond(resp);
                            } else {
                                let response = http::Response::builder()
                                    .status(status)
                                    .body(std::borrow::Cow::Borrowed(&[] as &[u8]))
                                    .unwrap();
                                responder.respond(response);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to send request: {}", e);
                        // 发送错误响应
                        let error_response = http::Response::builder()
                            .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                            .header("Access-Control-Allow-Origin", "*")
                            .header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
                            .header("Access-Control-Allow-Headers", "Content-Type, Authorization")
                            .body(format!("Failed to send request: {}", e).as_bytes().to_vec())
                            .unwrap();
                        responder.respond(error_response);
                        // 重置连接，下次请求会重新建立
                        connection_store.reset_connection();
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

                // 2. 初始化连接存储
                let connection_store = Arc::new(ConnectionStore::new());

                // 3. 建立初始连接
                let ticket = EndpointTicket::from_str(
                    "endpointaalojw2f2o37vs7pzkynsdt2xjgdpvh2l6iyfhubrrri7zjpt5smyaiaf5uhi5dqom5c6l3von3tcljrfzzgk3dbpexg4mbonfzg62bnmnqw4ylspexgs4tpnaxgy2lonmxc6",
                ).unwrap();
                
                match endpoint.connect(ticket, &nexapipe::ALPN.to_vec()).await {
                    Ok(conn) => {
                        let conn = Arc::new(conn);
                        connection_store.set_connection(conn);
                        println!("Initial connection established successfully");
                    },
                    Err(e) => {
                        eprintln!("Failed to establish initial connection: {}", e);
                    }
                }

                // 4. 将 Endpoint 和连接存储注入 Tauri 状态供协议处理器使用
                handle.manage(endpoint);
                handle.manage(connection_store);
            }); 

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
