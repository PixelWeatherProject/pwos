# Dynamic Driver Loader System (DDLS)

This subsystem provides an API to PWOS that allows it to detect connected peripherals to the MCU and automatically load the correct driver for it.

It is similar to how Linux, macOS and Windows automatically load drivers, however DDLS is much more simple and limited, due to the inherent hardware limitations.

The subsystem further handles safe sharing of driver instances. For every peripheral there can only be 1 loaded driver. Once a peripheral is initialized, the driver cannot be changed and will remain the same for the lifetime of the firmware. Every instantiated driver is wrapped in a `SharedDriver` structure, which safely protects the driver when being used multiple times by multiple callers. It is similiar to `Arc<Mutex<T>>`. DDLS only handles 1 device for every peripheral type, with the exception of temperature sensors.

Each peripheral has a globally available structure that can be instantiated anywhere in the firmware, allowing easy access to whatever peripheral it needs. These structs will be referred to as "Peripheral Accessors" or "Peripheral Structs."

## Protocol support
For now, only I2C is supported.

### I2C autoloading
Because I2C does not have a standardized way to exchange device metadata such as the manufacturer and device model (unlike USB for eg.), it's impossible to know what device is behind it.
There are only 2 solutions:
- brute-force the driver: Attempt to load every driver on an unknown device and see which one initializes successfully,
- map known addresses to specific drivers: Every driver will have an array of known addresses and DDLS can use that.

Brute-force is not a good option, as two sensors may share a similar set of commands, but they can be completely different. Additionally some sensors liek Bosh's, like to lock up when you send a wrong command.

Mapping is the best option for here, at least for this project. It has it's own limitations, but statistically, it's more likely to be successful.

## Driver store
When the subsystem detects a device and successfully loads the correct driver, it will place a shared reference of the driver into a global "storage."
Dedicated structures for every device type will then attempt to fetch that driver from the global store.

The driver store only stores a reference to the driver's `*Driver` trait implementation, it does not care about what exact driver it is.
For example a driver like [`Htu`](src/ext_drivers/htu.rs) is only stored as a `Arc<dyn TemperatureDriver>` (equivalent to a `Box<dyn ...>` but with reference counting). The store knows that the object
implements `TemperatureDriver`, but it does not know that [`Htu`](src/ext_drivers/htu.rs) is behind it, nor does it need to.

### Temperature Sensors
DDLS provides a globally available `TemperatureSensor` struct, that allows access to whatever temperature sensor is available.
All temperature measurements use Celsius as units for temperature.

Because the ESP32S3 has a built-in temperature sensor, this struct can also be used to access it.

**Note that DDLS's `TemperatureDriver` uses <u>decidegrees</u> Celsius, to avoid floating point logic.**

#### Example using the internal temperature sensor
```rust
// The internal sensor is always available, so optional is returned.
let sensor = TemperatureSensor::get_internal();

let temp_c = sensor.read().unwrap();
println!("{temp_c}");
```

#### Example using an external temperature sensor
Since the external sensor may not be connected, or could've failed to load, `get_external()` returns an optional.
```rust
let sensor = TemperatureSensor::get_external().unwrap();

let temp_dgc = sensor.read().unwrap();
println!("{temp_dgc}");
```

### Humidity Sensors
DDLS provides a globally available `HumiditySensor` struct, that allows access to whatever hudmidity sensor is available.
All humidity measurements use relative humidity percentage (RH %) as units for humidity. Unlike temperature sensors, there is only one external humidity sensor, so you call `get()`.

```rust
let sensor = HumiditySensor::get_external().unwrap();

let hum_rh = sensor.read().unwrap();
println!("{hum_rh}");
```