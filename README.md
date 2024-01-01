# PixelWeatherOS

This is a universal firmware for all PixelWeather nodes. It was created using the [esp-idf template](https://github.com/esp-rs/esp-idf-template).

Hardware requirements:
- Espressif ESP32 microcontroller
    - Both Xtensa (S3 series) and RISC-V (C3 series) are supported.
    - 512KB SRAM (models with less may be sufficient)
    - PSRAM **required**
        - 2MB minimum
        - 4MB recommended
    - Dual core model recommended, but not required
- 2x resistors for measuring battery voltage. Exact values are defined in [`battery.rs`](src/sysc/battery.rs) - `DIVIDER_R1` and `DIVIDER_R2`.
- Battery - any generic 18650 will do
    - Additional protection circuit recommended
- An environment sensor
    - Temperature reading support (**required**)
    - Humidity reading support (**required**)
    - _Optional_:
        - Air pressure reading support
    - I2C interface

Software requirements (for building):
- [Rust](https://rustlang.org/)
- [ESP32 Rust toolchain](https://esp-rs.github.io/book/)
- A PixelWeather Messaging Protocol server

## Recommended hardware
As of now, this firmware has been tested with:
- [Adafruit Si7021 Temperature & Humidity Sensor](https://www.adafruit.com/product/3251)
- Generic ESP32 Dev board with 4MB PSRAM

It's recommended to use hardware from reputable brands such as Adafruit, SparkFun, DFRobot, etc. These are generally more expensive but also higher quality.

## Drivers
The firmware includes drivers for the Si7021 and HTU21D temperature & humidity sensors. You could also implement your own driver, however the sensor must support temperature **and** humidity measuring at minimum. Your driver then must implement the `EnvironmentSensor` trait.

## Other hardware
The project currently only supports the ESP32 and no support is planned for any other hardware at the moment.

## Building
1. Follow the toolchain setup in [Espressifs Rust Book](https://esp-rs.github.io/book/)
2. Use `cargo build` to compile the firmware.
3. Use `cargo espflash flash --baud 921600 --port /dev/yourserialport --frozen --locked --partition-table partitions.csv` to burn the firmware onto the microcontroller.

### Caveats
- If you're planning to flash the firmware and use it "in production", you should always use release builds. Just pass `--release` to `cargo build` **and** `cargo espflash`.
- If you just want to test the firmware, you should use debug builds. They are smaller and have more verbose logging.
- Make sure to use the given partition layout ([`partitions.csv`](partitions.csv)) by passing `--partition-table partitions.csv` to `cargo espflash`. The default partition layout has a way too small `app` partition.
- Some lower-quality ESP32 clones and USB cables may require a lower baud rate. Use `115200` if `921600` does not work for you.

## WIP Features
- [ ] OTA firmware updates