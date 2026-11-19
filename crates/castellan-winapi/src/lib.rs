//! Safe wrappers over Win32. This is the only crate allowed to use `unsafe`.
//!
//! Destructive operations are named `guard_*` and take a [`GuardToken`]. The architecture
//! test rejects any call to those functions from outside `castellan-guard`.

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::todo,
        clippy::unimplemented,
        clippy::dbg_macro,
    )
)]

use std::ffi::OsStr;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use thiserror::Error;
use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, DELETE, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
    FILE_DISPOSITION_INFO, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_GENERIC_READ, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, GetDiskFreeSpaceExW,
    GetFileAttributesW, GetFinalPathNameByHandleW, GetTempPath2W, OPEN_EXISTING,
    SetFileInformationByHandle, SetFileTime, VOLUME_NAME_DOS,
};
use windows::Win32::System::LibraryLoader::{
    GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW,
};
use windows::Win32::System::SystemInformation::{GetSystemDirectoryW, GetSystemWindowsDirectoryW};
use windows::core::PCWSTR;

/// Failure from a Win32 wrapper. The message names the API and the Win32 code.
#[derive(Debug, Error)]
#[error("{operation} failed (Win32 {code})")]
pub struct WinError {
    /// Which wrapper failed.
    pub operation: &'static str,
    /// `GetLastError` style code, when the API reported one.
    pub code: u32,
}

impl WinError {
    fn new(operation: &'static str, code: u32) -> Self {
        Self { operation, code }
    }
}

/// Capability token required by destructive wrappers.
///
/// Constructing it is public only so `castellan-guard` can. The architecture test
/// forbids every other crate from naming [`GuardToken::issue`] or `guard_delete_opened`.
#[derive(Clone, Copy, Debug)]
pub struct GuardToken {
    _private: (),
}

impl GuardToken {
    /// Issue a token. Call this only from `castellan-guard`.
    #[must_use]
    pub const fn issue() -> Self {
        Self { _private: () }
    }
}

/// Attributes that the sweeper needs before it decides to walk or delete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileAttr {
    /// The path is a directory.
    pub directory: bool,
    /// Junction, symlink, mount point, or other reparse point.
    pub reparse_point: bool,
}

/// Free space for one volume, measured at `path`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiskSpace {
    /// Total bytes available to the caller.
    pub total_bytes: u64,
    /// Free bytes available to the caller.
    pub free_bytes: u64,
}

/// An open file that was opened without following a reparse point.
pub struct OpenedFile {
    handle: HANDLE,
    /// Final path reported by `GetFinalPathNameByHandleW`.
    pub final_path: PathBuf,
}

impl Drop for OpenedFile {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            // SAFETY: the handle was returned by CreateFileW and has not been closed yet.
            // CloseHandle is safe to call on that handle exactly once. Drop runs once.
            let _ = unsafe { CloseHandle(self.handle) };
        }
    }
}

/// Restrict later `LoadLibrary` calls to System32.
///
/// This cannot change DLLs that were already mapped before `main`. Castellan avoids
/// delay-loading anything except through [`guard_flush_dns`], which uses an absolute path.
pub fn lock_dll_search() -> Result<(), WinError> {
    // SAFETY: SetDefaultDllDirectories changes process DLL search state and takes no pointer.
    // LOAD_LIBRARY_SEARCH_SYSTEM32 is a documented flag. Failure is returned, not ignored.
    unsafe {
        windows::Win32::System::LibraryLoader::SetDefaultDllDirectories(
            LOAD_LIBRARY_SEARCH_SYSTEM32,
        )
    }
    .map_err(|error| WinError::new("SetDefaultDllDirectories", error.code().0 as u32))
}

/// `GetTempPath2W`, which does not follow the legacy `TMP` hijack of `GetTempPathW`.
pub fn user_temp_dir() -> Result<PathBuf, WinError> {
    let mut buffer = vec![0u16; 1024];
    // SAFETY: buffer is writable and its length is passed. The API writes a
    // NUL-terminated path and does not retain the pointer.
    let len = unsafe { GetTempPath2W(Some(buffer.as_mut_slice())) };
    path_from_api("GetTempPath2W", &mut buffer, len)
}

