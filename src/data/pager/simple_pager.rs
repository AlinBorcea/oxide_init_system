use core::ffi::CStr;
use libc::{size_t, c_void};

use crate::{Pager, pager::Handle};

pub struct SimplePager {
    pages: *mut Option<Page>,
    positions: *mut PositionInPager,
    page_size: u16,
    page_length: u8,
    page_capacity: u8,
    page_resize_by: u8,
    position_length: usize,
    position_capacity: usize,
    position_resize_by: u8,
    maximum_str_size: u16,
}

#[derive(Clone, Copy)]
pub struct PositionInPager {
    page: u8,
    position_on_page: PositionOnPage
}

impl SimplePager {
    pub fn new(number_of_pages: u8, page_size: u16, maximum_str_size: u16, 
        position_capacity: usize, position_resize_by: u8, page_resize_by: u8) -> Option<Self> {

        let pages_size_bytes = core::mem::size_of::<Option<Page>>();
        let raw_pages = unsafe { 
            libc::malloc(pages_size_bytes * (number_of_pages as usize)) as *mut Option<Page> 
        };
        if raw_pages.is_null() {
            return None;
        }

        let positions_size = core::mem::size_of::<PositionInPager>();
        let raw_positions = unsafe { libc::malloc(positions_size * position_capacity) as *mut PositionInPager };
        if raw_positions.is_null() {
            unsafe { libc::free(raw_pages as *mut c_void); }
            return None;
        }

        for index in 0..number_of_pages {
            let new_page = page_new(page_size);
            unsafe { raw_pages.add(index as usize).write(new_page); }
        }

        Some(Self {
            pages: raw_pages,
            positions: raw_positions,
            page_size: page_size,
            page_length: 0,
            page_capacity: number_of_pages,
            page_resize_by: page_resize_by,
            position_length: 0,
            position_capacity: position_capacity,
            position_resize_by: position_resize_by,
            maximum_str_size: maximum_str_size,
        })
    }

    fn resize_pages(&mut self) -> bool {
        let pages_size_bytes = core::mem::size_of::<Option<Page>>();
        let new_buffer = unsafe { libc::realloc(self.pages as *mut c_void, 
            pages_size_bytes * (self.page_capacity + self.page_resize_by) as usize) 
        };

        if new_buffer.is_null() {
            return false;
        }

        let old_capacity = self.page_capacity;
        self.pages = new_buffer as *mut Option<Page>;
        self.page_capacity += self.page_resize_by;

        for index in old_capacity..self.page_capacity {
            let page = page_new(self.page_size);
            if page.is_none() {
                return false;
            }
            unsafe { self.pages.add(index as usize).write(page); }
        }

        true
    }

    fn resize_positions(&mut self) -> bool {
        let position_size = core::mem::size_of::<PositionInPager>();
        let new_bufer = unsafe { libc::realloc(self.positions as *mut c_void, 
            position_size * (self.position_capacity + self.position_resize_by as usize)) 
        };
        if new_bufer.is_null() {
            return false;
        }

        self.positions = new_bufer as *mut PositionInPager;
        self.position_capacity += self.position_resize_by as usize;
        true
    }

}

impl Pager for SimplePager {
    fn delete(&self) {
        for index in 0..self.page_capacity {
            let page = unsafe { self.pages.add(index as usize).read() };
            if let None = page {
                continue;
            }

            page_delete(&page.unwrap());
        }
        unsafe { libc::free(self.pages as *mut c_void); }
        unsafe { libc::free(self.positions as *mut c_void); }
    }

    fn add_str(&mut self, str: &CStr) -> Option<Handle> {
        if str.count_bytes() > self.maximum_str_size as usize {
            return None;
        }

        if self.page_length >= self.page_capacity && self.page_capacity < 255 {
            if !self.resize_pages() {
                return None;
            }
        }

        if self.position_length >= self.position_capacity {
            if !self.resize_positions() {
                return None;
            }
        }

        for index in self.page_length..self.page_capacity {
            
            let page_ptr = unsafe { self.pages.add(index as usize) };
            let page_optional = unsafe { &mut *page_ptr };
            if let None = page_optional {
                continue;
            }

            let page = page_optional.as_mut().unwrap();
            let position_on_page = page_add(page, str);
            if let None = position_on_page {
                self.page_length = index + 1;
                continue;
            }

            let position = PositionInPager {
                page: index,
                position_on_page: position_on_page.unwrap()
            };

            unsafe { self.positions.add(self.position_length).write(position) };
            self.position_length += 1;

            return Some(Handle::new(self.position_length-1));
        }

        if self.page_capacity < 255 {
            return self.add_str(str);
        }

        None
    }

    fn find(&self, handle: &Handle) -> &CStr {
        let position_raw = unsafe { self.positions.add(handle.value()) };
        let position = unsafe { *position_raw };
        let page_ptr = unsafe { self.pages.add(position.page as usize) }; 
        let page_optional = unsafe { &*page_ptr };
        let page = page_optional.as_ref().unwrap();
        page_find(page, &position.position_on_page)
    }

}

pub struct Page {
    raw_ptr: *mut u8,
    length: u16,
    capacity: u16
}

#[derive(Clone, Copy)]
pub struct PositionOnPage {
    index: u16,
    length: u16,
}

pub fn page_new(size: u16) -> Option<Page> {
    let raw_ptr = unsafe { libc::malloc(size as size_t) as *mut u8 };
    if raw_ptr.is_null() {
        return None;
    }

    Some(Page {
        raw_ptr: raw_ptr,
        length: 0,
        capacity: size
    })
}

pub fn page_delete(page: &Page) {
    unsafe { libc::free(page.raw_ptr as *mut c_void); }
}

pub fn page_add(page: &mut Page, str: &CStr) -> Option<PositionOnPage> {
    if !page_has_room(page, str) {
        return None;
    }

    let bytes = str.to_bytes_with_nul();
    let bytes_len = bytes.len() as u16;

    let start_index = page.length;
    
    unsafe {
        let ptr = page.raw_ptr.add(page.length as usize);
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
    }

    page.length += bytes_len;
    Some(PositionOnPage { 
        index: start_index, 
        length: bytes_len 
    })
}

pub fn page_find<'a>(page: &'a Page, position: &PositionOnPage) -> &'a CStr {
    let raw = unsafe { page.raw_ptr.add(position.index as usize) };
    unsafe { CStr::from_ptr(raw as *const i8) }
}

pub fn page_has_room(page: &Page, str: &CStr) -> bool {
    // bytes needed <= bytes available
    str.count_bytes() + 1 <= (page.capacity - page.length) as usize
}