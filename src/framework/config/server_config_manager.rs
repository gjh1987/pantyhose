use crate::framework::config::config::{Config, ServerConfig};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

/// Server information containing type and configuration
#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub server_type: String,
    pub server_config: ServerConfig,
}

impl ServerInfo {
    pub fn new(server_type: String, server_config: ServerConfig) -> Self {
        Self {
            server_type,
            server_config,
        }
    }
}

/// Manager for server configurations with dual indexing
#[derive(Debug)]
pub struct ServerConfigManager {
    // server_id -> ServerInfo mapping
    servers_by_id: HashMap<u32, ServerInfo>,
    // server_type -> List<ServerConfig> mapping
    servers_by_type: HashMap<String, Vec<ServerConfig>>,
    // master server configuration (initialized once)
    master_config: Option<ServerConfig>,
    // author key from config
    author_key: String,
}

impl ServerConfigManager {
    /// Create a new ServerConfigManager
    pub fn new() -> Self {
        Self {
            servers_by_id: HashMap::new(),
            servers_by_type: HashMap::new(),
            master_config: None,
            author_key: String::new(),
        }
    }

    /// Initialize the manager from the global configuration
    pub fn init_from_config(&mut self, config: &Config) -> bool {

        // Clear existing data
        self.servers_by_id.clear();
        self.servers_by_type.clear();
        self.master_config = None;
        
        // Save author key from config
        self.author_key = config.author.key.clone();

        // Process each server group
        for server_group in &config.servers.group {
            let server_type = server_group.name.clone();
            let mut type_servers = Vec::new();

            // Process each server in the group
            for server_config in &server_group.server {
                let server_info = ServerInfo::new(server_type.clone(), server_config.clone());
                
                // Add to servers_by_id mapping
                if self.servers_by_id.insert(server_config.id, server_info).is_some() {
                    warn!("Duplicate server ID {} found, replacing previous entry", server_config.id);
                }

                // Add to type_servers list
                type_servers.push(server_config.clone());
            }

            // Check if this is master server type and save the first one
            if server_type == "master" && self.master_config.is_none() {
                if let Some(first_master) = type_servers.first() {
                    self.master_config = Some(first_master.clone());
                    info!("Master server config found and saved: {}:{}", first_master.back_host, first_master.back_tcp_port);
                }
            }

            // Add to servers_by_type mapping
            self.servers_by_type.insert(server_type.clone(), type_servers);
            if let Some(servers) = self.servers_by_type.get(&server_type) {
            info!("Registered {} servers of type '{}'", servers.len(), server_type);
        }
        }

        
        // Validate the configuration after initialization
        if !self.validate() {
            error!("Configuration validation failed");
            return false;
        }

        info!("Configuration validation passed");
        return true
    }

    /// Get server info by server ID
    pub fn get_server_by_id(&self, server_id: u32) -> Option<ServerInfo> {
        self.servers_by_id.get(&server_id).cloned()
    }

    /// Get all servers of a specific type
    pub fn get_servers_by_type(&self, server_type: &str) -> Option<Vec<ServerConfig>> {
        self.servers_by_type.get(server_type).cloned()
    }

    /// Get all available server types
    pub fn get_server_types(&self) -> Vec<String> {
        self.servers_by_type.keys().cloned().collect()
    }

    /// Get all server IDs
    pub fn get_server_ids(&self) -> Vec<u32> {
        self.servers_by_id.keys().cloned().collect()
    }

    /// Check if a server ID exists
    pub fn has_server(&self, server_id: u32) -> bool {
        self.servers_by_id.contains_key(&server_id)
    }

    /// Check if a server type exists
    pub fn has_server_type(&self, server_type: &str) -> bool {
        self.servers_by_type.contains_key(server_type)
    }

    /// Get server count by type
    pub fn get_server_count_by_type(&self, server_type: &str) -> usize {
        self.servers_by_type
            .get(server_type)
            .map(|servers| servers.len())
            .unwrap_or(0)
    }

    /// Get total server count
    pub fn get_total_server_count(&self) -> usize {
        self.servers_by_id.len()
    }

    /// Add a new server (for dynamic configuration)
    pub fn add_server(&mut self, server_info: ServerInfo) -> bool {
        let server_id = server_info.server_config.id;
        let server_type = server_info.server_type.clone();
        let server_config = server_info.server_config.clone();

        // Check for duplicate server ID
        if self.has_server(server_id) {
            error!("Server with ID {} already exists", server_id);
            return false;
        }

        // Add to servers_by_id
        self.servers_by_id.insert(server_id, server_info);

        // Add to servers_by_type
        self.servers_by_type.entry(server_type.clone())
            .or_insert_with(Vec::new)
            .push(server_config);

        info!("Added server ID {} of type '{}'", server_id, server_type);
        true
    }

