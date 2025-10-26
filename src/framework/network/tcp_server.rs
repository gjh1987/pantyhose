use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tracing::{info, error, debug};
use super::network_event_queue::{NetworkEventQueue, NetworkEventData, NetworkEventType, ServerType};

pub struct TcpServer {
    port: u16,
    listener: Option<TcpListener>,
    task_handle: Option<JoinHandle<()>>,
    notify: Arc<Notify>,
    event_queue: NetworkEventQueue,
    server_type: ServerType,
}

impl TcpServer {
    pub fn new(port: u16, notify: Arc<Notify>, event_queue: NetworkEventQueue, server_type: ServerType) -> Self {
        Self {
            port,
            listener: None,
            task_handle: None,
            notify,
            event_queue,
            server_type,
        }
    }

    fn trigger_notify(&self) {
        self.notify.notify_one();
    }

    pub fn run(&mut self) -> Result<(), String> {
        let addr = format!("127.0.0.1:{}", self.port)
            .parse::<SocketAddr>()
            .map_err(|e| format!("Failed to parse address: {}", e))?;
        let notify = Arc::clone(&self.notify);
        let event_queue = self.event_queue.clone();
        let server_type = self.server_type;

        self.task_handle = Some(tokio::spawn(async move {
            let listener = match TcpListener::bind(addr).await {
                Ok(listener) => listener,
                Err(e) => {
                    error!("Failed to bind to address: {}", e);
                    return;
                }
            };
            
            info!("TCP server ({:?}) listening on {}", server_type, addr);
            
            // 端口绑定成功，触发ServerOpen事件
            let event = NetworkEventData::new(
                NetworkEventType::ServerOpen,
                server_type,
                0, // ServerOpen不需要session_id
                Some(addr),
            );
            event_queue.push(event).await;
            info!("Server opened on {} (type: {:?})", addr, server_type);
            
            // 通知主循环处理事件
            notify.notify_one();
            
            loop {
                match listener.accept().await {
                    Ok((stream, client_addr)) => {
                        
                        // 触发NewTcpConnection事件，并包含TcpStream
                        // 注意：此时还没有session，session_id设为0，由SessionManager在创建session时分配
                        let event = NetworkEventData::new_with_stream(
                            NetworkEventType::NewTcpConnection,
                            server_type,
                            0, // session还未创建，暂时为0
                            Some(client_addr),
                            stream,
                        );
                        event_queue.push(event).await;
                        
                        // 通知主循环处理事件
                        notify.notify_one();
                        
                        // TODO: 处理连接断开事件
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                    }
                }
            }
        }));

        Ok(())
    }

    pub fn dispose(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
        self.listener = None;
    }
}
