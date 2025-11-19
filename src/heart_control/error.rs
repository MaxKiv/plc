#[derive(thiserror::Error, Debug)]
pub enum ControlError {
    #[error("Unable to communicate with the Pressure Regulator")]
    Regulator,
    #[error("Unable to communicate with a Solenoid Valve")]
    Valve,
}
