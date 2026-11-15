//! Windows runtime-file privacy. Only the process user, SYSTEM and administrators
//! may have allow entries; the DACL is protected from future parent inheritance.
//! Administrators retain the same machine-wide authority as root on Unix.

use std::ffi::c_void;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::Path;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{GENERIC_WRITE, INVALID_HANDLE_VALUE, LocalFree};
use windows_sys::Win32::Security::Authorization::*;
use windows_sys::Win32::Security::*;
use windows_sys::Win32::Storage::FileSystem::{
    CREATE_NEW, CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

struct Local(*mut c_void);

impl Drop for Local {
    fn drop(&mut self) {
        // SAFETY: Local contains only an allocation returned by the documented
        // LocalAlloc-based security APIs below, and owns it exactly once.
        unsafe { LocalFree(self.0) };
    }
}

fn bool_result(value: i32) -> io::Result<()> {
    if value == 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
}

fn wide(path: &Path) -> io::Result<Vec<u16>> {
    let mut v: Vec<_> = path.as_os_str().encode_wide().collect();
    if v.contains(&0) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "a runtime path contains NUL"));
    }
    v.push(0);
    Ok(v)
}

fn current_user() -> io::Result<Vec<usize>> {
    let mut handle = null_mut();
    // SAFETY: the pseudo process handle is valid; the output owns a new token handle.
    unsafe { bool_result(OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut handle))? };
    // SAFETY: OpenProcessToken succeeded and transferred ownership of this handle.
    let token = unsafe { OwnedHandle::from_raw_handle(handle) };
    let mut bytes = 0;
    // SAFETY: a null buffer with zero length asks Windows for the required size.
    unsafe { GetTokenInformation(token.as_raw_handle(), TokenUser, null_mut(), 0, &mut bytes) };
    if bytes == 0 {
        return Err(io::Error::last_os_error());
    }
    // usize storage gives TOKEN_USER and its embedded SID sufficient alignment.
    let mut buffer = vec![0usize; (bytes as usize).div_ceil(size_of::<usize>())];
    // SAFETY: buffer has at least bytes writable bytes and stays alive for the SID.
    unsafe {
        bool_result(GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            buffer.as_mut_ptr().cast(),
            bytes,
            &mut bytes,
        ))?;
    }
    Ok(buffer)
}

fn user_sid(buffer: &[usize]) -> PSID {
    // SAFETY: buffer is the successful TokenUser result and remains allocated.
    unsafe { (*(buffer.as_ptr().cast::<TOKEN_USER>())).User.Sid }
}

fn from_sddl(text: &str) -> io::Result<Local> {
    let text: Vec<_> = text.encode_utf16().chain(Some(0)).collect();
    let mut sd = null_mut();
    // SAFETY: input is terminated; Windows returns a LocalFree-owned descriptor.
    unsafe {
        bool_result(ConvertStringSecurityDescriptorToSecurityDescriptorW(
            text.as_ptr(),
            SDDL_REVISION_1,
            &mut sd,
            null_mut(),
        ))?
    };
    Ok(Local(sd))
}

fn private_descriptor(user: PSID, directory: bool) -> io::Result<Local> {
    let mut text = null_mut();
    // SAFETY: user points into the live TokenUser buffer; output uses LocalFree.
    unsafe { bool_result(ConvertSidToStringSidW(user, &mut text))? };
    let allocation = Local(text.cast());
    let mut len = 0;
    // SAFETY: ConvertSidToStringSidW returned a terminated UTF-16 SID string.
    let sid = unsafe {
        while *text.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(text, len))
    };
    drop(allocation);
    let inherit = if directory { "OICI" } else { "" };
    from_sddl(&format!("O:{sid}D:P(A;{inherit};FA;;;{sid})(A;{inherit};FA;;;SY)(A;{inherit};FA;;;BA)"))
}

fn security(path: &Path) -> io::Result<Local> {
    use std::os::windows::fs::MetadataExt;
    // Do not follow a junction or file symlink when checking or changing rights.
    if std::fs::symlink_metadata(path)?.file_attributes() & 0x400 != 0 {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "runtime path is a reparse point"));
    }
    let path = wide(path)?;
    let mut sd = null_mut();
    // SAFETY: name is terminated, only the descriptor output is requested.
    let error = unsafe {
        GetNamedSecurityInfoW(
            path.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            &mut sd,
        )
    };
    if error != 0 {
        return Err(io::Error::from_raw_os_error(error as i32));
    }
    Ok(Local(sd))
}

