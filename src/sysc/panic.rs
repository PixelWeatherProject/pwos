use std::panic::PanicHookInfo;

/// Sets a custom panic hook as the global one.
pub fn setup() {
    std::panic::set_hook(Box::new(handle_panic));
}

fn handle_panic(info: &PanicHookInfo) {
    let payload = info.payload_as_str().unwrap_or("N/A");

    log::error!("====================[PANIC]====================");
    log::error!("Firmware paniced!");
    log::error!("Message: {payload}");
    log::error!(
        "Location: {}",
        info.location()
            .map_or_else(|| "N/A".to_string(), ToString::to_string)
    );
    log::error!("====================[PANIC]====================");
}
