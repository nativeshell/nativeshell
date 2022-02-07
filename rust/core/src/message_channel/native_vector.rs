use std::mem::ManuallyDrop;

unsafe fn allocate_vec<T: Copy + Default>(size: u64) -> *mut T {
    let mut v = Vec::<T>::with_capacity(size as usize);
    v.resize(size as usize, T::default());
    assert!(v.capacity() == v.len());
    let res = v.as_mut_ptr();
    let _ = ManuallyDrop::new(v);
    res
}

pub(super) unsafe extern "C" fn allocate_vec_u8(size: u64) -> *mut u8 {
    allocate_vec::<u8>(size)
}

pub(super) unsafe extern "C" fn allocate_vec_i16(size: u64) -> *mut i16 {
    allocate_vec::<i16>(size)
}

pub(super) unsafe extern "C" fn allocate_vec_u16(size: u64) -> *mut u16 {
    allocate_vec::<u16>(size)
}

pub(super) unsafe extern "C" fn allocate_vec_i32(size: u64) -> *mut i32 {
    allocate_vec::<i32>(size)
}

pub(super) unsafe extern "C" fn allocate_vec_u32(size: u64) -> *mut u32 {
    allocate_vec::<u32>(size)
}

pub(super) unsafe extern "C" fn allocate_vec_i64(size: u64) -> *mut i64 {
    allocate_vec::<i64>(size)
}

pub(super) unsafe extern "C" fn allocate_vec_f32(size: u64) -> *mut f32 {
    allocate_vec::<f32>(size)
}

pub(super) unsafe extern "C" fn allocate_vec_f64(size: u64) -> *mut f64 {
    allocate_vec::<f64>(size)
}

pub(super) unsafe extern "C" fn free_vec_u8(data: *mut u8, len: u64) {
    let _ = Vec::from_raw_parts(data, len as usize, len as usize);
}

pub(super) unsafe extern "C" fn free_vec_i16(data: *mut i16, len: u64) {
    let _ = Vec::from_raw_parts(data, len as usize, len as usize);
}

pub(super) unsafe extern "C" fn free_vec_u16(data: *mut u16, len: u64) {
    let _ = Vec::from_raw_parts(data, len as usize, len as usize);
}

pub(super) unsafe extern "C" fn free_vec_i32(data: *mut i32, len: u64) {
    let _ = Vec::from_raw_parts(data, len as usize, len as usize);
}

pub(super) unsafe extern "C" fn free_vec_u32(data: *mut u32, len: u64) {
    let _ = Vec::from_raw_parts(data, len as usize, len as usize);
}

pub(super) unsafe extern "C" fn free_vec_i64(data: *mut i64, len: u64) {
    let _ = Vec::from_raw_parts(data, len as usize, len as usize);
}

pub(super) unsafe extern "C" fn free_vec_f32(data: *mut f32, len: u64) {
    let _ = Vec::from_raw_parts(data, len as usize, len as usize);
}

pub(super) unsafe extern "C" fn free_vec_f64(data: *mut f64, len: u64) {
    let _ = Vec::from_raw_parts(data, len as usize, len as usize);
}

unsafe fn modify<T: Copy + Default, F: FnOnce(&mut Vec<T>)>(
    data: *mut T,
    len: u64,
    f: F,
) -> *mut T {
    let mut vec = Vec::<T>::from_raw_parts(data, len as usize, len as usize);
    f(&mut vec);
    assert!(vec.len() == vec.capacity());
    let res = vec.as_mut_ptr();
    let _ = ManuallyDrop::new(vec);
    res
}

pub(super) unsafe extern "C" fn resize_vec_u8(data: *mut u8, size: u64, new_size: u64) -> *mut u8 {
    modify(data, size, |v| {
        let new_size = new_size as usize;
        if new_size > v.capacity() {
            v.reserve_exact(new_size - v.capacity());
        }
        v.resize(new_size, 0);
        v.shrink_to_fit();
    })
}
