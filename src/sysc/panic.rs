use crate::os_error;
use std::panic::PanicHookInfo;

pub fn setup() {
    std::panic::set_hook(Box::new(handle_panic));
}

fn handle_panic(info: &PanicHookInfo) {
    let payload = info.payload_as_str().unwrap_or("N/A");

    os_error!("====================[PANIC]====================");
    os_error!("Firmware paniced!");
    os_error!("Message: {payload}");
    os_error!(
        "Location: {}",
        info.location()
            .map_or_else(|| "N/A".to_string(), ToString::to_string)
    );
    os_error!("====================[PANIC]====================");
}
