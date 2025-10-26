use std::net::SocketAddr;

pub trait SessionTrait {
    fn get_session_id(&self) -> u64;
    fn get_user_id(&self) -> Option<u64>;
    fn set_user_id(&mut self, user_id: u64) -> bool;
    fn get_remote_addr(&self) -> Option<SocketAddr>;
    fn is_connected(&self) -> bool;
    fn is_authenticated(&self) -> bool;
    fn set_authenticated(&mut self, authenticated: bool) -> bool;
    fn close(&mut self) -> bool;
}