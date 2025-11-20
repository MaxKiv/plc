openocd:
    openocd -f interface/stlink.cfg -f target/stm32g4x.cfg

check:
    cargo check

run:
    cargo run

attach:
    probe-rs attach --chip STM32G474RE ./target/thumbv7em-none-eabihf/debug/plc-lite
