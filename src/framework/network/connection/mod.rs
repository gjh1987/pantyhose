pub mod connection;
pub mod tcp_connection;
pub mod websocket_connection;

pub use connection::{Connection, ConnectionTrait, ConnectionState, ConnectionType};
pub use tcp_connection::TcpConnection;
pub use websocket_connection::WebSocketConnection;