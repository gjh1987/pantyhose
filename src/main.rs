mod framework;
mod servers;
mod proto;

use std::env;
use framework::server::ServerTrait;
use servers::chat::chat_server::ChatServer;
use servers::session::session_server::SessionServer;
use framework::master::MasterServer;
use tracing::error;

use crate::framework::config::config::Config;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    
    let config_path = args.get(1)
        .cloned()
        .unwrap_or_else(|| "bin/config.xml".to_string());
        
    let server_id = args.get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    
    let config = match Config::from_file(config_path) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to load config: {}", e);
            return;
        }
    };

    let (server_config, server_group) = match config.find_server(server_id) {
        Some((s, g)) => (s, g),
        None => {
            error!("No server configuration found for ID: {}", server_id);
            return;
        }
    };

    enum ServerType {
        Chat(ChatServer),
        Session(SessionServer),
        Master(MasterServer),
    }
    
    impl ServerType {
        fn init(&mut self, server_id: u32, config: &Config) -> bool {
            match self {
                ServerType::Chat(server) => server.init(server_id, config),
                ServerType::Session(server) => server.init(server_id, config),
                ServerType::Master(server) => server.init(server_id, config),
            }
        }
        
        fn late_init(&mut self) -> bool {
            match self {
                ServerType::Chat(server) => server.lateInit(),
                ServerType::Session(server) => server.lateInit(),
                ServerType::Master(server) => server.lateInit(),
            }
        }
        
        async fn run(&mut self) {
            match self {
                ServerType::Chat(server) => server.run().await,
                ServerType::Session(server) => server.run().await,
                ServerType::Master(server) => server.run().await,
            }
        }
        
        fn dispose(&mut self) {
            match self {
                ServerType::Chat(server) => server.dispose(),
                ServerType::Session(server) => server.dispose(),
                ServerType::Master(server) => server.dispose(),
            }
        }
    }

    let mut server = if server_group.name == ChatServer::ServerName() {
        ServerType::Chat(ChatServer::new())
    } else if server_group.name == SessionServer::ServerName() {
        ServerType::Session(SessionServer::new())
    } else if server_group.name == MasterServer::ServerName() {
        ServerType::Master(MasterServer::new())
    } else {
        error!("Unknown server type: {}", server_group.name);
        return;
    };
   
    if server.init(server_id, &config) == false {
        error!("Failed to initialize server");
        server.dispose();
        return;
    }
    
    if server.late_init() == false {
        error!("Failed in late initialization");
        server.dispose();
        return;
    }
    
    server.run().await;
    server.dispose();
}
