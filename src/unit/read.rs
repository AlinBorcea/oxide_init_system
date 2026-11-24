use core::ffi::{CStr, c_int, c_void};

use crate::data::unit_file::*;
use crate::multi_pager::MultiPager;
use crate::pager_set::PagerSet;
use crate::pager::{Pager, SlicePager, Handle};

use libc::{size_t, dirent};

pub struct DirEntry {
    pub filename: [u8;256],
    pub written: u8,
}

pub struct DirGuard {
    dir: *mut libc::DIR,
}

impl Drop for DirGuard {
    fn drop(&mut self) {
        unsafe { libc::closedir(self.dir); }
    }
}

pub fn open_dir(dirpath: &CStr) -> Option<DirGuard> {
    let dir_ptr = unsafe { libc::opendir(dirpath.as_ptr()) };
    if dir_ptr.is_null() {
        return None;
    }

    Some(DirGuard { dir: dir_ptr })
}

pub fn read_dir_entry(dir_guard: &mut DirGuard, entry: &mut DirEntry) -> c_int {
    unsafe { *libc::__errno_location() = 0; }
    let entry_ptr: *mut dirent = unsafe { libc::readdir(dir_guard.dir) };

    if entry_ptr.is_null() {
        return 1;
    }

    if unsafe { *libc::__errno_location() != 0 } {
        return 2;
    }

    let local_entry = unsafe { &*entry_ptr };
    let name_ptr = local_entry.d_name.as_ptr() as *const core::ffi::c_char;
    let cstr = unsafe { CStr::from_ptr(name_ptr) };

    let bytes_with_nul = cstr.to_bytes_with_nul();
    if bytes_with_nul.len() > entry.filename.len() {
        return 3;
    }
    
    entry.filename[..bytes_with_nul.len()].copy_from_slice(bytes_with_nul);
    entry.written = bytes_with_nul.len() as u8;
    0
}

pub fn read_unit<P: Pager>(filename: &CStr, fd: i32, read_buffer: &mut [u8], unit: &mut Unit, pager: &mut P, multi_pager: &mut MultiPager, pager_set: &mut PagerSet) -> Option<isize> {
    let mut bytes_read: isize;
    let mut read_offset = 0;

    match pager_set.append(filename) {
        Some(handle) => unit.id = Some(handle),
        None => { return None },
    };

    loop {
        unsafe { 
            bytes_read = libc::read(fd, 
            read_buffer.as_mut_ptr().add(read_offset) as *mut c_void, 
            (read_buffer.len() - read_offset) as size_t);
        }

        if bytes_read <= 0 {
            break;
        }

        let total_valid = bytes_read as usize + read_offset;

        let mut ida = 0;
        while let Some(idb) = find_next_symbol(read_buffer,ida, total_valid, b'\n') {
            let line = &read_buffer[ida..idb];

            if !line.is_empty() && line[0] != b'#' && line[0] != b';' {
                parse_and_apply_service_line(line, unit, pager, multi_pager, pager_set);
            }

            ida = idb + 1;
        }

        read_buffer.copy_within(ida..total_valid, 0);
        read_offset = total_valid - ida;
    }
    Some(0)
}

