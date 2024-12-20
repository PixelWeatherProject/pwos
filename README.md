# PixelWeatherOS
This is a universal firmware for all PixelWeather nodes. It was created using the [esp-idf template](https://github.com/esp-rs/esp-idf-template).

PixelWeather is a weather station network that collects environment data using "nodes" (a collection of microcontrollers and sensors). This repository contains the firware for said nodes _(PWOS)_.

**⚠️ Note that this project is under development. While it is decently stable, is not complete! There are missing and incomplete implementations of features. Production use is highly discouraged!**

### Hardware requirements:
- Espressif ESP32 microcontroller
    - Generic ESP32 series with Xtensa CPU.
    - S3 series and RISC-V (C3) series should work too, but haven't been tested.
    - 4MB Flash minimum
        - Read section [Build variants](#build-variants) for details
    - 512KB SRAM (models with less may be sufficient)
    - PSRAM **not** required, it's not used (yet)
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

> **⚠️ Note**: OTA support is planned, which **will** increase the minimum hardware requirements, especially flash size to **at least 8MB**. Additionally, **at least 2MB** of PSRAM *may* be required to temporarily store downloaded firmware upgrades in RAM. You may want to check out the newer S3 series, which usually come with much larger flash sizes.

### Software requirements (for building):
- [Rust](https://rustlang.org/)
- [ESP32 Rust toolchain](https://esp-rs.github.io/book/)

## Recommended ESP32 boards
As of now, this firmware has been tested with:
- [x] Generic ESP32 Dev board with 4MB PSRAM
    - [x] [LYLYGO T7 V1.3 MINI 32 ESP32](https://lilygo.cc/products/t7-v1-3-mini-32-esp32)
- [ ] ESP32-S3 (untested)
- [ ] ESP32-C3 (untested)

## Recommended sensor hardware
As of now, this firmware has been tested with:
- [Adafruit Si7021 Temperature & Humidity Sensor](https://www.adafruit.com/product/3251)
- [HTU21D from SparkFun](https://www.sparkfun.com/products/retired/12064)

It's recommended to use hardware from reputable brands such as Adafruit, SparkFun, DFRobot, etc. These are generally more expensive but also higher quality.

## Code structure
- [`src/firmware.rs`](/src/firmware.rs) - This is the entry point for the firmware. If you want to explore this project, you should start from here.
- [`src/main.rs`](/src/main.rs) - The main entry point, it's responsible for initializing core components.
- [`src/sysc/`](/src/sysc/) - Contains components of PWOS
- [`src/config/`](src/config/) - Contains configuration definitions for the firmware.

## Drivers
All drivers for external hardware are in [`src/sysc/drivers`](src/sysc/drivers).

The firmware includes one universal driver that should be compatible with any HTU21-like sensor. It works with:
- [HTU21D from SparkFun](https://www.sparkfun.com/products/retired/12064)
- [Adafruit Si7021 Temperature & Humidity Sensor](https://www.adafruit.com/product/3251)

You could also implement your own driver, however the sensor must support temperature **and** humidity measuring at minimum. Your driver then must implement the [`EnvironmentSensor`](src/sysc/drivers/envsensor_trait.rs) trait.

Using multiple environment sensors is **not** supported. The firmware will use the first sensor it finds (which is typically the one with the lowest I2C address). This also means that every I2C hardware must use a different address.

## Other hardware
The project currently only supports the ESP32. There are no plans to support any other MCU.

## Power
Consumption measurements:
| **Board**                    | **Sensor**      | **Test voltage** | **Running** | **Sleeping** | **Peak** | **Notes**                     |
|------------------------------|-----------------|------------------|-------------|--------------|----------|-------------------------------|
| LYLYGO T7 V1.3 MINI 32 ESP32 | Adafruit Si7012 | 4.2V             | 150mA       | 400µA        | >2A      | 6612C power supply, peaks >2? |

Battery life measurements:
| **Board**                    | **Sensor**      | **Battery model** | **Capacity** | **Environment**      | **Sleep time** | **Time**        |
|------------------------------|-----------------|-------------------|--------------|----------------------|:--------------:|:---------------:|
| LYLYGO T7 V1.3 MINI 32 ESP32 | Adafruit Si7012 | Generic 18650     | 2.2Ah        | Outdoor (12-37°C)    | 10m            | 41d/7h/58m      |
| LYLYGO T7 V1.3 MINI 32 ESP32 | Adafruit Si7012 | Generic 18650     | ~2Ah         | Indoor (26-32°C)     | 10m            | 27d/4h/55m      |

**Note that the battery voltage measurement is currently unreliable.**

## Building
1. Follow the toolchain setup in [Espressifs Rust Book](https://esp-rs.github.io/book/)
2. Create a custom `sys.rs` config using the [example](src/config/sys.rs.example).
3. Use `cargo build` to compile the firmware.
4. Use the commands below to build an image or flash the firmware.

If you just want to build the image, use the following command:
```sh
cargo espflash save-image -T partitions.csv --frozen --locked --release --chip esp32 -s 4mb --merge image.bin
```

To directly flash the firmware, use the command below. **Remember to change the serial port for your machine.**
```sh
cargo espflash flash -T partitions.csv --frozen --locked --release --baud 921600 --port /dev/cu.usbserial-XXXXXXXX
```

To build a debug image (or flash it) remove the `--release` flag from the above commands.

## Build variants
Firmware size (as of commit 2b34673):
- Release build: `1,131,776/3,145,728 bytes, 35.98%`
- Debug build: `1,209,696/3,145,728 bytes, 38.46%`

Debug builds may be slower and contain a lot of debug messages. As such they are ~2% larger.

You will likely need an ESP32 chip with at least 4MB of Flash memory. About ~25% of this memory is reserved for [PHY init data](https://en.m.wikipedia.org/w/index.php?title=Physical_layer&diffonly=true#PHY) and [NVS](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/storage/nvs_flash.html?highlight=nvs) (read more [here](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-guides/partition-tables.html#built-in-partition-tables)).

## Stability
__Latest verified stable version: 1.1.5__

A version is deemed "stable" if it runs without interruptions/buggy behaviour for at least 1 month.

## Caveats
- If you're planning to flash the firmware and use it "in production", you should always use release builds. Just pass `--release` to `cargo build` **and** `cargo espflash`.
- For troubleshooting, you should use debug builds, as they have more verbose logging.
- Make sure to use the given partition layout ([`partitions.csv`](partitions.csv)) by passing `--partition-table partitions.csv` to `cargo espflash`. The default partition layout has a way too small `app` partition.
- Some lower-quality ESP32 clones and USB cables may require a lower baud rate. Use `115200` if `921600` does not work for you.

## WIP Features
- [ ] OTA firmware updates
    - Experiemental support is being worked on in the `experimental-ota` branch.

## Emulation
You can download prebuilt binaries of Espressif's QEMU fork from [here](https://github.com/espressif/qemu/releases). However as of now, PWOS cannot be emulated. You will get a panic on boot. This is likely due to the emulator not being able to emulate the WiFi hardware.