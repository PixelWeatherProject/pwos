use crate::sysc::nvs::NonVolatileStorage;

pub fn main(nvs: &NonVolatileStorage) {
    println!(
        "Last OS error: {}",
        nvs.get_last_os_error()
            .ok()
            .flatten()
            .unwrap_or_else(|| "N/A".to_string())
    );
}
