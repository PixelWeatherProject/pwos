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
    match info.location() {
        Some(loc) => log::error!("Location: {loc}"),
        None => log::error!("Location: N/A"),
    }
    log::error!("====================[PANIC]====================");
}