fn dacl(sd: &Local) -> io::Result<*mut ACL> {
    let mut present = 0;
    let mut defaulted = 0;
    let mut acl = null_mut();
    // SAFETY: sd is a live security descriptor; all outputs have valid storage.
    unsafe { bool_result(GetSecurityDescriptorDacl(sd.0, &mut present, &mut acl, &mut defaulted))? };
    if present == 0 || acl.is_null() {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "runtime DACL is absent"));
    }
    Ok(acl)
}

fn owner(sd: &Local, user: PSID) -> io::Result<()> {
    let mut sid = null_mut();
    let mut defaulted = 0;
    // SAFETY: the descriptor and TokenUser buffer outlive both SID pointers.
    unsafe {
        bool_result(GetSecurityDescriptorOwner(sd.0, &mut sid, &mut defaulted))?;
        if sid.is_null() || EqualSid(sid, user) == 0 {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "runtime path belongs to another user"));
        }
    }
    Ok(())
}

fn validate(sd: &Local, user: PSID) -> io::Result<()> {
    owner(sd, user)?;
    let mut control = 0;
    let mut revision = 0;
    // SAFETY: the live descriptor and scalar output pointers are valid.
    unsafe { bool_result(GetSecurityDescriptorControl(sd.0, &mut control, &mut revision))? };
    let private = private_descriptor(user, false)?;
    let allowed = dacl(&private)?;
    let actual = dacl(sd)?;
    let refused = || io::Error::new(io::ErrorKind::PermissionDenied, "runtime DACL is not private");
    if control & SE_DACL_PROTECTED == 0 {
        return Err(refused());
    }
    // SAFETY: both ACLs came from valid OS descriptors. GetAce bounds-checks each
    // index; only ordinary allow ACEs are cast, and their SID storage belongs to sd.
    unsafe {
        for i in 0..(*actual).AceCount {
            let mut raw = null_mut();
            bool_result(GetAce(actual, u32::from(i), &mut raw))?;
            let header = &*raw.cast::<ACE_HEADER>();
            if header.AceType != 0 || usize::from(header.AceSize) < size_of::<ACCESS_ALLOWED_ACE>() {
                return Err(refused());
            }
            let ace = &*raw.cast::<ACCESS_ALLOWED_ACE>();
            let sid = (&raw const ace.SidStart).cast_mut().cast();
            let mut trusted = false;
            for j in 0..(*allowed).AceCount {
                let mut raw = null_mut();
                bool_result(GetAce(allowed, u32::from(j), &mut raw))?;
                let other = &*raw.cast::<ACCESS_ALLOWED_ACE>();
                trusted |= EqualSid(sid, (&raw const other.SidStart).cast_mut().cast()) != 0;
            }
            if !trusted {
                return Err(refused());
            }
        }
    }
    Ok(())
}

/// Refuses non-owned or publicly readable runtime objects without changing them.
pub(super) fn check(path: &Path) -> io::Result<()> {
    let user = current_user()?;
    validate(&security(path)?, user_sid(&user))
}

/// Protects this owned object only, then verifies the result before token use.
pub(super) fn protect(path: &Path, directory: bool) -> io::Result<()> {
    let user = current_user()?;
    owner(&security(path)?, user_sid(&user))?;
    let sd = private_descriptor(user_sid(&user), directory)?;
    set_dacl(path, &sd)?;
    validate(&security(path)?, user_sid(&user))
}

/// Supplies the private DACL to file creation itself, before any handle or bytes
/// can be observed. Do not create with inherited permissions and repair later.
pub(super) fn create_private(path: &Path) -> io::Result<std::fs::File> {
    let user = current_user()?;
    let sd = private_descriptor(user_sid(&user), false)?;
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: sd.0,
        bInheritHandle: 0,
    };
    let name = wide(path)?;
    // SAFETY: the terminated name, attributes and descriptor outlive CreateFileW;
    // Windows applies the descriptor atomically. CREATE_NEW never opens a planted
    // existing object. A successful new handle is transferred to File exactly once.
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            &attributes,
            CREATE_NEW,
            FILE_ATTRIBUTE_NORMAL,
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: handle is an owned, successful CreateFileW result.
    let file = unsafe { std::fs::File::from_raw_handle(handle) };
    check(path)?;
    Ok(file)
}

fn set_dacl(path: &Path, sd: &Local) -> io::Result<()> {
    let path = wide(path)?;
    let acl = dacl(sd)?;
    // SAFETY: name and descriptor remain alive; only the named object's DACL is
    // changed. Protected inheritance prevents a broad profile grant returning.
    let error = unsafe {
        SetNamedSecurityInfoW(
            path.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            acl,
            null(),
        )
    };
    if error == 0 { Ok(()) } else { Err(io::Error::from_raw_os_error(error as i32)) }
}

#[cfg(test)]
mod tests;
