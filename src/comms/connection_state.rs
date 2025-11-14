#[derive(Clone, PartialEq)]
pub enum ConnectionState {
    Connected,
    Stale,
    Disconnected,
}
