openocd:
    openocd -f interface/stlink.cfg -f target/stm32g4x.cfg

check:
    cargo check

run:
    cargo run
