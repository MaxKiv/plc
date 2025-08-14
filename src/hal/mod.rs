//! HAL facade
//! picks MCU-specific embedded-hal implementation

#[cfg(feature = "stm32f103c6")]
mod stm32f103c6;
#[cfg(feature = "stm32f103c6")]
pub use stm32f103c6::Hal;

#[cfg(feature = "stm32g474re")]
mod stm32g474re;
#[cfg(feature = "stm32g474re")]
pub use stm32g474re::Hal;
