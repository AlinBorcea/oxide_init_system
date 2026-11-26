use crate::pager::Handle;

pub const FLAG_REMAIN_AFTER_EXIT: u8 = 1 << 0; // RemainAfterExit=
pub const FLAG_PRIVATE_TMP: u8      = 1 << 1; // PrivateTmp=
pub const FLAG_NO_NEW_PRIVILEGES: u8 = 1 << 2; // NoNewPrivileges=

pub struct Unit {
    pub id: Option<Handle>,
    pub unit_section: UnitSection,
    pub install_section: Option<InstallSection>,
    pub kind: UnitKind,
    //pub service: ServiceSection,
}

pub struct UnitSection {
    //pub description_handle: Handle,
    pub after_handle: Option<Handle>,
    pub requires_handle: Option<Handle>,
    pub before_handle: Option<Handle>,
    pub wants_handle: Option<Handle>,
    pub in_degree: u16,
}

pub struct InstallSection {
    pub wanted_by_handle: Handle,
}

pub enum UnitKind {
    Service(ServiceSection),
    Target(TargetSection),
    Socket(SocketSection),
    Timer(TimerSection),
    Mount(MountSection),
    Automount(AutomountSection),
    Swap(SwapSection),
    Path(PathSection),
    Device(DeviceSection),
    Slice(SliceSection),
    Scope(ScopeSection),
}

pub struct ServiceSection {
    pub exec_start_handle: Handle,
    pub working_dir_handle: Option<Handle>,
    pub service_type: ServiceType,
    pub restart_policy: RestartPolicy,
    pub restart_sec: u16,
    pub uid: u32,
    pub gid: u32,
    pub flags: u8,
}

pub struct TargetSection {
    // Targets are weightless synchronization barriers; fields can be added here if needed.
}

pub struct SocketSection {
    pub listen_stream_handle: Option<Handle>,
    pub listen_datagram_handle: Option<Handle>,
    pub listen_fifo_handle: Option<Handle>,
    pub unit_handle: Option<Handle>,
    pub backlog: u16,
    pub socket_mode: u16,
    pub flags: u8,
}

pub struct TimerSection {
    pub on_boot_sec: u32,
    pub on_active_sec: u32,
    pub on_unit_active_sec: u32,
    pub unit_handle: Option<Handle>,
    pub flags: u8,
}

pub struct MountSection {
    pub what_handle: Handle,
    pub where_handle: Handle,
    pub fstype_handle: Option<Handle>,
    pub options_handle: Option<Handle>,
    pub mount_flags: u32,
    pub flags: u8,
}

pub struct AutomountSection {
    pub where_handle: Handle,
    pub unit_handle: Option<Handle>,
    pub timeout_idle_sec: u32,
    pub flags: u8,
}

pub struct SwapSection {
    pub what_handle: Handle,
    pub priority: i32,
    pub flags: u8,
}

pub struct PathSection {
    pub path_exists_handle: Option<Handle>,
    pub path_changed_handle: Option<Handle>,
    pub path_modified_handle: Option<Handle>,
    pub unit_handle: Option<Handle>,
    pub flags: u8,
}

pub struct DeviceSection {
    pub sysfs_path_handle: Option<Handle>,
    pub udev_tag_handle: Option<Handle>,
    pub flags: u8,
}

pub struct SliceSection {
    pub parent_slice_handle: Option<Handle>,
    pub cpu_weight: u16,
    pub memory_max_bytes: u64,
    pub io_weight: u16,
    pub flags: u8,
}

