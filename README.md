# PixelWeatherOS
This is a universal firmware for all PixelWeather nodes. It was created using the [esp-idf template](https://github.com/esp-rs/esp-idf-template).

PixelWeather is a weather station network that collects environment data using "nodes" (a collection of microcontrollers and sensors). This repository contains the firware for said nodes _(PWOS)_.

**⚠️WARNING⚠️: You are currently viewing the v2 branch, which is an overhaul of v1, containing many significant changes - including updated hardware support!**

**⚠️ Note that this project is under development. While it is decently stable, is not complete! There are missing and incomplete implementations of features. Production use is highly discouraged!**

### Hardware requirements:
- Espressif ESP32-S3 microcontroller
    - Classic ESP32 and ESP32-C3 series are **no longer supported**!
    - 4MB Flash **minimum**, 8MB **recommended**, 16MB **best**
        - Future versions may drop support for 4MB models. It's advised to use 8MB models for future-proofing.
        - Read section [Build variants](#build-variants) for details
    - 512KB SRAM (models with less may be sufficient)
    - PSRAM **not** required, it's not used (yet)
- 2x resistors for measuring battery voltage. Exact values are defined in [`battery.rs`](src/sysc/battery.rs) - `DIVIDER_R1` and `DIVIDER_R2`.
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

## Recommended ESP32 boards
As of now, this firmware has been tested with:
- [x] [LILYGO T7 S3 v1.2](https://lilygo.cc/products/t7-s3)
- [x] [Arduino Nano ESP32](https://store.arduino.cc/en-sk/products/nano-esp32)
- [x] [Seeed Studio XIAO ESP32S3](https://wiki.seeedstudio.com/xiao_esp32s3_getting_started/)

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
| **Board** | **Sensor** | **Test voltage** | **Running** | **Sleeping** | **Peak** | **Notes** |
| --------- | ---------- | ---------------- | ----------- | ------------ | -------- | --------- |
| N/A       | N/A        | N/A              | N/A         | N/A          | N/A      | N/A       |

Battery life measurements:
| **Board** | **Sensor** | **Battery model** | **Capacity** | **Environment** | **Sleep time** | **Time** |
| --------- | ---------- | ----------------- | ------------ | --------------- | :------------: | :------: |
| N/A       | N/A        | N/A               | N/A          | N/A             |      N/A       |   N/A    |

**Note that the battery voltage measurement is currently unreliable.**

## Building
1. Make sure that `sdkconfig.debug` and `sdkconfig.release` are correct for your specific board.
2. Check if the firmware uses the correct GPIO pins for I2C and on-board LED.
3. Follow the toolchain setup in [Espressifs Rust Book](https://esp-rs.github.io/book/)
4. Create a custom `sys.rs` config using the [example](src/config/sys.rs.example).
5. Use `cargo build` to compile the firmware.
6. Use the commands below to build an image or flash the firmware.

If you just want to build the image, use the following command (for example):
```sh
cargo espflash save-image -T partitions.csv --frozen --locked --release --chip esp32s3 --merge image.bin 
```

To directly flash the firmware, use the command below. **Remember to change the serial port for your machine.**
```sh
cargo espflash flash -T partitions.csv --frozen --locked --release -c esp32s3 --noverify --erase-data-parts otadata -B 921600 -p /dev/ttyXXXX
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

To build a debug image (or flash it) remove the `--release` flag from the above commands.

### Board-specific configuration
<details>
  <summary>LILYGO T7 S3 v1.2</summary>
  
  ### ESP SDK configuration 
  The provided `sdkconfig.debug` and `sdkconfig.release` configurations are designed for this board by default.
  No changes are needed.

  ### GPIO Pins
  The default pin configuration of PWOS is designed for this board. No changes are needed.
  
  - On-board LED: `GPIO_17`
  - I2C SDA: `GPIO_5`
  - I2C SCL: `GPIO_8`
  - Battery measurement: `GPIO_2`
  
  ### `espflash` commands
  - For saving as image:
    - `cargo espflash save-image --frozen --locked --release -T partitions.csv -s 16mb --chip esp32s3 image.bin`
  - For flashing:
    - `cargo espflash flash --frozen --locked --release -T partitions.csv -s 16mb -c esp32s3 -B 921600 -p /dev/ttyXXXX -M --no-verify --erase-data-parts ota`
</details>

<details>
  <summary>Arduino Nano ESP32</summary>
  
  ### ESP SDK configuration 
  In both `sdkconfig.debug` and `sdkconfig.release` uncomment/add the following entries:
  ```
  CONFIG_RTC_CLK_SRC_EXT_CRYS=y
  ```

  ### GPIO Pins
  The on-board LED is on a different pin. You'll need to set `LED_BUILTIN` in your sysconfig ([src/config/sys.rs](src/config/sys.rs)).
  - On-board LED: `GPIO_48`
  - I2C SDA: `GPIO_5`
  - I2C SCL: `GPIO_8`
  - Battery measurement: `GPIO_2`
  
  ### `espflash` commands
  - For saving as image:
    - `cargo espflash save-image --frozen --locked --release -T partitions.csv -s 16mb --chip esp32s3 image.bin`
  - For flashing:
    - `cargo espflash flash --frozen --locked --release -T partitions.csv -s 16mb -c esp32s3 -B 921600 -p /dev/ttyXXXX -M --no-verify --erase-data-parts ota`
</details>

<details>
  <summary>Seeed Studio XIAO ESP32S3</summary>
  
  ### ESP SDK configuration 
  The provided `sdkconfig.debug` and `sdkconfig.release` configurations are designed for this board by default.
  No changes are needed.

  ### GPIO Pins
  The on-board LED is on a different pin. You'll need to set `LED_BUILTIN` in your sysconfig ([src/config/sys.rs](src/config/sys.rs)). Additionally, you'll also need to set `LED_BUILTIN_INVERT` to `true`, because the LED's negative terminal is connected to the GPIO pin.
  
  - On-board LED: `GPIO_21`
  - I2C SDA: `GPIO_5`
  - I2C SCL: `GPIO_8`
  - Battery measurement: `GPIO_2`
  
  ### `espflash` commands
  - For saving as image:
    - `cargo espflash save-image --frozen --locked --release -T partitions.csv -s 8mb --chip esp32s3 image.bin`
  - For flashing:
    - `cargo espflash flash --frozen --locked --release -T partitions.csv -s 8mb -c esp32s3 -B 921600 -p /dev/ttyXXXX -M --no-verify --erase-data-parts ota`
</details>

## Build variants
Firmware size (as of commit `c42f7cf`):
- Release build: `1,120,640/2,031,616 bytes, 55.16%`
- Debug build: `1,233,760/2,031,616 bytes, 60.73%`

Debug builds may be slower and contain a lot of debug messages. As such they are ~2% larger.

Some parts of the flash memory are reserved for other data then PWOS itself. 12KB are reserved for [NVS](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/storage/nvs_flash.html?highlight=nvs) storage. Read more [here](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-guides/partition-tables.html#built-in-partition-tables).

## Stability
__Latest verified stable version: N/A__

A version is deemed "stable" if it runs without interruptions/buggy behaviour for at least 1 month.

## Caveats
- If you're planning to flash the firmware and use it "in production", you should always use release builds. Just pass `--release` to `cargo build` **and** `cargo espflash`.
- For troubleshooting, you should use debug builds, as they have more verbose logging.
- Make sure to use the given partition layout ([`partitions.csv`](partitions.csv)) by passing `-T partitions.csv` to `cargo espflash`. The default partition layout has a way too small `app` partition.
- Some lower-quality USB cables may require a lower baud rate. Use `115200` if `921600` does not work for you.
- The firmware does **not** support unencrypted WiFi networks (at least not without modifying [`src/firmware.rs`](src/firmware.rs) and [`src/sysc/net/wifi.rs`](src/sysc/net/wifi.rs)).
- By default, the firmware sets the WiFi [power saving mode](https://docs.espressif.com/projects/esp-idf/en/v5.2.2/esp32s3/api-guides/wifi.html#station-sleep) to minimum. **This may change in future versions**, as the goal is to make it work with *maximum* power savings.

## Terms
- *node* - A station that consists of PWOS-compatible hardware and runs PWOS. It collects weather information and sends it over PWMP to a remote server.
- *sysconfig*/*system configuration* - Board-specific configuration with pin definitions. Should be in `src/config.sys.rs`. For an example configuration, check [`src/config/sys.rs.example`](src/config/sys.rs.example)
- *appconfig*/*application configuration* - Defines how PWOS behaves, e.g. whether it should check battery voltages, how long should the node sleep, etc. This configuration is defined in the PWMP database.
- *sBOP*/*software-based battery overdischarge protection* - Permanently shuts down the node if the battery voltage drops below a critical value.
- *OTA*/*Over-the-Air (updates)* - Firmware updates that are delivered wirelessly to the nodes.

## WIP Features
- [x] OTA Updates
- [ ] Storing secrects in NVS instead of in the code
  - [ ] NVS encryption
- [ ] Flash encryption
- [x] USB connection detection
  - Serial port detection can't be implemented yet due to API limitations of ESP IDF v5.2.2

## Emulation
You can download prebuilt binaries of Espressif's QEMU fork from [here](https://github.com/espressif/qemu/releases). However as of now, PWOS cannot be emulated. You will get a panic on boot. This is likely due to the emulator not being able to emulate the WiFi hardware.