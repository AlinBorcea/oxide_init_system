use core::ffi::CStr;

pub trait Pager {
    fn add_str(&mut self, str: &CStr) -> Option<Handle>;
    fn find(&self, handle: &Handle) -> &CStr;
    fn delete(&self);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Handle(usize);

impl Handle {
    pub(crate) fn new(val: usize) -> Self {
        Handle(val)
    }

    pub(crate) fn value(&self) -> usize {
        self.0
    }
}