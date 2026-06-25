//! POSIX / Windows shared memory for Kitty graphics (`t=s`).
//!
//! Creates a named shared memory object, writes the encoded image payload, and
//! returns the object name for base64 encoding in the APC sequence. The client
//! does not unlink the object; Kitty-compatible terminals read the data and
//! unlink (POSIX) or close (Windows) the object.

use std::io;

/// Prefix for randomly generated shared memory object names.
///
/// Matches the Kitty temp-file naming convention (`tty-graphics-protocol`).
pub const SHM_NAME_PREFIX: &str = "/tty-graphics-protocol-";

/// Maximum POSIX shared memory name length (Linux `SHM_NAME_MAX`).
#[cfg(unix)]
const SHM_NAME_MAX: usize = 255;

/// Number of attempts when a generated name collides.
const CREATE_RETRIES: u32 = 30;

/// Returns whether shared-memory transmission is supported on this platform.
#[must_use]
pub fn shared_memory_available() -> bool {
    cfg!(any(unix, windows))
}

/// Creates a shared memory object containing `data` and returns its name.
///
/// On POSIX the name begins with `/`. On Windows the name uses the `Local\`
/// session namespace (e.g. `Local\tty-graphics-protocol-deadbeef`).
///
/// # Errors
///
/// Returns [`ShmError`] when the platform API fails or the payload is empty.
pub fn create_with_data(data: &[u8]) -> Result<String, ShmError> {
    if data.is_empty() {
        return Err(ShmError::EmptyPayload);
    }
    if !shared_memory_available() {
        return Err(ShmError::Unsupported);
    }
    platform::create(data)
}

/// Error creating or writing a Kitty shared memory segment.
#[derive(Debug)]
pub enum ShmError {
    /// Shared memory is not supported on this platform.
    Unsupported,
    /// The payload must contain at least one byte.
    EmptyPayload,
    /// Could not allocate a unique object name after several attempts.
    NameCollision,
    /// Underlying platform error.
    Io(io::Error),
}

impl std::fmt::Display for ShmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => f.write_str("shared memory transmission is not supported"),
            Self::EmptyPayload => f.write_str("shared memory payload must not be empty"),
            Self::NameCollision => {
                f.write_str("failed to allocate a unique shared memory object name")
            }
            Self::Io(e) => write!(f, "shared memory io error: {e}"),
        }
    }
}

impl std::error::Error for ShmError {}

impl From<io::Error> for ShmError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

fn random_hex(byte_len: usize) -> String {
    let mut buf = vec![0u8; byte_len];
    if getrandom::fill(&mut buf).is_err() {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let pid = u128::from(std::process::id());
        for (i, byte) in buf.iter_mut().enumerate() {
            *byte = ((seed >> (i % 16)) ^ (pid >> (i % 8))) as u8;
        }
    }
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(unix)]
mod platform {
    use std::ffi::CString;
    use std::io;

    use libc::{O_CREAT, O_EXCL, O_RDWR, close, ftruncate, mode_t, shm_open};
    use memmap2::MmapMut;

    use super::{CREATE_RETRIES, SHM_NAME_MAX, SHM_NAME_PREFIX, ShmError, random_hex};