pub struct ScopeSection {
    pub slice_handle: Option<Handle>,
    pub pids_handle: Option<Handle>,
    pub flags: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SocketKind {
    Stream = 0,
    Datagram = 1,
    Fifo = 2,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ServiceType {
    Simple = 0,
    Forking = 1,
    Oneshot = 2,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RestartPolicy {
    No = 0,
    Always = 1,
    OnFailure = 2,
}

fn default_unit_section() -> UnitSection {
    UnitSection {
        after_handle: None,
        requires_handle: None,
        before_handle: None,
        wants_handle: None,
        in_degree: 0,
    }
}

pub fn default_service_unit() -> Unit {
    Unit {
        id: None,
        install_section: None,
        unit_section: default_unit_section(),
        kind: UnitKind::Service(ServiceSection {
            exec_start_handle: Handle::new(0),
            working_dir_handle: None,
            service_type: ServiceType::Simple,
            restart_policy: RestartPolicy::No,
            restart_sec: 0,
            uid: 0,
            gid: 0,
            flags: 0,
        }),
    }
}

pub fn default_target_unit() -> Unit {
    Unit {
        id: None,
        install_section: None,
        unit_section: default_unit_section(),
        kind: UnitKind::Target(TargetSection {}),
    }
}

pub fn default_socket_unit() -> Unit {
    Unit {
        id: None,
        install_section: None,
        unit_section: default_unit_section(),
        kind: UnitKind::Socket(SocketSection {
            listen_stream_handle: None,
            listen_datagram_handle: None,
            listen_fifo_handle: None,
            unit_handle: None,
            backlog: 0,
            socket_mode: 0,
            flags: 0,
        }),
    }
}

pub fn default_timer_unit() -> Unit {
    Unit {
        id: None,
        install_section: None,
        unit_section: default_unit_section(),
        kind: UnitKind::Timer(TimerSection {
            on_boot_sec: 0,
            on_active_sec: 0,
            on_unit_active_sec: 0,
            unit_handle: None,
            flags: 0,
        }),
    }
}

pub fn default_mount_unit() -> Unit {
    Unit {
        id: None,
        install_section: None,
        unit_section: default_unit_section(),
        kind: UnitKind::Mount(MountSection {
            what_handle: Handle::new(0),
            where_handle: Handle::new(0),
            fstype_handle: None,
            options_handle: None,
            mount_flags: 0,
            flags: 0,
        }),
    }
}

pub fn default_automount_unit() -> Unit {
    Unit {
        id: None,
        install_section: None,
        unit_section: default_unit_section(),
        kind: UnitKind::Automount(AutomountSection {
            where_handle: Handle::new(0),
            unit_handle: None,
            timeout_idle_sec: 0,
            flags: 0,
        }),
    }
}

pub fn default_swap_unit() -> Unit {
    Unit {
        id: None,
        install_section: None,
        unit_section: default_unit_section(),
        kind: UnitKind::Swap(SwapSection {
            what_handle: Handle::new(0),
            priority: 0,
            flags: 0,
        }),
    }
}

pub fn default_path_unit() -> Unit {
    Unit {
        id: None,
        install_section: None,
        unit_section: default_unit_section(),
        kind: UnitKind::Path(PathSection {
            path_exists_handle: None,
            path_changed_handle: None,
            path_modified_handle: None,
            unit_handle: None,
            flags: 0,
        }),
    }
}

pub fn default_device_unit() -> Unit {
    Unit {
        id: None,
        install_section: None,
        unit_section: default_unit_section(),
        kind: UnitKind::Device(DeviceSection {
            sysfs_path_handle: None,
            udev_tag_handle: None,
            flags: 0,
        }),
    }
}

pub fn default_slice_unit() -> Unit {
    Unit {
        id: None,
        install_section: None,
        unit_section: default_unit_section(),
        kind: UnitKind::Slice(SliceSection {
            parent_slice_handle: None,
            cpu_weight: 0,
            memory_max_bytes: 0,
            io_weight: 0,
            flags: 0,
        }),
    }
}

pub fn default_scope_unit() -> Unit {
    Unit {
        id: None,
        install_section: None,
        unit_section: default_unit_section(),
        kind: UnitKind::Scope(ScopeSection {
            slice_handle: None,
            pids_handle: None,
            flags: 0,
        }),
    }
}