fn parse_and_apply_service_line<P: Pager, S: SlicePager>(line: &[u8], unit: &mut Unit, pager: &mut P, multi_pager: &mut S, pager_set: &mut PagerSet) {
    let eq_idx_opt = find_next_symbol(line, 0, line.len(), b'=');
    if let None = eq_idx_opt {
        return;
    }

    let eq_idx = eq_idx_opt.unwrap();

    let key = &line[..eq_idx];
    let value = &line[eq_idx + 1..];

    let mut buffer = [0u8;256];
    buffer[..value.len()].copy_from_slice(value);

    match (key, &mut unit.kind) {
        (b"Type", UnitKind::Service(s)) => {
            match value {
                b"simple" => s.service_type = ServiceType::Simple,
                b"forking" => s.service_type = ServiceType::Forking,
                b"oneshot" => s.service_type = ServiceType::Oneshot,
                _ => {}
            }
        }
        (b"Restart", UnitKind::Service(s)) => {
            match value {
                b"no" => s.restart_policy = RestartPolicy::No,
                b"always" => s.restart_policy = RestartPolicy::Always,
                b"on-failure" => s.restart_policy = RestartPolicy::OnFailure,
                _ => {}
            }
        }
        (b"RestartSec", UnitKind::Service(s)) => {
            let mut secs: u16 = 0;
            for &b in value {
                if (b'0'..=b'9').contains(&b) {
                    secs = secs.saturating_mul(10).saturating_add((b - b'0') as u16);
                } else if b == b's' || b == b' ' || b == b'\r' || b == b'\n' {
                    break;
                } else {
                    break;
                }
            }
            s.restart_sec = secs;
        }
        (b"User", UnitKind::Service(s)) => {
            // 1. Try parsing as a raw numeric UID first (e.g., User=1000)
            let mut uid_val: u32 = 0;
            let mut is_numeric = true;
            for &b in value {
                if (b'0'..=b'9').contains(&b) {
                    uid_val = uid_val.saturating_mul(10).saturating_add((b - b'0') as u32);
                } else {
                    is_numeric = false;
                    break;
                }
            }

            if is_numeric && !value.is_empty() {
                s.uid = uid_val;
            } else {
                 // 2. If it's a username string (e.g., User=www-data), resolve via libc::getpwnam
                if value.len() < 256 {
                    let mut stack_buf = [0u8; 256];
                    stack_buf[..value.len()].copy_from_slice(value);
                    // stack_buf[value.len()] is 0 (null terminator)

                    if let Ok(cstr) = core::ffi::CStr::from_bytes_with_nul(&stack_buf[..=value.len()]) {
                        unsafe {
                            let pwd = libc::getpwnam(cstr.as_ptr());
                            if !pwd.is_null() {
                                s.uid = (*pwd).pw_uid;
                            } else {
                                // Fallback: Default to root (0) or handle error appropriately
                                s.uid = 0;
                            }
                        }
                    }
                }
            }
        }
        (b"Group", UnitKind::Service(s)) => {
            let mut gid_val: u32 = 0;
            let mut is_numeric = true;
            for &b in value {
                if (b'0'..=b'9').contains(&b) {
                    gid_val = gid_val.saturating_mul(10).saturating_add((b - b'0') as u32);
                } else {
                    is_numeric = false;
                    break;
                }
            }

            if is_numeric && !value.is_empty() {
                s.gid = gid_val;
            } else {
                // 2. If it's a group name string (e.g., Group=www-data), resolve via libc::getgrnam
                if value.len() < 256 {
                    let mut stack_buf = [0u8; 256];
                    stack_buf[..value.len()].copy_from_slice(value);
                    // stack_buf[value.len()] is already 0 by default initialization

                    if let Ok(cstr) = core::ffi::CStr::from_bytes_with_nul(&stack_buf[..=value.len()]) {
                        unsafe {
                            let grp = libc::getgrnam(cstr.as_ptr());
                            if !grp.is_null() {
                                s.gid = (*grp).gr_gid;
                            } else {
                                // Fallback: Default to root (0) or handle error appropriately
                                s.gid = 0;
                            }
                        }
                    }
                }
            }
        }
        (b"ExecStart", UnitKind::Service(s)) => {
            if let Ok(cstr) = CStr::from_bytes_with_nul(&buffer[..=value.len()]) {
                    if let Some(handle) = pager.append(cstr) {
                        s.exec_start_handle = handle;
                    }
            }    
        }
        (b"WorkingDirectory", UnitKind::Service(s)) => {
            // Ensure the value fits within our temporary stack buffer (+1 for null terminator)
            if value.len() < 256 {
                let mut stack_buf = [0u8; 256];
                stack_buf[..value.len()].copy_from_slice(value);
                // stack_buf[value.len()] is already 0 due to zero-initialization, 
                // which provides the required null terminator for CStr.

                if let Ok(cstr) = core::ffi::CStr::from_bytes_with_nul(&stack_buf[..=value.len()]) {
                    if let Some(handle) = pager.append(cstr) {
                        s.working_dir_handle = Some(handle);
                    }
                }
            }
        }
        (b"RemainAfterExit", UnitKind::Service(s)) => {
            let is_true = matches!(value, b"true" | b"yes" | b"1" | b"on");
            let is_false = matches!(value, b"false" | b"no" | b"0" | b"off");

            if is_true {
                s.flags |= FLAG_REMAIN_AFTER_EXIT;
            } else if is_false {
                s.flags &= !FLAG_REMAIN_AFTER_EXIT;
            }
        }
        (b"PrivateTmp", UnitKind::Service(s)) => {
            let is_true = matches!(value, b"true" | b"yes" | b"1" | b"on");
            let is_false = matches!(value, b"false" | b"no" | b"0" | b"off");

            if is_true {
                s.flags |= FLAG_PRIVATE_TMP;
            } else if is_false {
                s.flags &= !FLAG_PRIVATE_TMP;
            }
        }
        (b"After" | b"Before" | b"Wants" | b"Requires", _) => {
        let mut handles = [Handle::new(0); 32]; // Max 32 items per line buffer
        let mut count = 0;
            
        // Zero-allocation whitespace tokenization of the value line
        let mut i = 0;
        while i < value.len() && count < handles.len() {
            // Skip leading whitespace
            while i < value.len() && (value[i] == b' ' || value[i] == b'\t' || value[i] == b'\r' || value[i] == b'\n') {
                i += 1;
            }
            if i >= value.len() { break; }
                
            // Find token end
            let start = i;
            while i < value.len() && value[i] != b' ' && value[i] != b'\t' && value[i] != b'\r' && value[i] != b'\n' {
                i += 1;
            }
            let token = &value[start..i];
                
            if !token.is_empty() && token.len() < 255 {
                let mut word_buf = [0u8; 256];
                word_buf[..token.len()].copy_from_slice(token);
                // word_buf[token.len()] is already 0 (null-terminated)
                    
                if let Ok(cstr) = core::ffi::CStr::from_bytes_with_nul(&word_buf[..=token.len()]) {
                    if let Some(h) = pager_set.append(cstr) {
                        handles[count] = h;
                        count += 1;
                    }
                }
            }
        }

            // If we successfully parsed handles, store them in the multi_pager
            if count > 0 {
                if let Some(multi_handle) = multi_pager.append(&handles[..count]) {
                    match key {
                        b"After" => { 
                            unit.unit_section.after_handle = Some(multi_handle); 
                            unit.unit_section.in_degree += count as u16; 
                        },
                        b"Before" => unit.unit_section.before_handle = Some(multi_handle),
                        b"Wants" => unit.unit_section.wants_handle = Some(multi_handle),
                        b"Requires" => unit.unit_section.requires_handle = Some(multi_handle),
                        _ => {}
                    }
                }
            }
        }
         _ => {}
    }
}

fn find_next_symbol(buffer: &[u8], start_index: usize, end_index: usize, symbol: u8) -> Option<usize> {
    if start_index > end_index {
        return None;
    }

    for i in start_index..end_index {
        if buffer[i] == symbol {
            return Some(i);
        }
    }

    None
}
