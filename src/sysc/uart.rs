use super::OsResult;
use esp_idf_svc::sys::{esp, uart_wait_tx_idle_polling, CONFIG_ESP_CONSOLE_UART_NUM};

pub fn flush() -> OsResult<()> {
    esp!(unsafe { uart_wait_tx_idle_polling(CONFIG_ESP_CONSOLE_UART_NUM) })?;
    Ok(())
}