/// `GetSystemWindowsDirectoryW`, so the Windows directory is never hard-coded as `C:\Windows`.
pub fn windows_directory() -> Result<PathBuf, WinError> {
    let mut buffer = vec![0u16; 1024];
    // SAFETY: buffer is writable. The API writes a NUL-terminated directory and
    // does not retain the pointer.
    let len = unsafe { GetSystemWindowsDirectoryW(Some(buffer.as_mut_slice())) };
    path_from_api("GetSystemWindowsDirectoryW", &mut buffer, len)
}

/// System32 from `GetSystemDirectoryW`.
pub fn system_directory() -> Result<PathBuf, WinError> {
    let mut buffer = vec![0u16; 1024];
    // SAFETY: buffer is writable. The API writes a NUL-terminated directory and
    // does not retain the pointer.
    let len = unsafe { GetSystemDirectoryW(Some(buffer.as_mut_slice())) };
    path_from_api("GetSystemDirectoryW", &mut buffer, len)
}

/// Free space for the volume that contains `path`. Read-only.
pub fn disk_free_space(path: &Path) -> Result<DiskSpace, WinError> {
    let wide = to_wide(path);
    let mut free = 0u64;
    let mut total = 0u64;
    // SAFETY: wide is NUL-terminated and not retained. The output pointers are local.
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            PCWSTR(wide.as_ptr()),
            Some(&raw mut free),
            Some(&raw mut total),
            None,
        )
    };
    ok.map_err(|error| WinError::new("GetDiskFreeSpaceExW", error.code().0 as u32))?;
    Ok(DiskSpace {
        total_bytes: total,
        free_bytes: free,
    })
}

/// Read attributes without opening the target of a reparse point.
pub fn file_attributes(path: &Path) -> Result<FileAttr, WinError> {
    let wide = to_wide(path);
    // SAFETY: wide is NUL-terminated. GetFileAttributesW only reads the path.
    let attrs = unsafe { GetFileAttributesW(PCWSTR(wide.as_ptr())) };
    if attrs == u32::MAX {
        return Err(WinError::new("GetFileAttributesW", last_error_code()));
    }
    Ok(FileAttr {
        directory: (attrs & FILE_ATTRIBUTE_DIRECTORY.0) != 0,
        reparse_point: (attrs & FILE_ATTRIBUTE_REPARSE_POINT.0) != 0,
    })
}

/// Open a path without following a reparse point, then read its final path.
pub fn open_no_follow(path: &Path) -> Result<OpenedFile, WinError> {
    let wide = to_wide(&extended_path(path));
    // SAFETY: wide is NUL-terminated. OPEN_EXISTING does not create a file.
    // FILE_FLAG_OPEN_REPARSE_POINT stops the kernel from traversing a junction.
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide.as_ptr()),
            FILE_GENERIC_READ.0 | DELETE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
    }
    .map_err(|error| WinError::new("CreateFileW", error.code().0 as u32))?;
    match final_path(handle) {
        Ok(final_path) => Ok(OpenedFile { handle, final_path }),
        Err(error) => {
            // SAFETY: CreateFileW succeeded, so this handle is open and not yet owned
            // by OpenedFile. Closing it here prevents a leak on the error path.
            let _ = unsafe { CloseHandle(handle) };
            Err(error)
        }
    }
}

