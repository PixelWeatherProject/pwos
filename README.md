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

## Recommended sensor hardware
As of now, this firmware has been tested with:
- [Adafruit Si7021 Temperature & Humidity Sensor](https://www.adafruit.com/product/3251)
- [HTU21D from SparkFun](https://www.sparkfun.com/products/retired/12064)

It's recommended to use hardware from reputable brands such as Adafruit, SparkFun, DFRobot, etc. These are generally more expensive but also higher quality.

## Recommended ESP32 boards
As of now, this firmware has been tested with:
- [x] Generic ESP32 Dev board with 4MB PSRAM
- [ ] ESP32-S3
- [ ] ESP32-C3

## Drivers
All drivers for external hardware are in [`src/sysc/drivers`](src/sysc/drivers).

The firmware includes one universal driver that should be compatible with any HTU21-like sensor. It works with:
- [HTU21D from SparkFun](https://www.sparkfun.com/products/retired/12064)
- [Adafruit Si7021 Temperature & Humidity Sensor](https://www.adafruit.com/product/3251)

You could also implement your own driver, however the sensor must support temperature **and** humidity measuring at minimum. Your driver then must implement the [`EnvironmentSensor`](src/sysc/drivers/envsensor_trait.rs) trait.

## Other hardware
The project currently only supports the ESP32. There are no plans to support any other MCU.

## Power
Consumption measurements:
| **Board**                    | **Test voltage** | **Running** | **Sleeping** | **Peak** | **Notes**          |
|------------------------------|------------------|-------------|--------------|----------|--------------------|
| LYLYGO T7 V1.3 MINI 32 ESP32 | 4.2V             | 150mA       | 400µA        | >2A      | 6612C power supply |

## Building
1. Follow the toolchain setup in [Espressifs Rust Book](https://esp-rs.github.io/book/)
2. Create a custom `sys.rs` config using the [example](src/config/sys.rs.example).
3. Use `cargo build` to compile the firmware.
4. Use `cargo espflash flash --baud 921600 --port /dev/yourserialport --frozen --locked --partition-table partitions.csv` to burn the firmware onto the microcontroller.

If you just want to build the image, use the following command:
```sh
cargo espflash save-image --chip esp32 -s 4mb --merge -T partitions.csv --frozen --release --locked image.bin
```

## Build variants
Firmware size (at the time of writing this):
- Release build: `1,114,176/3,145,728 bytes, 35.42%`
- Debug build: `1,182,256/3,145,728 bytes, 37.58%`

Debug builds may be slower and contain a lot of debug messages. As such they are ~2% larger.

## Stability
__Latest verified stale version: ❓__

A version is deemed "stable" if it runs without interruptions/buggy behaviour for at least 1 month.

### Caveats
- If you're planning to flash the firmware and use it "in production", you should always use release builds. Just pass `--release` to `cargo build` **and** `cargo espflash`.
- If you just want to test the firmware, you should use debug builds. They are smaller and have more verbose logging.
- Make sure to use the given partition layout ([`partitions.csv`](partitions.csv)) by passing `--partition-table partitions.csv` to `cargo espflash`. The default partition layout has a way too small `app` partition.
- Some lower-quality ESP32 clones and USB cables may require a lower baud rate. Use `115200` if `921600` does not work for you.

## WIP Features
- [ ] OTA firmware updates
