#[derive(thiserror::Error)]
pub enum ControlError {
    Regulator,
    Valve,
}