    pub fn create(data: &[u8]) -> Result<String, ShmError> {
        let prefix = SHM_NAME_PREFIX;
        let prefix_len = prefix.len();
        let safe_length = (prefix_len + 64).min(SHM_NAME_MAX);
        let hex_len = (safe_length - prefix_len).max(2) / 2;

        for _ in 0..CREATE_RETRIES {
            let name = format!("{prefix}{}", random_hex(hex_len));
            let c_name = CString::new(name.as_str())
                .map_err(|e| ShmError::Io(io::Error::new(io::ErrorKind::InvalidInput, e)))?;
            let fd = unsafe {
                shm_open(
                    c_name.as_ptr(),
                    O_CREAT | O_EXCL | O_RDWR,
                    mode_t::from(0o600),
                )
            };
            if fd < 0 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::AlreadyExists {
                    continue;
                }
                return Err(ShmError::Io(err));
            }

            let result = write_segment(fd, &name, data);
            unsafe {
                close(fd);
            }
            return result;
        }
        Err(ShmError::NameCollision)
    }

    fn write_segment(fd: i32, name: &str, data: &[u8]) -> Result<String, ShmError> {
        let size = i64::try_from(data.len()).map_err(|_| {
            ShmError::Io(io::Error::new(
                io::ErrorKind::InvalidInput,
                "shared memory payload too large",
            ))
        })?;
        if unsafe { ftruncate(fd, size) } != 0 {
            let err = io::Error::last_os_error();
            let _ = unlink_name(name);
            return Err(ShmError::Io(err));
        }

        let mut map = unsafe { MmapMut::map_mut(fd) }.map_err(|e| {
            let _ = unlink_name(name);
            ShmError::Io(e)
        })?;
        map.copy_from_slice(data);
        map.flush().map_err(|e| {
            let _ = unlink_name(name);
            ShmError::Io(e)
        })?;
        drop(map);

        Ok(name.to_string())
    }

    fn unlink_name(name: &str) -> io::Result<()> {
        let c_name = CString::new(name)?;
        let rc = unsafe { libc::shm_unlink(c_name.as_ptr()) };
        if rc != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn read_segment(name: &str, size: usize) -> io::Result<Vec<u8>> {
        let c_name = CString::new(name)?;
        let fd = unsafe { shm_open(c_name.as_ptr(), libc::O_RDONLY, 0) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let map = unsafe { MmapMut::map_mut(fd) }?;
        let data = map[..size.min(map.len())].to_vec();
        unsafe {
            close(fd);
        }
        Ok(data)
    }

    #[cfg(test)]
    pub(crate) fn cleanup(name: &str) {
        let _ = unlink_name(name);
    }
}

#[cfg(windows)]
mod platform {
    use std::io;
    use std::ptr;

    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Memory::{
        CreateFileMappingW, FILE_MAP_WRITE, MapViewOfFile, OpenFileMappingW, PAGE_READWRITE,
        UnmapViewOfFile,
    };

    use super::{CREATE_RETRIES, ShmError, random_hex};

    pub fn create(data: &[u8]) -> Result<String, ShmError> {
        for _ in 0..CREATE_RETRIES {
            let name = format!("Local\\tty-graphics-protocol-{}", random_hex(16));
            match try_create(&name, data) {
                Ok(()) => return Ok(name),
                Err(ShmError::Io(e))
                    if e.raw_os_error()
                        == Some(windows_sys::Win32::Foundation::ERROR_ALREADY_EXISTS as i32) =>
                {
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        Err(ShmError::NameCollision)
    }

    fn try_create(name: &str, data: &[u8]) -> Result<(), ShmError> {
        if mapping_exists(name) {
            return Err(ShmError::Io(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "shared memory name already exists",
            )));
        }
        let wide = encode_wide(name);
        let size = u32::try_from(data.len()).map_err(|_| {
            ShmError::Io(io::Error::new(
                io::ErrorKind::InvalidInput,
                "shared memory payload too large",
            ))
        })?;

        let handle = unsafe {
            CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                ptr::null(),
                PAGE_READWRITE,
                0,
                size,
                wide.as_ptr(),
            )
        };
        if handle.is_null() {
            return Err(ShmError::Io(io::Error::last_os_error()));
        }

        let view = unsafe { MapViewOfFile(handle, FILE_MAP_WRITE, 0, 0, data.len()) };
        if view.Value.is_null() {
            let err = io::Error::last_os_error();
            unsafe {
                CloseHandle(handle);
            }
            return Err(ShmError::Io(err));
        }

        unsafe {
            ptr::copy_nonoverlapping(data.as_ptr(), view.Value as *mut u8, data.len());
            UnmapViewOfFile(view);
            CloseHandle(handle);
        }
        Ok(())
    }

    fn encode_wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn mapping_exists(name: &str) -> bool {
        let wide = encode_wide(name);
        let handle = unsafe { OpenFileMappingW(FILE_MAP_WRITE, 0, wide.as_ptr()) };
        if handle.is_null() {
            return false;
        }
        unsafe {
            CloseHandle(handle);
        }
        true
    }
}

#[cfg(not(any(unix, windows)))]
mod platform {
    use super::ShmError;

    pub fn create(_data: &[u8]) -> Result<String, ShmError> {
        Err(ShmError::Unsupported)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn create_and_read_roundtrip() {
        if !shared_memory_available() {
            return;
        }
        let payload = b"hello-kitty-shm";
        let name = create_with_data(payload).expect("create shm");
        assert!(name.starts_with(SHM_NAME_PREFIX));
        let read = platform::read_segment(&name, payload.len()).expect("read shm");
        assert_eq!(read, payload);
        platform::cleanup(&name);
    }

    #[test]
    fn rejects_empty_payload() {
        assert!(matches!(create_with_data(&[]), Err(ShmError::EmptyPayload)));
    }
}
