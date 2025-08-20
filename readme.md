# plc-lite

Firmware for SMARTs Holland Hybrid Heart Mockloop measuring & control system.

# TODO

Test out current blocking one-shot ADC vs continous scan-mode using DMA

- [x] read out single adc/multiple one-shot channels
- [x] use single DMA channel with single ADC using multiple scan-mode channels

When even more adc performance is required, try this:

- [ ] Split sensor inputs over the 5 different ADC's using interleaved mode & use multiple DMA channels (different dma controllers make no sense since they share the bus matrix -> the bus arbitrator would yield control sequentially and no further performance is gained)

Figure out the serialisation protocol to/from the RPI

- [x] Use Postcard & shared crate, See below
- [ ] Refactor love_letter to use COBS encoding

Implement the control loop

- [x] Receive adc frame
- [ ] AdcFrame -> Report (Filtering?, mv->quantity conversion)
- [ ] AppState management + Emergency stop loop
- [ ] Pneumatic heart control loop
- [ ] impl heart controller pressure regulator output using DAC
