# Dynamic Driver Loader System (DDLS)

This subsystem provides an API to PWOS that allows it to detect connected peripherals to the MCU and automatically load the correct driver for it.

It is similar to how Linux, macOS and Windows automatically load drivers, however DDLS is much more simple and limited, due to the inherent hardware limitations.

The subsystem further handles safe sharing of driver instances. For every peripheral there can only be loaded 1 driver. Once a peripheral is initialized, the driver cannot be changed and will remain the same for the lifetime of the firmware. Every instantiated driver is wrapped in a `SharedDriver` structure, which safely protects the driver when being used multiple times by multiple callers. It is similiar to `Arc<Mutex<T>>`. DDLS only handles 1 device for every peripheral type, with the exception of temperature sensors.

Each peripheral has a globally available structure that can be instantiated anywhere in the firmware, allowing easy access to whatever peripheral it needs. These structs are referred to as "Peripheral Accessors."

### Temperature Sensors
DDLS provides a globally available `TemperatureSensor` struct, that allows access to whatever temperature sensor is available.
All temperature measurements use Celsius as units for temperature.

Because the ESP32S3 has a built-in temperature sensor, this struct can also be used to access it.

#### Example using the internal temperature sensor
```rust
// The internal sensor is always available, so optional is returned.
let sensor = TemperatureSensor::get_internal();

let temp_c = sensor.read().unwrap();
println!("{temp_c}");
```

#### Example using an external temperature sensor
```rust
// The sensor may not be available, so this returns an `Option<TemperatureSensor>`.
let sensor = TemperatureSensor::get_external().unwrap();

let temp_c = sensor.read().unwrap();
println!("{temp_c}");
```

### Humidity Sensors
DDLS provides a globally available `HumiditySensor` struct, that allows access to whatever hudmidity sensor is available.
All humidity measurements use relative humidity percentage (RH %) as units for humidity. Unlike temperature sensors, there is only one external humidity sensor, so you call `get()`.

```rust
let sensor = HumiditySensor::get_external().unwrap();

let hum_rh = sensor.read().unwrap();
println!("{hum_rh}");
```