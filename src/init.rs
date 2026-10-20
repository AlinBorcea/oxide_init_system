#![no_std]
#![no_main]

#[path ="setup/linux.rs"]
pub mod linux;

use core::panic::PanicInfo;

use linux::MountEntry;
use libc::{c_int};

#[link(name = "c")]
unsafe extern "C" {}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe { libc::_exit(101) };
}

#[unsafe(no_mangle)]
pub extern "C" fn main(_argc: c_int, _argv: *const *const u8) {
    setup_filesystems();
    setup_symlinks();
    setup_hostname();
    setup_loopback();
    
    _ = linux::signal_setup();

    // Invokes /bin/sh in order to have a usable system with user input.
    unsafe {
        let pid = libc::fork();
        if pid == 0 {
            // CHILD PROCESS: Execute login shell
            let sh_path = c"/bin/sh".as_ptr();
            let arg0 = c"/bin/sh".as_ptr();
            let arg1 = c"-l".as_ptr();
            
            let args = [arg0, arg1, core::ptr::null()];
            let env = [
                c"HOME=/root".as_ptr(),
                c"TERM=linux".as_ptr(),
                core::ptr::null(),
            ];

            libc::execve(sh_path, args.as_ptr(), env.as_ptr());
            
            // If execve fails, exit immediately to prevent child duplicate logic
            libc::_exit(127);
        }
    }
    
    linux::signal_run_loop();
}

// Mounting Filesystems
fn setup_filesystems() {
    let default_flags = libc::MS_NOSUID | libc::MS_NOEXEC | libc::MS_NODEV;
    let empty_str = c"";
    let dir_root = c"/";
    let dir_dev = c"/dev";

    let entries = [
        // Core Filesystems
        MountEntry {parent_path: dir_root, source: c"proc", target: c"/proc", filesystem_type: c"proc", flags: default_flags, data: empty_str},
        MountEntry {parent_path: dir_root, source: c"sysfs", target: c"/sys", filesystem_type: c"sysfs", flags: default_flags, data: empty_str},
        MountEntry {parent_path: dir_root, source: c"run", target: c"/run", filesystem_type: c"tmpfs", flags: default_flags, data: empty_str},
        MountEntry {parent_path: dir_root, source: c"devtmpfs", target: dir_dev, filesystem_type: c"devtmpfs", flags: libc::MS_NOEXEC, data: c"mode=0755"},
        MountEntry {parent_path: dir_root, source: dir_root, target: dir_root, filesystem_type: empty_str, flags: libc::MS_REMOUNT | libc::MS_RELATIME, data: empty_str},
        
        // Secondary Filesystems
        MountEntry {parent_path: dir_dev, source: c"devpts", target: c"/dev/pts", filesystem_type: c"devpts", flags: libc::MS_NOSUID | libc::MS_NOEXEC, data: c"mode=0620,ptmxmode=000"},
        MountEntry {parent_path: dir_dev, source: c"mqueue", target: c"/dev/mqueue", filesystem_type: c"mqueue", flags: default_flags, data: c"mode=0620,ptmxmode=000"},
        
        // Optional Filesystems
        MountEntry {parent_path: c"/proc", source: c"proc/xenfs", target: c"/proc/xen", filesystem_type: c"xenfs", flags: default_flags, data: empty_str},
        MountEntry {parent_path: c"/sys", source: c"securityfs", target: c"/sys/kernel/security", filesystem_type: c"securityfs", flags: default_flags, data: empty_str},
        MountEntry {parent_path: c"/sys", source: c"debugfs", target: c"/sys/kernel/debug", filesystem_type: c"debugfs", flags: default_flags, data: empty_str},
        MountEntry {parent_path: c"/sys", source: c"tracefs", target: c"/sys/kernel/tracing", filesystem_type: c"tracefs", flags: default_flags, data: empty_str},
        MountEntry {parent_path: c"/sys", source: c"pstore", target: c"/sys/fs/pstore", filesystem_type: c"pstore", flags: default_flags, data: empty_str},
        MountEntry {parent_path: c"/sys", source: c"bpf", target: c"/sys/fs/bpf", filesystem_type: c"bpf", flags: default_flags, data: empty_str}
    ];
    
    for entry in entries {
        _ = linux::mount_fs(&entry);
    }
}

// Creating symlinks
fn setup_symlinks() {
    unsafe {
        let run_lock_path = c"/run/lock";
        libc::mkdir(run_lock_path.as_ptr(), 0o1777);
        libc::chmod(run_lock_path.as_ptr(), 0o1777);
    }

    _ = linux::symlink_create(c"/proc/self/fd", c"/dev/fd");
    _ = linux::symlink_create(c"/proc/self/fd/0", c"/dev/stdin");
    _ = linux::symlink_create(c"/proc/self/fd/1", c"/dev/stdout");
    _ = linux::symlink_create(c"/proc/self/fd/2", c"/dev/stderr");
}

// Hostname
fn setup_hostname() {
    let hostname = c"Oxide-vm";
    unsafe {
        libc::sethostname(hostname.as_ptr(), hostname.to_bytes().len());
    }
}

// Loopback
fn setup_loopback() {
    #[cfg(target_env = "gnu")]
    let io1 = libc::SIOCGIFFLAGS as u64;
    #[cfg(target_env = "gnu")]
    let io2 = libc::SIOCSIFFLAGS as u64;

    #[cfg(target_env = "musl")]
    let io1 = libc::SIOCGIFFLAGS as i32;
    #[cfg(target_env = "musl")]
    let io2 = libc::SIOCSIFFLAGS as i32;

    unsafe {
        let sock = libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0);
        if sock >= 0 {
            let mut ifr: libc::ifreq = core::mem::zeroed();
            let ifname = b"lo\0";
            core::ptr::copy_nonoverlapping(ifname.as_ptr() as *const i8, ifr.ifr_name.as_mut_ptr(), ifname.len());
            
            // Get existing flags
            if libc::ioctl(sock, io1, &mut ifr) >= 0 {
                ifr.ifr_ifru.ifru_flags |= (libc::IFF_UP | libc::IFF_RUNNING) as i16;
                libc::ioctl(sock, io2, &ifr);
            }
            libc::close(sock);
        }
    }
}