/// Try to open a file with no sharing, as a proxy for "somebody else is using it".
pub fn exclusive_open_possible(path: &Path) -> Result<bool, WinError> {
    let wide = to_wide(&extended_path(path));
    // SAFETY: wide is NUL-terminated. The share mode is zero, so this only probes
    // whether the file can be opened exclusively. The handle is closed before return.
    let opened = unsafe {
        CreateFileW(
            PCWSTR(wide.as_ptr()),
            FILE_GENERIC_READ.0,
            windows::Win32::Storage::FileSystem::FILE_SHARE_NONE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
    };
    match opened {
        Ok(handle) => {
            // SAFETY: CreateFileW returned this handle. It is closed once, here.
            let _ = unsafe { CloseHandle(handle) };
            Ok(true)
        }
        Err(error) => {
            let code = error.code().0 as u32;
            // ERROR_SHARING_VIOLATION = 32. Anything else is a real failure.
            if code == 32 {
                Ok(false)
            } else {
                Err(WinError::new("CreateFileW", code))
            }
        }
    }
}

/// Delete an opened file or empty directory by handle. Does not take a path.
///
/// # Errors
/// Returns the Win32 code when the disposition cannot be set.
pub fn guard_delete_opened(token: GuardToken, opened: &OpenedFile) -> Result<(), WinError> {
    let _ = token;
    let info = FILE_DISPOSITION_INFO { DeleteFile: true };
    // SAFETY: `opened.handle` is an open CreateFileW handle that includes DELETE
    // access. The info pointer lives for the duration of the call and refers to
    // a FILE_DISPOSITION_INFO of the size we pass. The handle is not closed here;
    // the kernel deletes the name when the last handle closes.
    unsafe {
        SetFileInformationByHandle(
            opened.handle,
            windows::Win32::Storage::FileSystem::FileDispositionInfo,
            (&raw const info).cast(),
            u32::try_from(std::mem::size_of::<FILE_DISPOSITION_INFO>()).unwrap_or(0),
        )
    }
    .map_err(|error| WinError::new("SetFileInformationByHandle", error.code().0 as u32))
}

/// Age a file the test suite created. Production code does not call this.
pub fn set_file_ages(
    path: &Path,
    created: SystemTime,
    modified: SystemTime,
) -> Result<(), WinError> {
    let wide = to_wide(&extended_path(path));
    // SAFETY: wide is NUL-terminated. The handle is closed before this function returns.
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide.as_ptr()),
            windows::Win32::Storage::FileSystem::FILE_WRITE_ATTRIBUTES.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
    }
    .map_err(|error| WinError::new("CreateFileW", error.code().0 as u32))?;
    let created_ft = filetime_from_system(created);
    let modified_ft = filetime_from_system(modified);
    // SAFETY: handle is open with FILE_WRITE_ATTRIBUTES. The FILETIME pointers are
    // local and live for the call. NULL is not used; both timestamps are set.
    let result = unsafe {
        SetFileTime(
            handle,
            Some(&raw const created_ft),
            None,
            Some(&raw const modified_ft),
        )
    };
    // SAFETY: handle came from CreateFileW above and has not been closed.
    let _ = unsafe { CloseHandle(handle) };
    result.map_err(|error| WinError::new("SetFileTime", error.code().0 as u32))
}

/// Resolve `DnsFlushResolverCache` from `System32\dnsapi.dll` and call it.
///
/// The library path is absolute under System32, so a DLL planted next to the
/// executable is not used.
pub fn guard_flush_dns(token: GuardToken) -> Result<(), WinError> {
    let _ = token;
    let mut dll = system_directory()?;
    dll.push("dnsapi.dll");
    let wide = to_wide(&dll);
    // SAFETY: wide is a NUL-terminated absolute path under System32.
    // LOAD_LIBRARY_SEARCH_SYSTEM32 blocks the application directory and the PATH.
    let module =
        unsafe { LoadLibraryExW(PCWSTR(wide.as_ptr()), None, LOAD_LIBRARY_SEARCH_SYSTEM32) }
            .map_err(|error| WinError::new("LoadLibraryExW", error.code().0 as u32))?;
    // SAFETY: the name is a NUL-terminated constant. GetProcAddress does not retain it.
    // The module stays loaded for the process, which is acceptable for a system DLL.
    let symbol = unsafe { GetProcAddress(module, windows::core::s!("DnsFlushResolverCache")) };
    let Some(flush) = symbol else {
        return Err(WinError::new("GetProcAddress", last_error_code()));
    };
    // SAFETY: the symbol was loaded from System32\dnsapi.dll under the name
    // DnsFlushResolverCache. FARPROC is `unsafe extern "system" fn() -> isize`,
    // which matches that export's register return (BOOL). The call only drops
    // the resolver cache and takes no pointer.
    let ok = unsafe { flush() };
    if ok == 0 {
        Err(WinError::new("DnsFlushResolverCache", last_error_code()))
    } else {
        Ok(())
    }
}

