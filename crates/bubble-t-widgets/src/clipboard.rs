//! System clipboard access via [`arboard`](https://docs.rs/arboard).

#[cfg(feature = "clipboard-support")]
pub fn read_text() -> Result<String, String> {
    arboard::Clipboard::new()
        .map_err(|e| format!("Failed to create clipboard context: {e}"))?
        .get_text()
        .map_err(|e| format!("Failed to read clipboard: {e}"))
}

#[cfg(feature = "clipboard-support")]
pub fn write_text(text: &str) -> Result<(), String> {
    arboard::Clipboard::new()
        .map_err(|e| format!("Failed to create clipboard context: {e}"))?
        .set_text(text.to_owned())
        .map_err(|e| format!("Failed to write to clipboard: {e}"))
}
