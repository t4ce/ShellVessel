# ESP32-S3 mini_exec proof

Contained `esp-hal` project for proving the optional `shvessel` `mini_exec`
feature on real ESP32-S3 hardware.

The app:

- initializes `esp-hal`
- adapts `esp_hal::time::Instant` into `shvessel::time::Clock`
- runs `Vessel + MiniExecutor`
- prints proof markers through `esp-println`

Expected monitor output:

```text
SHVESSEL_ESP_HAL:boot
SHVESSEL_ESP_HAL:registered
SHVESSEL_ESP_HAL:task_spawned:1
SHVESSEL_ESP_HAL:vessel_job:1
SHVESSEL_ESP_HAL:task_woke_at:<n>ms
SHVESSEL_ESP_HAL:done
```

## Tooling

Install the ESP Rust toolchain and flasher:

```bash
cargo install espup --locked
espup install
cargo install espflash --locked
```

Then source the environment printed by `espup`, or add it to your shell profile.

Without the ESP Xtensa toolchain active, a host `cargo check` may fail inside
ESP support crates before it reaches the application. Build this folder with
the toolchain/environment installed by `espup`.

## Build and flash

From this folder:

```bash
cargo run --release
```

The local `.cargo/config.toml` targets `xtensa-esp32s3-none-elf` and uses:

```text
espflash flash --monitor
```

Use an explicit port if needed:

```bash
ESPFLASH_PORT=/dev/ttyACM0 cargo run --release
```

## Notes

This proof intentionally has no command parser yet. It exercises the hardware
clock adapter, printing path, `Vessel`, `MiniExecutor`, and `PlatformRuntime`
bridge.
The next step is replacing the automatic `prove` execution with a UART or USB
serial command input loop.
