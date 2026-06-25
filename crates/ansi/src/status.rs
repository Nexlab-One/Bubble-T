//! Device status report (DSR) and cursor position report builders.

pub use crate::cursor::REQUEST_CURSOR_POSITION_REPORT;

/// Requests extended cursor position including page (`CSI ? 6 n`).
pub const REQUEST_EXTENDED_CURSOR_POSITION_REPORT: &str = "\x1b[?6n";

/// Requests terminal light/dark preference (`CSI ? 996 n`).
pub const REQUEST_LIGHT_DARK_REPORT: &str = "\x1b[?996n";

/// ANSI status report identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnsiStatusReport(pub i32);

impl AnsiStatusReport {
    /// Returns the numeric status code.
    pub fn code(self) -> i32 {
        self.0
    }
}

/// DEC private status report identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecStatusReport(pub i32);

impl DecStatusReport {
    /// Returns the numeric status code.
    pub fn code(self) -> i32 {
        self.0
    }
}

/// Status report target for [`device_status_report`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusReport {
    /// ANSI status code.
    Ansi(AnsiStatusReport),
    /// DEC private status code.
    Dec(DecStatusReport),
}

impl StatusReport {
    fn code(self) -> i32 {
        match self {
            Self::Ansi(s) => s.code(),
            Self::Dec(s) => s.code(),
        }
    }

    fn is_dec(self) -> bool {
        matches!(self, Self::Dec(_))
    }
}

/// Builds a device status report request (`CSI Ps n` / `CSI ? Ps n`).
pub fn device_status_report(reports: &[StatusReport]) -> String {
    if reports.is_empty() {
        return String::new();
    }
    let dec = reports.iter().any(|r| r.is_dec());
    let codes: Vec<String> = reports.iter().map(|r| r.code().to_string()).collect();
    if dec {
        format!("\x1b[?{}n", codes.join(";"))
    } else {
        format!("\x1b[{}n", codes.join(";"))
    }
}

/// Builds a cursor position report response (`CSI Pl ; Pc R`).
pub fn cursor_position_report(line: i32, column: i32) -> String {
    let line = line.max(1);
    let column = column.max(1);
    format!("\x1b[{line};{column}R")
}

/// Builds an extended cursor position report (`CSI ? Pl ; Pc ; Pp R`).
pub fn extended_cursor_position_report(line: i32, column: i32, page: i32) -> String {
    let line = line.max(1);
    let column = column.max(1);
    if page < 1 {
        format!("\x1b[?{line};{column}R")
    } else {
        format!("\x1b[?{line};{column};{page}R")
    }
}

/// Builds a light/dark preference report (`CSI ? 997 ; mode n`).
pub fn light_dark_report(dark: bool) -> String {
    if dark {
        "\x1b[?997;1n".to_string()
    } else {
        "\x1b[?997;2n".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dsr_dec() {
        assert_eq!(
            device_status_report(&[StatusReport::Dec(DecStatusReport(6))]),
            "\x1b[?6n"
        );
    }

    #[test]
    fn light_dark_builder() {
        assert_eq!(light_dark_report(true), "\x1b[?997;1n");
    }
}
