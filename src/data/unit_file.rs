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

pub fn default_service_unit() -> Unit {
    Unit {
        id: None,
        install_section: None,
        unit_section: UnitSection { 
            after_handle: None, 
            requires_handle: None, 
            before_handle: None, 
            wants_handle: None, 
            in_degree: 0 
        },
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
        unit_section: UnitSection { 
            after_handle: None, 
            requires_handle: None, 
            before_handle: None, 
            wants_handle: None, 
            in_degree: 0 
        },
        kind: UnitKind::Target(TargetSection {}),
    }
}