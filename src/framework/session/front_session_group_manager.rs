use super::front_session_group::FrontSessionGroup;
use super::session_trait::SessionTrait;
use std::collections::HashMap;
use tracing::{info, debug, warn, error};

pub struct FrontSessionGroupManager {
    groups: HashMap<u64, FrontSessionGroup>,
    next_group_id: u64,
    session_to_group: HashMap<u64, u64>,
}

impl FrontSessionGroupManager {
    pub fn new() -> Self {
        Self {
            groups: HashMap::new(),
            next_group_id: 1,
            session_to_group: HashMap::new(),
        }
    }

    pub fn create_group(&mut self, group_name: String, max_sessions: Option<usize>) -> u64 {
        let group_id = self.next_group_id;
        self.next_group_id += 1;

        let group = FrontSessionGroup::new(group_id, group_name.clone(), max_sessions);
        self.groups.insert(group_id, group);
        
        info!("Created front session group {} with name '{}' and max sessions {:?}", 
              group_id, group_name, max_sessions);
        group_id
    }

    pub fn remove_group(&mut self, group_id: u64) -> bool {
        if let Some(group) = self.groups.remove(&group_id) {
            for session_id in group.get_session_ids() {
                self.session_to_group.remove(&session_id);
            }
            info!("Removed front session group {} ('{}')", group_id, group.get_group_name());
            true
        } else {
            error!("Attempt to remove non-existent group {}", group_id);
            false
        }
    }

    pub fn get_group(&self, group_id: u64) -> Option<&FrontSessionGroup> {
        self.groups.get(&group_id)
    }

    pub fn get_group_mut(&mut self, group_id: u64) -> Option<&mut FrontSessionGroup> {
        self.groups.get_mut(&group_id)
    }

    pub fn get_group_by_name(&self, group_name: &str) -> Option<&FrontSessionGroup> {
        self.groups.values().find(|group| group.get_group_name() == group_name)
    }

    pub fn get_group_by_name_mut(&mut self, group_name: &str) -> Option<&mut FrontSessionGroup> {
        self.groups.values_mut().find(|group| group.get_group_name() == group_name)
    }

    pub fn add_session_to_group(&mut self, group_id: u64, session_id: u64) -> bool {
        if self.session_to_group.contains_key(&session_id) {
            error!("Session {} is already in a group", session_id);
            return false;
        }

        if let Some(group) = self.groups.get_mut(&group_id) {
            if group.add_session(session_id) {
                self.session_to_group.insert(session_id, group_id);
                true
            } else {
                false
            }
        } else {
            error!("Group {} does not exist", group_id);
            false
        }
    }

    pub fn remove_session_from_group(&mut self, session_id: u64) -> bool {
        if let Some(&group_id) = self.session_to_group.get(&session_id) {
            if let Some(group) = self.groups.get_mut(&group_id) {
                if group.remove_session(session_id) {
                    self.session_to_group.remove(&session_id);
                    true
                } else {
                    false
                }
            } else {
                warn!("Session {} mapped to non-existent group {}", session_id, group_id);
                self.session_to_group.remove(&session_id);
                false
            }
        } else {
            warn!("Session {} is not in any group", session_id);
            false
        }
    }

    pub fn get_session_group_id(&self, session_id: u64) -> Option<u64> {
        self.session_to_group.get(&session_id).cloned()
    }

    pub fn get_session_group(&self, session_id: u64) -> Option<&FrontSessionGroup> {
        self.session_to_group.get(&session_id)
            .and_then(|&group_id| self.groups.get(&group_id))
    }

    pub fn broadcast_to_group<T>(&self, group_id: u64, message: T, front_session_manager: &mut super::FrontSessionManager) -> usize
    where
        T: crate::proto::messages::MessageIdSerialize + Clone + Send + 'static,
    {
        if let Some(group) = self.groups.get(&group_id) {
            let mut sent_count = 0;
            for session_id in group.get_session_ids() {
                if let Some(session) = front_session_manager.get_session_mut(session_id) {
                    if session.is_connected() && session.send_message(message.clone()) {
                        sent_count += 1;
                    }
                }
            }
            debug!("Broadcast message to {} sessions in group {} ('{}')", 
                   sent_count, group_id, group.get_group_name());
            sent_count
        } else {
            error!("Cannot broadcast to non-existent group {}", group_id);
            0
        }
    }

    pub fn broadcast_to_group_by_name<T>(&self, group_name: &str, message: T, front_session_manager: &mut super::FrontSessionManager) -> usize
    where
        T: crate::proto::messages::MessageIdSerialize + Clone + Send + 'static,
    {
        if let Some(group) = self.get_group_by_name(group_name) {
            let group_id = group.get_group_id();
            self.broadcast_to_group(group_id, message, front_session_manager)
        } else {
            error!("Cannot broadcast to non-existent group '{}'", group_name);
            0
        }
    }

    pub fn get_group_count(&self) -> usize {
        self.groups.len()
    }

    pub fn get_total_sessions_in_groups(&self) -> usize {
        self.groups.values().map(|group| group.get_session_count()).sum()
    }

    pub fn get_groups_info(&self) -> Vec<(u64, String, usize, Option<usize>)> {
        self.groups.values()
            .map(|group| (
                group.get_group_id(),
                group.get_group_name().to_string(),
                group.get_session_count(),
                group.get_max_sessions()
            ))
            .collect()
    }

    pub fn cleanup_empty_groups(&mut self) -> usize {
        let empty_groups: Vec<u64> = self.groups.iter()
            .filter(|(_, group)| group.is_empty())
            .map(|(&group_id, _)| group_id)
            .collect();

        let count = empty_groups.len();
        for group_id in empty_groups {
            self.remove_group(group_id);
        }

        if count > 0 {
            info!("Cleaned up {} empty groups", count);
        }
        count
    }

    pub fn cleanup_invalid_sessions(&mut self, valid_session_ids: &std::collections::HashSet<u64>) -> usize {
        let mut removed_count = 0;
        let invalid_sessions: Vec<u64> = self.session_to_group.keys()
            .filter(|&&session_id| !valid_session_ids.contains(&session_id))
            .cloned()
            .collect();

        for session_id in invalid_sessions {
            if self.remove_session_from_group(session_id) {
                removed_count += 1;
            }
        }

        if removed_count > 0 {
            info!("Cleaned up {} invalid sessions from groups", removed_count);
        }
        removed_count
    }

    pub fn clear_all_groups(&mut self) {
        let group_count = self.groups.len();
        let session_count = self.session_to_group.len();
        
        self.groups.clear();
        self.session_to_group.clear();
        
        if group_count > 0 || session_count > 0 {
            info!("Cleared {} groups and {} session mappings", group_count, session_count);
        }
    }
}