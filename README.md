# PixelWeatherOS
This is a universal firmware for all PixelWeather nodes. It was created using the [esp-idf template](https://github.com/esp-rs/esp-idf-template).

PixelWeather is a weather station network that collects environment data using "nodes" (a collection of microcontrollers and sensors). This repository contains the firware for said nodes _(PWOS)_.

**⚠️ Note that this project is under development. While it is decently stable, is not complete!**

### Hardware requirements:
- Espressif ESP32-S3 microcontroller
    - Classic ESP32 and ESP32-C3 series are **no longer supported**!
    - 8MB Flash **minimum**
        - 4MB models are **not** supported.
        - Read section [Build variants](#build-variants) for details
    - 512KB SRAM
    - PSRAM required due to build configuration
- 2x resistors for measuring battery voltage. Exact values are defined in [`battery.rs`](src/sysc/battery.rs) - `R1` and `R2`.
- Battery - any generic 18650 will do
    - Additional protection circuit recommended
- An environment sensor
    - Temperature reading support (**required**)
    - Humidity reading support (**required**)
    - _Optional_:
        - Air pressure reading support
    - I2C interface

### Software requirements (for building):
- [Rust](https://rustlang.org/)
- [ESP32 Rust toolchain](https://esp-rs.github.io/book/)
  - Version `1.84.0.0` is **recommended**.
    - `espup install --toolchain-version 1.84.0.0`
- An instance of the PixelWeather Messaging Protocol (PWMP) server.
  - Repository [here](https://github.com/PixelWeatherProject/pwmp-server).

### Recommended hardware
For a generally stable, safe and reliable experience, you should stick to reputable a higher-quality brands. Below are the listed recommendations for all categories of hardware.

#### ESP32S3 boards
As of now, this firmware has been tested with:
- [x] [LILYGO T7 S3 v1.2](https://lilygo.cc/products/t7-s3)
  - ⭐ Recommended
- [x] [Arduino Nano ESP32](https://store.arduino.cc/en-sk/products/nano-esp32)
- [x] [Seeed Studio XIAO ESP32S3](https://wiki.seeedstudio.com/xiao_esp32s3_getting_started/)
  - ⚠️ Not recommended, see board-specific details.

It's recommended to use hardware from reputable brands such as Adafruit, SparkFun, DFRobot, etc. These are generally more expensive but also higher quality.

#### Environment sensors
As of now, this firmware has been tested with:
- [Adafruit Si7021 Temperature & Humidity Sensor](https://www.adafruit.com/product/3251)
- [HTU21D from SparkFun](https://www.sparkfun.com/products/retired/12064)

#### Recommended batteries
> **⚠️WARNING⚠️**
>
> Lithium-ion batteries can be *highly dagerous*, **explosive** and a *fire hazard*!
> Handle them with care!
>
> It's recommended that you use models **with built-in protection**. Note that this does **not make them completely safe**.

- [x] [XTAR 18650 4000mAh (protected) - 10A](https://www.nkon.nl/en/xtar-18650-4000mah-protected-10a.html)
  - ⭐ Recommended - high capacity, legitimate brand and built-in protection

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
| **Board**        | **Sensor**      | **Test voltage**     | **Running** | **Sleeping** | **Peak**            | **Notes**   |
| ---------------- | --------------- | -------------------- | ----------- | ------------ | ------------------- | ----------- |
| LILYGO T7S3 v1.2 | Adafruit Si7021 | 5V<sup>*</sup> (USB) | ~140mA      | 0.75mA       | N/A                 | N/A         |
| LILYGO T7S3 v1.2 | Adafruit Si7021 | 4.2V<sup>**</sup>    | 112mA       | 904μA        | 438mA<sup>***</sup> | Using PPKII |

<details>
  <summary>Notes</summary>
  
  - `*`: There seems to be a large voltage drop from the USB connector. The measured voltage on the 5V was *4.352V*.
  - `**`: Powered through the 5V *output* pin, using a [Nordic Semiconductor Power Profiler Kit II](https://www.nordicsemi.com/Products/Development-hardware/Power-Profiler-Kit-2).
  - `***`: Only during WiFi communication
</details>

Battery life measurements:
| **Board**        | **Sensor**      | **Battery model** | **Capacity** | **Environment**  | **Sleep time** |  **Time**  |
| ---------------- | --------------- | ----------------- | ------------ | ---------------- | :------------: | :--------: |
| LILYGO T7S3 v1.2 | Adafruit Si7021 | XTAR 18650        | 4Ah          | Outdoor (2-21°C) |       1m       | 6d/22h/17m |
| LILYGO T7S3 v1.2 | Adafruit Si7021 | XTAR 18650        | 4Ah          | Outdoor (2-21°C) |       1m       | 7d/8h/23mi |

The default battery voltage measurement configuration has a measured inaccuracy of ±2-6mV. The inaccuracy is higher at higher input voltages, which is to be expected due to the ESP32S3's ADC not being fully linear.

## Building
1. Make sure that `sdkconfig.debug` and `sdkconfig.release` are correct for your specific board.
2. Check if the firmware uses the correct GPIO pins for I2C and on-board LED.
3. Follow the toolchain setup in [Espressifs Rust Book](https://esp-rs.github.io/book/)
4. Create a custom `sys.rs` config using the [example](src/config/sys.rs.example).
5. Use `cargo build` to compile the firmware.
6. Use the commands below to build an image or flash the firmware.

If you just want to build the image, use the following command (for example):
```sh
cargo espflash save-image --features $BOARD -T partitions.csv --frozen --locked --release --chip esp32s3 --merge image.bin 
```

To directly flash the firmware, use the command below. **Remember to change the serial port for your machine.**
```sh
cargo espflash flash --features $BOARD -T partitions.csv --frozen --locked --release -c esp32s3 --noverify --erase-data-parts otadata -B 921600 -p /dev/ttyXXXX
```

If you notice weird/buggy bevaiour, you can erase the entire flash like so:
```sh
cargo espflash erase-flash -c esp32s3 -p /dev/ttyACM0 -B 921600
```
<details>
  <summary>⚠️ Note for Arduino Nano ESP32</summary>
  
  After erasing the flash you may not be able to flash the board with Arduino IDE. You'll need to re-burn the bootloader.
</details>

### Additional arguments
Depending on which ESP32S3 development board you're using, you may need to add additional arguments to the two example commands above (especially `flash`).
- `-c esp32s3`
- `-s 16mb` / `-s 8mb` - For 16MB and 8MB of flash respectively.
- `--features` - Check section below.

To build a debug image (or flash it) remove the `--release` flag from the above commands.

### Board-specific configuration
<details>
  <summary>LILYGO T7 S3 v1.2</summary>

  ## Note
  This board has performed well during tests, even when powered over the 5V *output* pin.
  
  ### ESP SDK configuration 
  The provided `sdkconfig.debug` and `sdkconfig.release` configurations are designed for this board by default.
  No changes are needed.

  ### GPIO Pins
  The default pin configuration of PWOS is designed for this board. No changes are needed. You may also leave out the `--features` flag for `espflash`.
  
  - On-board LED: `GPIO_17`
  - I2C SDA: `GPIO_5`
  - I2C SCL: `GPIO_8`
  - Battery measurement: `GPIO_2`
  
  ### `espflash` commands
  - For saving as image:
    - `cargo espflash save-image --features lilygo-t7s3 --frozen --locked --release -T partitions.csv -s 16mb --chip esp32s3 image.bin`
  - For flashing:
    - `cargo espflash flash --features lilygo-t7s3 --frozen --locked --release -T partitions.csv -s 16mb -c esp32s3 -B 921600 -p /dev/ttyXXXX -M --no-verify --erase-data-parts ota`
</details>

<details>
  <summary>Arduino Nano ESP32</summary>
  
  ### ESP SDK configuration 
  In both `sdkconfig.debug` and `sdkconfig.release` uncomment/add the following entries:
  ```
  CONFIG_RTC_CLK_SRC_EXT_CRYS=y
  ```

  ### GPIO Pins
  The on-board LED is on a different pin.
  - On-board LED: `GPIO_48`
  - I2C SDA: `GPIO_5`
  - I2C SCL: `GPIO_8`
  - Battery measurement: `GPIO_2`
  
  ### `espflash` commands
  - For saving as image:
    - `cargo espflash save-image --features arduino-nano-esp32 --frozen --locked --release -T partitions.csv -s 16mb --chip esp32s3 image.bin`
  - For flashing:
    - `cargo espflash flash --features arduino-nano-esp32 --frozen --locked --release -T partitions.csv -s 16mb -c esp32s3 -B 921600 -p /dev/ttyXXXX -M --no-verify --erase-data-parts ota`
</details>

<details>
  <summary>Seeed Studio XIAO ESP32S3</summary>

  ### Note
  This board has **not** performed well during tests. It had many connectivity issues as well as many watchdog resets for an unknown reason, even if it was very close to an AP. This behavior did not change regardless if the board was powered over the 5V *output* pin, nor the intended battery input. It's **not** recommended to use this board.
  
  ### ESP SDK configuration 
  The provided `sdkconfig.debug` and `sdkconfig.release` configurations are designed for this board by default.
  No changes are needed.

  ### GPIO Pins
  The on-board LED is on a different pin, and its negative terminal is connected to the GPIO pin, meaning it works with inverted logic.
  
  - On-board LED: `GPIO_21`
  - I2C SDA: `GPIO_5`
  - I2C SCL: `GPIO_8`
  - Battery measurement: `GPIO_2`
  
  ### `espflash` commands
  - For saving as image:
    - `cargo espflash save-image --features xiao-s3 --frozen --locked --release -T partitions.csv -s 8mb --chip esp32s3 image.bin`
  - For flashing:
    - `cargo espflash flash --features xiao-s3 --frozen --locked --release -T partitions.csv -s 8mb -c esp32s3 -B 921600 -p /dev/ttyXXXX -M --no-verify --erase-data-parts ota`
</details>

## Build variants
Firmware size (as of commit `0b5441d`):
- Release build: `767,568/4,096,000 bytes, 18.74%`
- Debug build: `1,098,592/4,096,000 bytes, 26.82%`

Debug builds may be slower and contain a lot of debug messages. As such they are slightly larger.

Some parts of the flash memory are reserved for other data than PWOS itself. 16KB are reserved for [NVS](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/storage/nvs_flash.html?highlight=nvs) storage. Read more [here](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-guides/partition-tables.html#built-in-partition-tables).

## Stability
__Latest verified stable version: N/A__

A version is deemed "stable" if it runs without interruptions/buggy behaviour for at least 1 month.

## Example logs from runs

<details>
  <summary>Release build</summary>

  ```
  INFO  [pwos] PixelWeatherOS v2.0.2-b8c53b3-devel (04.04.2025 08:46:51)
  INFO  [pwos] (C) Fábián Varga 2025
  INFO  [pwos] Staring main
  WARN  [pwos::firmware] Got empty node settings, using defaults
  WARN  [pwos::firmware] Battery voltage measurement may be affected by USB power
  INFO  [pwos::firmware] Battery: 0.43V
  INFO  [pwos::firmware] 22.83*C / 62%
  INFO  [pwos::firmware] No update available
  INFO  [pwos] Tasks completed successfully
  INFO  [pwos] Tasks completed in 4.45s
  ```
</details>

<details>
  <summary>Debug build</summary>

  ```
  INFO  [pwos] PixelWeatherOS v2.0.2-b8c53b3-devel (04.04.2025 08:59:00)
  INFO  [pwos] (C) Fábián Varga 2025
  DEBUG [pwos] Using ESP-IDF v5.3.2
  DEBUG [pwos] Disabling brownout detector
  DEBUG [pwos] Initializing system peripherals
  DEBUG [pwos::sysc::periph] Initializing base peripherals
  DEBUG [pwos::sysc::periph] Initializing System Event Loop
  DEBUG [pwos] Initializing system LED
  DEBUG [pwos] Setting panic handle
  DEBUG [pwos] Initializing OTA system
  DEBUG [pwos] Reported current version: 2.0.1
  DEBUG [pwos] Previous installed version: ?
  DEBUG [pwos] Initializing system Battery
  DEBUG [pwos] Initializing I2C bus
  DEBUG [pwos] Initializing app configuration
  INFO  [pwos] Staring main
  DEBUG [pwos::firmware] Starting WiFi setup
  DEBUG [pwos::firmware] Initializing WiFi
  DEBUG [pwos::sysc::net::wifi] Configuring WiFi interface
  DEBUG [pwos::sysc::net::wifi] Starting WiFi interface
  DEBUG [pwos::sysc::net::wifi] Setting country code
  DEBUG [pwos::firmware] Starting WiFi scan
  DEBUG [pwos::firmware] Found networks: ["REDACTED"] in 2.50s
  DEBUG [pwos::firmware] Connecting to REDACTED (-50dBm)
  DEBUG [pwos::sysc::net::wifi] Starting connection to AP
  DEBUG [pwos::sysc::net::wifi] Waiting for connection result
  DEBUG [pwos::sysc::net::wifi] Waiting for IP address
  DEBUG [pwos::firmware] Connected in 1.39s
  DEBUG [pwos::firmware] IP: 192.168.0.199
  DEBUG [pwos::firmware] Connecting to PWMP
  DEBUG [pwos::firmware] Sending handshake request
  DEBUG [pwos::firmware] Requesting app configuration
  DEBUG [pwos::firmware] Reading settings
  WARN  [pwos::firmware] Got empty node settings, using defaults
  DEBUG [pwos::firmware] Settings updated
  WARN  [pwos::firmware] Battery voltage measurement may be affected by USB power
  INFO  [pwos::firmware] Battery: 0.42V
  DEBUG [pwos::firmware] Found device @ I2C/0x40
  DEBUG [pwos::firmware] Detected HTU-compatible sensor
  DEBUG [pwos::sysc::ext_drivers::htu] Loading driver
  WARN  [pwos::sysc::ext_drivers::htu] Air pressure is not supported
  INFO  [pwos::firmware] 23.10*C / 59%
  DEBUG [pwos::firmware] Posting measurements
  DEBUG [pwos::firmware] Posting stats
  DEBUG [pwos::firmware] Reset reason (USBPeripheral) is normal
  DEBUG [pwos::firmware] No error detected from previous run
  DEBUG [pwos::sysc::ota] Skipping report check on verified firmware
  DEBUG [pwos::firmware] No update report needed
  DEBUG [pwos::firmware] Checking for updates
  INFO  [pwos::firmware] No update available
  DEBUG [pwos::sysc::net::wifi] Deinitializing WiFi
  INFO  [pwos] Tasks completed successfully
  INFO  [pwos] Tasks completed in 4.56s
  DEBUG [pwos] Sleeping for 60s
  DEBUG [pwos] Using fake sleep instead of deep sleep
  ```
</details>

## Caveats
This section contains information about the current and possible limitations of this firmware. If you are having issues, you should read this.

### Building/Compilation/Deployment
- If you're planning to flash the firmware and use it "in production", you should always use release builds. Just pass `--release` to `cargo build` **and** `cargo espflash`.
- For troubleshooting, you should use debug builds, as they have more verbose logging.
- Make sure to use the given partition layout ([`partitions.csv`](partitions.csv)) by passing `-T partitions.csv` to `cargo espflash`. The default partition layout has a way too small `app` partition.
- Some lower-quality USB cables may require a lower baud rate. Use `115200` if `921600` does not work for you.

### General
- The maximum battery voltage (with the default resistor values in [`src/sysc/battery.rs`](src/sysc/battery.rs)) should be `969.23mV`.
- If you change the default resistor values, make sure to also adjust the ADC attenuation value [accordingly](https://docs.espressif.com/projects/esp-idf/en/v4.4/esp32s3/api-reference/peripherals/adc.html#adc-attenuation).
- While the order in which you connect the `R1` and `R2` resistors (for measuring battery voltage) **matters**, PWOS will detect this and auto-correct the measurement. **It is however recommended that you fix this to prevent potential damage to your MCU.**

### WiFi/Networking/Connectivity
- Hidden WiFi networks are **not** supported.
- Unencrypted WiFi networks are **not** supported.
- It's recommended to ensure that the RSSI (signal strength) is no less than *-70dBm*. Some boards can handle worse scenarios, but others may experience connectivity issues.
- WiFi credentials are stored in code, instead of NVS because it's design is way too simple and limited to properly store the kind of configuration PWOS needs. This would require some hacky workarounds, and adjusting the OTA system to allow updating the credentials.
- When scanning for APs, the firmware uses the default scan configuration if ESP-IDF. This configuration has minimal enough to severely limit the maximum scan duration to preserve as much power as possible. However, this comes at a cost - you AP/s might not be detected fast enough. If this is a problem for you, you can try the following:
  1. Lower the [*Beacon Interval*](https://www.7signal.com/news/blog/controlling-beacons-boosts-wi-fi-performance) in your AP's settings. This is usually set to 100(ms), but you can lower this to (for e.g.) 50. **Don't mess with these settings if you don't know what you're doing!**
     - In OpenWRT you can find this under *Network* > *Wireless* > *Edit* (your AP) > *Advanced Settings*
     - In AsusWRT/Merlin you can find this under *Advanced Settings* > *Wireless* > *Professional* > (select 2.4GHz band if needed)
- Support for *Management Frame Protection* (*IEEE 802.11w-2009*) is disabled to improve connection times.

## Terms
- *node* - A station that consists of PWOS-compatible hardware and runs PWOS. It collects weather information and sends it over PWMP to a remote server.
- *sysconfig*/*system configuration* - Board-specific configuration with pin definitions. Should be in `src/config.sys.rs`. For an example configuration, check [`src/config/sys.rs.example`](src/config/sys.rs.example)
- *appconfig*/*application configuration* - Defines how PWOS behaves, e.g. whether it should check battery voltages, how long should the node sleep, etc. This configuration is defined in the PWMP database.
- *sBOP*/*software-based battery overdischarge protection* - Permanently shuts down the node if the battery voltage drops below a critical value.
- *OTA*/*Over-the-Air (updates)* - Firmware updates that are delivered wirelessly to the nodes.

## Emulation
You can download prebuilt binaries of Espressif's QEMU fork from [here](https://github.com/espressif/qemu/releases). However as of now, PWOS cannot be emulated. You will get a panic on boot. This is likely due to the emulator not being able to emulate the WiFi hardware.
