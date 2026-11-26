use core::ffi::CStr;

use libc;
use libc::{c_void, c_char};

use crate::data::unit_file::*;

pub fn run_unit(unit: &Unit, path: &CStr) -> Result<i32, &'static str> {
    let arg0 = path.as_ptr() as *const c_char;
    let argv: [*const c_char; 2] = [arg0, core::ptr::null()];
    let envp: [*const c_char; 1] = [core::ptr::null()];

    let UnitKind::Service(s) = &unit.kind else {
        return Err("Only Services Allowed");
    };

    match s.service_type {
        ServiceType::Simple => {
            let pid = unsafe { libc::fork() };
            if pid < 0 {
                return Err("Fork failed");
            }

            if pid == 0 {
                // --- CHILD PROCESS ---
                unsafe { libc::execve(
                    path.as_ptr() as *const c_char,
                    argv.as_ptr(),
                    envp.as_ptr(),
                ); }
                // If execve returns, it failed
                unsafe { libc::_exit(127) };
            } else {
                // --- PARENT (SUPERVISOR) ---
                // Store `pid` in your active tracking table for monitoring/reaping later
                Ok(pid)
            }
        }

        ServiceType::Oneshot => {
            let pid = unsafe { libc::fork() };
            if pid < 0 {
                return Err("Fork failed");
            }

            if pid == 0 {
                // --- CHILD PROCESS ---
                unsafe { libc::execve(
                    path.as_ptr() as *const c_char,
                    argv.as_ptr(),
                    envp.as_ptr(),
                ); }
                unsafe { libc::_exit(127); }
            } else {
                // --- PARENT (SUPERVISOR) ---
                // Block until the oneshot completes
                let mut status: i32 = 0;
                let wait_res = unsafe { libc::waitpid(pid, &mut status, 0) };
                
                if wait_res < 0 {
                    return Err("Waitpid failed");
                }

                // Check if process exited normally with status 0
                if libc::WIFEXITED(status) {
                    let exit_code = libc::WEXITSTATUS(status);
                    if exit_code == 0 {
                        Ok(0) // Oneshot succeeded, continue startup
                    } else {
                        Err("Oneshot failed with non-zero exit code")
                    }
                } else {
                    Err("Oneshot crashed or was terminated")
                }
            }
        }

        ServiceType::Forking => {
            // For forking daemons, setting up a pipe ensures the supervisor
            // can safely synchronize and track the final daemon PID.
            let mut pipe_fds = [0; 2];
            if unsafe { libc::pipe(pipe_fds.as_mut_ptr()) } < 0 {
                return Err("Pipe creation failed");
            }

            let pid = unsafe { libc::fork() };
            if pid < 0 {
                return Err("Fork failed");
            }

            if pid == 0 {
                // --- FIRST CHILD ---
                // Close read end
                unsafe { libc::close(pipe_fds[0]); }

                let grandchild_pid = unsafe { libc::fork() };
                if grandchild_pid == 0 {
                    // --- GRANDCHILD (Actual Daemon) ---
                    // Exec into service
                    unsafe { libc::execve(
                    path.as_ptr() as *const c_char,
                    argv.as_ptr(),
                    envp.as_ptr(),
                ); }
                    unsafe { libc::_exit(127); }
                } else {
                    // --- FIRST CHILD (Intermediate) ---
                    // Write the grandchild PID through the pipe to the supervisor, then exit
                    if grandchild_pid > 0 {
                        unsafe { libc::write(
                            pipe_fds[1],
                            &grandchild_pid as *const i32 as *const c_void,
                            core::mem::size_of::<i32>(),
                        ); }
                    }
                    unsafe { libc::close(pipe_fds[1]); }
                    unsafe { libc::_exit(0); }
                }
            } else {
                // --- PARENT (SUPERVISOR) ---
                unsafe { libc::close(pipe_fds[1]); }// Close write end

                let mut daemon_pid: i32 = 0;
                unsafe { libc::read(
                    pipe_fds[0],
                    &mut daemon_pid as *mut i32 as *mut c_void,
                    core::mem::size_of::<i32>(),
                ); }
                unsafe { libc::close(pipe_fds[0]); }

                // Reap the intermediate first child
                let mut status: i32 = 0;
                unsafe { libc::waitpid(pid, &mut status, 0); }

                if daemon_pid > 0 {
                    // Track `daemon_pid` in your supervisor process table
                    Ok(daemon_pid)
                } else {
                    Err("Failed to acquire forking daemon PID")
                }
            }
        }
    }
}

pub fn run_target(unit: &Unit) -> Result<i32, &'static str> {
    let UnitKind::Target(t) = &unit.kind else {
        return Err("Not a target");
    };

    Ok(0)
}

pub fn run_socket(unit: &Unit) -> Result<i32, &'static str> {
    let UnitKind::Socket(s) = &unit.kind else {
        return Err("Not a socket");
    };

    Ok(0)
}

pub fn run_timer(unit: &Unit) -> Result<i32, &'static str> {
    let UnitKind::Timer(t) = &unit.kind else {
        return Err("Not a timer");
    };

    Ok(0)
}

pub fn run_mount(unit: &Unit) -> Result<i32, &'static str> {
    let UnitKind::Mount(m) = &unit.kind else {
        return Err("Not a mount");
    };

    Ok(0)
}

pub fn run_automount(unit: &Unit) -> Result<i32, &'static str> {
    let UnitKind::Automount(a) = &unit.kind else {
        return Err("Not an automount");
    };

    Ok(0)
}

pub fn run_swap(unit: &Unit) -> Result<i32, &'static str> {
    let UnitKind::Swap(s) = &unit.kind else {
        return Err("Not a swap");
    };

    Ok(0)
}

pub fn run_path(unit: &Unit) -> Result<i32, &'static str> {
    let UnitKind::Path(p) = &unit.kind else {
        return Err("Not a path");
    };

    Ok(0)
}

pub fn run_device(unit: &Unit) -> Result<i32, &'static str> {
    let UnitKind::Device(d) = &unit.kind else {
        return Err("Not a device");
    };

    Ok(0)
}

pub fn run_slice(unit: &Unit) -> Result<i32, &'static str> {
    let UnitKind::Slice(s) = &unit.kind else {
        return Err("Not a slice");
    };

    Ok(0)
}

pub fn run_scope(unit: &Unit) -> Result<i32, &'static str> {
    let UnitKind::Scope(s) = &unit.kind else {
        return Err("Not a scope");
    };

    Ok(0)
}