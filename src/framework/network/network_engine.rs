use std::sync::Arc;
use tracing::{info, warn};
use tokio::sync::Notify;
use crate::framework::network::{TcpServer, WebSocketServer};
use super::network_event_queue::{NetworkEventQueue, ServerType};

pub struct NetworkEngine {
    notify: Option<Arc<Notify>>,
    event_queue: NetworkEventQueue,

    back_tcp_server: Option<TcpServer>,
    front_tcp_server: Option<TcpServer>,
    front_websocket_server: Option<WebSocketServer>,
}

impl NetworkEngine {
    // ========== new methods ==========
    pub fn new() -> Self {
        Self {
            notify: None,
            event_queue: NetworkEventQueue::new(),

            back_tcp_server:None,
            front_tcp_server:None,
            front_websocket_server:None,
        }
    }
    
    // ========== get/set methods ==========
    pub fn get_event_queue(&self) -> NetworkEventQueue {
        self.event_queue.clone()
    }
    
    pub fn set_notify(&mut self, notify: Arc<Notify>) {
        self.notify = Some(notify);
    }

    // ========== other methods ==========
    fn trigger_notify(&self) {
        if let Some(notify) = &self.notify {
            notify.notify_one();
        }
    }

    pub fn start_front_tcp(&mut self, port:u16){
        if let Some(notify) = &self.notify {
            let mut server = TcpServer::new(
                port, 
                Arc::clone(notify),
                self.event_queue.clone(),
                ServerType::FrontTcp
            );
            
            if let Err(e) = server.run() {
                warn!("Failed to start front TCP server on port {}: {}", port, e);
            } else {
                info!("Front TCP server started on port {}", port);
            }
            
            self.front_tcp_server = Some(server);
        } else {
            warn!("Cannot start front TCP server: notify not set");
        }
    }

    pub fn start_front_websocket(&mut self, port:u16){
        if let Some(notify) = &self.notify {
            let mut server = WebSocketServer::new(
                port, 
                Arc::clone(notify),
                self.event_queue.clone(),
                ServerType::FrontWebSocket
            );
            
            if let Err(e) = server.run() {
                warn!("Failed to start front WebSocket server on port {}: {}", port, e);
            } else {
                info!("Front WebSocket server started on port {}", port);
            }
            
            self.front_websocket_server = Some(server);
        } else {
            warn!("Cannot start front WebSocket server: notify not set");
        }
    }

    pub fn start_back_tcp(&mut self, port:u16){
        if let Some(notify) = &self.notify {
            let mut server = TcpServer::new(
                port, 
                Arc::clone(notify),
                self.event_queue.clone(),
                ServerType::BackTcp
            );
            
            if let Err(e) = server.run() {
                warn!("Failed to start back TCP server on port {}: {}", port, e);
            } else {
                info!("Back TCP server started on port {}", port);
            }
            
            self.back_tcp_server = Some(server);
        } else {
            warn!("Cannot start back TCP server: notify not set");
        }
    }
}