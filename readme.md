# plc-lite

Firmware for SMARTs Holland Hybrid Heart Mockloop measuring & control system.

# TODO

Test out current blocking one-shot ADC vs continous scan-mode using DMA

- [x] read out single adc/multiple one-shot channels
- [x] use single DMA channel with single ADC using multiple scan-mode channels

When even more adc performance is required, try this:

- [ ] Split sensor inputs over the 5 different ADC's using interleaved mode & use multiple DMA channels (different dma controllers make no sense since they share the bus matrix -> the bus arbitrator would yield control sequentially and no further performance is gained)

Figure out the serialisation protocol to/from the RPI

- [ ] Use Postcard & shared crate, See below

Implement the control loop

- [x] Receive adc frame
- [ ] AdcFrame -> Report (Filtering?, mv->quantity conversion)
- [ ] AppState management + Emergency stop loop
- [ ] Pneumatic heart control loop
- [ ] impl heart controller pressure regulator output using DAC

## Uart comms ideas

Alright — let’s break this down step-by-step, because we’re mixing three concepts here:

1. **UART + DMA behavior in Embassy**
2. **How to frame messages over UART so you can deserialize cleanly**
3. **Deserializing a single `Setpoint` sent from the RPi using Postcard**

---

## 1. How Embassy’s `Uart` with DMA behaves

When you use:

```rust
uart.read(&mut buf).await.unwrap();
```

- This reads **exactly** `buf.len()` bytes before returning.
- If fewer bytes arrive, it waits until the buffer is full.

When you use:

```rust
uart.read_until_idle(&mut buf).await.unwrap();
```

- DMA starts filling the buffer.
- As soon as there’s a pause on the line (idle condition), Embassy stops DMA and returns the number of bytes actually received.
- Useful when you don’t know the exact message length in advance.

**Key:** DMA doesn’t “already have” all bytes somewhere unless you set up a ring buffer manually.
By default, Embassy starts DMA when you call `read()` or `read_until_idle()`.
If nothing’s reading, bytes are dropped.

---

## 2. Framing your UART messages for Postcard

UART is just a byte stream — no boundaries.
If you send multiple serialized messages back-to-back, the receiver can’t tell where one ends unless you add **framing**.
With Postcard, a common approach is to use `postcard-rpc` or the [postcard streaming API](https://docs.rs/postcard/latest/postcard/accumulator/index.html):

- **Option A:** Prefix each message with a length byte/word.
- **Option B:** Use Postcard’s `postcard::accumulator::CobsAccumulator` with [COBS](https://en.wikipedia.org/wiki/Consistent_Overhead_Byte_Stuffing) framing.
  This lets you detect message boundaries reliably, even if messages have variable length.

---

## 3. Receiving and deserializing a `Setpoint` from RPi

### Using `read_until_idle`

If you know the message fits in your buffer (and is sent in one go):

```rust
use postcard::from_bytes;
use crate::comms::messages::Setpoint;

let mut buf = [0u8; 64];

loop {
    let n = uart.read_until_idle(&mut buf).await.unwrap();
    let msg: Setpoint = from_bytes(&buf[..n]).unwrap();
    // process msg...
}
```

Downside:
If the RPi sends multiple messages without idle gaps, you might get two concatenated messages in one read — `from_bytes` will fail.

---

### Using COBS + Postcard (more robust)

Sender (RPi) wraps postcard output in COBS encoding + `0x00` terminator.
Receiver uses `CobsAccumulator`:

```rust
use postcard::accumulator::{CobsAccumulator, FeedResult};
use postcard::from_bytes_cobs;

let mut acc = CobsAccumulator::<64>::new();
let mut buf = [0u8; 32];

loop {
    let n = uart.read_until_idle(&mut buf).await.unwrap();
    for &b in &buf[..n] {
        match acc.feed(b) {
            FeedResult::Success { data, .. } => {
                let msg: Setpoint = from_bytes_cobs(data).unwrap();
                // process msg...
            }
            FeedResult::OverFull(_) => {
                // handle error
                acc.reset();
            }
            _ => {}
        }
    }
}
```

This way, you can:

- Send multiple messages back-to-back
- Recover from byte drops or corruption
- Avoid depending on idle gaps

---

✅ **TL;DR**

- `read()` waits for exact N bytes; `read_until_idle()` waits for a pause in reception.
- Without framing, you risk boundary problems — COBS + Postcard fixes this.
- Deserialize with `postcard::from_bytes` or `from_bytes_cobs` if you use framing.

---

If you want, I can **rewrite your `manage_communications` task** so it **sends framed postcard messages** to the RPi **and** listens for framed `Setpoint` messages in the same async loop.
That would give you a robust two-way link.

Do you want me to go ahead with that?