    /// Remove a server (for dynamic configuration)
    pub fn remove_server(&mut self, server_id: u32) -> bool {
        // Get server info before removal
        let Some(server_info) = self.servers_by_id.remove(&server_id) else {
            error!("Server with ID {} not found", server_id);
            return false;
        };

        // Remove from servers_by_type
        if let Some(type_servers) = self.servers_by_type.get_mut(&server_info.server_type) {
            type_servers.retain(|config| config.id != server_id);
            
            // Remove the type entry if no servers left
            if type_servers.is_empty() {
                self.servers_by_type.remove(&server_info.server_type);
            }
        }

        info!("Removed server ID {} of type '{}'", server_id, server_info.server_type);
        true
    }

    /// Get statistics about the managed servers
    pub fn get_statistics(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        stats.insert("total_servers".to_string(), self.servers_by_id.len());
        stats.insert("total_types".to_string(), self.servers_by_type.len());

        for (server_type, servers) in self.servers_by_type.iter() {
            stats.insert(format!("type_{}_count", server_type), servers.len());
        }

        stats
    }

    /// Check for port conflicts across all servers
    fn check_port_conflicts(&self) -> bool {
        // Map: (host, port) -> Vec<(server_id, port_type)>
        let mut port_usage: HashMap<(String, u16), Vec<(u32, &str)>> = HashMap::with_capacity(
            self.servers_by_id.len() * 3 // Estimate: each server might have 3 ports max
        );

        for server_info in self.servers_by_id.values() {
            let server = &server_info.server_config;
            let server_id = server.id;

            // Check back_tcp_port (always exists) - use back_host
            port_usage
                .entry((server.back_host.clone(), server.back_tcp_port))
                .or_insert_with(Vec::new)
                .push((server_id, "back_tcp_port"));

            // Check front_tcp_port if exists - use front_host
            if let Some(port) = server.front_tcp_port {
                port_usage
                    .entry((server.front_host.clone(), port))
                    .or_insert_with(Vec::new)
                    .push((server_id, "front_tcp_port"));
            }

            // Check front_ws_port if exists - use front_host
            if let Some(port) = server.front_ws_port {
                port_usage
                    .entry((server.front_host.clone(), port))
                    .or_insert_with(Vec::new)
                    .push((server_id, "front_ws_port"));
            }
        }

        // Check for conflicts (same host:port used by multiple servers)
        for ((host, port), usage) in port_usage {
            if usage.len() > 1 {
                let conflict_details: Vec<String> = usage
                    .iter()
                    .map(|(server_id, port_type)| format!("server {} ({})", server_id, port_type))
                    .collect();
                
                error!("Port conflict detected on {}:{} - used by: {}", host, port, conflict_details.join(", "));
                return false;
            }
        }

        true
    }

    /// Validate all server configurations
    pub fn validate(&self) -> bool {
        if self.servers_by_id.is_empty() {
            error!("No servers defined in configuration");
            return false;
        }

        if self.servers_by_type.is_empty() {
            error!("No server groups defined in configuration");
            return false;
        }

        for (server_type, servers) in &self.servers_by_type {
            if servers.is_empty() {
                error!("Server group '{}' has no servers defined", server_type);
                return false;
            }

            // Check if this is a front server type
            let is_front_type = self.servers_by_id.values()
                .any(|info| info.server_type == *server_type && 
                     (info.server_config.front_tcp_port.is_some() || info.server_config.front_ws_port.is_some()));

            for server in servers {
                // If this server type has front servers, then at least one of front_tcp_port or front_ws_port must have value
                if is_front_type {
                    if server.front_tcp_port.is_none() && server.front_ws_port.is_none() {
                        error!("Server type '{}' has front servers, but server (ID: {}) has neither front_tcp_port nor front_ws_port configured", server_type, server.id);
                        return false;
                    }
                }
            }
        }

        // Check for port conflicts
        if !self.check_port_conflicts() {
            return false;
        }

        true
    }

    /// Get the master server configuration (initialized during init_from_config)
    /// Returns the master server configuration if found, or None if no master server is configured
    pub fn get_master_config(&self) -> Option<&ServerConfig> {
        self.master_config.as_ref()
    }
    
    /// Get the author key
    pub fn get_author_key(&self) -> &str {
        &self.author_key
    }
}

impl Default for ServerConfigManager {
    fn default() -> Self {
        Self::new()
    }
}