/// Confirm `System32\dnsapi.dll` exports the flush function, without calling it.
pub fn dns_flush_export_present() -> Result<bool, WinError> {
    let mut dll = system_directory()?;
    dll.push("dnsapi.dll");
    let wide = to_wide(&dll);
    // SAFETY: same constraints as `guard_flush_dns`: absolute System32 path.
    let module =
        unsafe { LoadLibraryExW(PCWSTR(wide.as_ptr()), None, LOAD_LIBRARY_SEARCH_SYSTEM32) }
            .map_err(|error| WinError::new("LoadLibraryExW", error.code().0 as u32))?;
    // SAFETY: constant NUL-terminated symbol name. The pointer is not dereferenced here.
    let symbol = unsafe { GetProcAddress(module, windows::core::s!("DnsFlushResolverCache")) };
    Ok(symbol.is_some())
}

fn final_path(handle: HANDLE) -> Result<PathBuf, WinError> {
    let mut buffer = vec![0u16; 1024];
    // SAFETY: handle is open. The buffer is writable for `buffer.len()` code units.
    // Volume name is the DOS path (`\\?\C:\...`), which is what the guard compares.
    let len = unsafe { GetFinalPathNameByHandleW(handle, buffer.as_mut_slice(), VOLUME_NAME_DOS) };
    path_from_api("GetFinalPathNameByHandleW", &mut buffer, len)
}

fn path_from_api(
    operation: &'static str,
    buffer: &mut [u16],
    len: u32,
) -> Result<PathBuf, WinError> {
    let len = usize::try_from(len).unwrap_or(0);
    if len == 0 || len >= buffer.len() {
        return Err(WinError::new(operation, last_error_code()));
    }
    let Some(text) = buffer.get(..len) else {
        return Err(WinError::new(operation, 0));
    };
    Ok(PathBuf::from(std::ffi::OsString::from_wide(text)))
}

fn extended_path(path: &Path) -> PathBuf {
    let text = path.as_os_str();
    let raw: Vec<u16> = text.encode_wide().collect();
    if raw.starts_with(&[
        u16::from(b'\\'),
        u16::from(b'\\'),
        u16::from(b'?'),
        u16::from(b'\\'),
    ]) {
        return path.to_path_buf();
    }
    let mut out = PathBuf::from(r"\\?\");
    out.push(path);
    out
}

fn to_wide(path: &Path) -> Vec<u16> {
    let mut wide: Vec<u16> = OsStr::new(path).encode_wide().collect();
    wide.push(0);
    wide
}

fn filetime_from_system(time: SystemTime) -> FILETIME {
    let duration = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    // FILETIME is 100ns ticks since 1601-01-01. The offset from Unix epoch is constant.
    const EPOCH_DIFF_100NS: u64 = 116_444_736_000_000_000;
    let ticks = duration.as_secs().saturating_mul(10_000_000)
        + u64::from(duration.subsec_nanos()) / 100
        + EPOCH_DIFF_100NS;
    FILETIME {
        dwLowDateTime: ticks as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    }
}

fn last_error_code() -> u32 {
    // SAFETY: GetLastError reads thread-local state and takes no pointer.
    unsafe { windows::Win32::Foundation::GetLastError().0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_folders_are_absolute_and_not_hard_coded() {
        let temp = user_temp_dir().expect("temp");
        let windows = windows_directory().expect("windows");
        assert!(temp.is_absolute());
        assert!(windows.is_absolute());
        assert!(temp != windows);
    }

    #[test]
    fn system_drive_reports_free_space() {
        let windows = windows_directory().expect("windows");
        let space = disk_free_space(&windows).expect("space");
        assert!(space.total_bytes > 0);
        assert!(space.free_bytes <= space.total_bytes);
    }

    #[test]
    fn dns_export_is_present_without_flushing() {
        assert!(dns_flush_export_present().expect("dnsapi"));
    }
}
