use std::collections::HashSet;
use tracing::{info, debug, warn, error};

#[derive(Debug, Clone)]
pub struct FrontSessionGroup {
    group_id: u64,
    group_name: String,
    session_ids: HashSet<u64>,
    max_sessions: Option<usize>,
}

impl FrontSessionGroup {
    pub fn new(group_id: u64, group_name: String, max_sessions: Option<usize>) -> Self {
        Self {
            group_id,
            group_name,
            session_ids: HashSet::new(),
            max_sessions,
        }
    }

    pub fn get_group_id(&self) -> u64 {
        self.group_id
    }

    pub fn get_group_name(&self) -> &str {
        &self.group_name
    }

    pub fn set_group_name(&mut self, name: String) {
        self.group_name = name;
    }

    pub fn get_max_sessions(&self) -> Option<usize> {
        self.max_sessions
    }

    pub fn set_max_sessions(&mut self, max_sessions: Option<usize>) {
        self.max_sessions = max_sessions;
    }

    pub fn add_session(&mut self, session_id: u64) -> bool {
        if let Some(max) = self.max_sessions {
            if self.session_ids.len() >= max {
                error!("Group {} ({}) has reached max sessions limit: {}", 
                       self.group_id, self.group_name, max);
                return false;
            }
        }

        if self.session_ids.insert(session_id) {
            debug!("Added session {} to group {} ({})", session_id, self.group_id, self.group_name);
            true
        } else {
            warn!("Session {} already exists in group {} ({})", session_id, self.group_id, self.group_name);
            false
        }
    }

    pub fn remove_session(&mut self, session_id: u64) -> bool {
        if self.session_ids.remove(&session_id) {
            debug!("Removed session {} from group {} ({})", session_id, self.group_id, self.group_name);
            true
        } else {
            warn!("Session {} not found in group {} ({})", session_id, self.group_id, self.group_name);
            false
        }
    }

    pub fn contains_session(&self, session_id: u64) -> bool {
        self.session_ids.contains(&session_id)
    }

    pub fn get_session_ids(&self) -> Vec<u64> {
        self.session_ids.iter().cloned().collect()
    }

    pub fn get_session_count(&self) -> usize {
        self.session_ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.session_ids.is_empty()
    }

    pub fn is_full(&self) -> bool {
        if let Some(max) = self.max_sessions {
            self.session_ids.len() >= max
        } else {
            false
        }
    }

    pub fn clear_sessions(&mut self) {
        let count = self.session_ids.len();
        self.session_ids.clear();
        if count > 0 {
            info!("Cleared {} sessions from group {} ({})", count, self.group_id, self.group_name);
        }
    }

    pub fn get_available_slots(&self) -> Option<usize> {
        self.max_sessions.map(|max| max.saturating_sub(self.session_ids.len()))
    }
}