use core::{ffi::CStr};
use core::mem::MaybeUninit;

use libc::{
    EINTR, MS_RDONLY, MS_REMOUNT, S_IFDIR, S_IFMT, SIG_BLOCK, SIGCHLD, SIGHUP, SIGINT, SIGKILL, SIGPWR, SIGTERM, WNOHANG, 
    c_int, c_ulong, c_void, kill, mkdir, mode_t, mount, 
    sigaddset, sigemptyset, siginfo_t, sigprocmask, sigset_t, sigwaitinfo, stat, waitpid
};

/// Setcion: Mounting Filesystems
 
/// A struct used to define all the attributes needed to make the mount system call.
pub struct MountEntry<'a> {
    pub parent_path: &'a CStr,
    pub source: &'a CStr,
    pub target: &'a CStr,
    pub filesystem_type: &'a CStr,
    pub flags: c_ulong,
    pub data: &'a CStr
}

/// Uses a MountEntry to verify that:
/// 1. The target mount exists and ensure it is a directory and creates it if not present with permission 0o755.
/// 2. No filesystem is mounted on the target.
/// Return Ok when a filesystem is already mounted or after a successful call to mount.
pub fn mount_fs(me: &MountEntry) -> c_int {
    let mut target_stat = MaybeUninit::<stat>::uninit();
    let mut parent_stat = MaybeUninit::<stat>::uninit();
    let mut err: c_int;

    err = unsafe { stat(me.target.as_ptr(), target_stat.as_mut_ptr()) };
    if err != 0 { return err; }

    err = unsafe { stat(me.parent_path.as_ptr(), parent_stat.as_mut_ptr()) };
    if err != 0 { return err; }

    let target_stat = unsafe { target_stat.assume_init() };
    let parent_stat = unsafe { parent_stat.assume_init() };
    // Ensures target is a directory. st_mode & S_IFMT means filetype and S_IFDIR means directory.
    if (target_stat.st_mode & S_IFMT) != S_IFDIR {
        if unsafe { mkdir(me.target.as_ptr(), 0o755 as mode_t) } != 0 {
            return 1;
        }
    }

    // If the device ids of target and its parent are different it means a filesystem has already been mounted to target.
    if target_stat.st_dev != parent_stat.st_dev {
        return 0;
    }

    unsafe { mount(me.source.as_ptr(),me.target.as_ptr(),me.filesystem_type.as_ptr(),me.flags,me.data.as_ptr() as *const c_void) }
}

///Section: Symlink

/// Creates a symbolic link.
pub fn symlink_create(src: &CStr, dst: &CStr) -> c_int {
    let mut st = MaybeUninit::<stat>::uninit();
    if unsafe { stat(dst.as_ptr(), st.as_mut_ptr()) } != 0 {
        return unsafe { libc::symlink(src.as_ptr(), dst.as_ptr()) };
    }
    0
}

///Section: Signals

/// Sets up the signals that need to be handled.
pub fn signal_setup() -> c_int {
    let set = init_sigset();
    unsafe { sigprocmask(SIG_BLOCK, &set, core::ptr::null_mut()) }
}

pub fn signal_run_loop() -> ! {
    let set = init_sigset();
    loop {
        let mut info = MaybeUninit::<siginfo_t>::zeroed();
        let ret = unsafe { sigwaitinfo(&set, info.as_mut_ptr()) };
        if ret >= 0 {
                let saved = unsafe {info.assume_init() };
            dispatch(&saved);
        } else {
            let err = unsafe { *libc::__errno_location() };
            if err == EINTR {
                continue; // sigwaitinfo can EINTR even on blocked sets
            }
        }
    }
}

/// Shutsdown the system gracefully.
fn signal_shutdown(cmd: libc::c_int) {
    unsafe { kill(-1, SIGTERM); }

    let mut status: c_int = 0;
    for _ in 0..30 {
        while unsafe { waitpid(-1, &mut status, WNOHANG) } > 0 {}
        unsafe { libc::usleep(100_000); }
        //thread::sleep(Duration::from_millis(100));
    }

    unsafe { kill(-1, SIGKILL); }

    while unsafe { waitpid(-1, &mut status, WNOHANG) } > 0 {}

    unsafe { libc::sync(); }

    let root = c"/";
    unsafe { mount(core::ptr::null(), root.as_ptr(), core::ptr::null(), MS_REMOUNT | MS_RDONLY, core::ptr::null()); }

    unsafe { libc::reboot(cmd); }

    loop {
        unsafe { libc::pause(); }
    }

}

/// Initializes a set containing all the signals that need to be handled.
fn init_sigset() -> sigset_t {
    let mut set = MaybeUninit::<sigset_t>::zeroed();
    unsafe {
        let set_ptr = set.as_mut_ptr();
        assert_eq!(sigemptyset(set_ptr), 0);
        assert_eq!(sigaddset(set_ptr, SIGCHLD), 0);
        assert_eq!(sigaddset(set_ptr, SIGINT), 0);
        assert_eq!(sigaddset(set_ptr, SIGTERM), 0);
        assert_eq!(sigaddset(set_ptr, SIGHUP), 0);
        assert_eq!(sigaddset(set_ptr, SIGPWR), 0);
        set.assume_init()
    }
}

/// Calls the appropriate function for each signal that needs to be handled.
fn dispatch(info: &siginfo_t) {
    match info.si_signo {
        SIGCHLD => reap_children(),
        SIGINT  => signal_shutdown(libc::LINUX_REBOOT_CMD_RESTART),
        SIGTERM => signal_shutdown(libc::LINUX_REBOOT_CMD_POWER_OFF),
        SIGPWR => signal_shutdown(libc::LINUX_REBOOT_CMD_POWER_OFF),
        //SIGHUP  => { println!("[Oxide] SIGUP received (reload)"); }
        _ => {}
    }
}

/// Reaps children until there are none using waitpid inside an infinite loop.
fn reap_children() {
    let mut status: c_int = 0;
    loop {
        if unsafe { waitpid(-1, &mut status, WNOHANG) } <= 0 {
                break;
        }
    }
}
