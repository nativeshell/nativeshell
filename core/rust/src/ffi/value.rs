use std::{
    ffi::CString,
    mem::ManuallyDrop,
    os::raw::{c_char, c_void},
};

use std::ffi::CStr;

#[derive(Clone, Debug, PartialEq)]
/// Represents a value that can be send to a dart port.
pub enum DartValue {
    Null,
    Bool(bool),
    I32(i32),
    I64(i64),
    Double(f64),
    String(CString),
    Array(Vec<DartValue>),
    I8List(Vec<i8>),
    U8List(Vec<u8>),
    I16List(Vec<i16>),
    U16List(Vec<u16>),
    I32List(Vec<i32>),
    U32List(Vec<u32>),
    I64List(Vec<i64>),
    U64List(Vec<u64>),
    F32List(Vec<f32>),
    F64List(Vec<f64>),
    SendPort(raw::DartCObjectSendPort),
    Capability(raw::DartCObjectCapability),
    NativePointer(raw::DartCObjectNativePointer),
    ExternalTypedData(raw::DartCObjectExternalTypedData),
    Unsupported,
}

macro_rules! impl_from {
    ($variant:path, $for_type:ty) => {
        impl From<$for_type> for DartValue {
            fn from(v: $for_type) -> DartValue {
                $variant(v.into())
            }
        }
    };
}

impl_from!(DartValue::Bool, bool);
impl_from!(DartValue::I32, i32);
impl_from!(DartValue::I64, i64);
impl_from!(DartValue::Double, f64);
impl_from!(DartValue::String, CString);
impl_from!(DartValue::Array, Vec<DartValue>);
impl_from!(DartValue::U8List, Vec<u8>);
impl_from!(DartValue::I8List, Vec<i8>);
impl_from!(DartValue::U16List, Vec<u16>);
impl_from!(DartValue::I16List, Vec<i16>);
impl_from!(DartValue::U32List, Vec<u32>);
impl_from!(DartValue::I32List, Vec<i32>);
impl_from!(DartValue::I64List, Vec<i64>);
impl_from!(DartValue::F32List, Vec<f32>);
impl_from!(DartValue::F64List, Vec<f64>);
impl_from!(DartValue::SendPort, raw::DartCObjectSendPort);
impl_from!(DartValue::Capability, raw::DartCObjectCapability);
impl_from!(DartValue::NativePointer, raw::DartCObjectNativePointer);
impl_from!(
    DartValue::ExternalTypedData,
    raw::DartCObjectExternalTypedData
);

impl From<()> for DartValue {
    fn from(_: ()) -> Self {
        DartValue::Null
    }
}

impl From<&str> for DartValue {
    fn from(str: &str) -> Self {
        CString::new(str).unwrap().into()
    }
}

impl From<String> for DartValue {
    fn from(string: String) -> Self {
        DartValue::from(string.as_str())
    }
}

impl DartValue {
    /// Creates a DartValue instance from raw DartCObject. Any data in the
    /// object is copied.
    ///
    /// # Safety
    /// Unsafe because the object must point to a valid DartCObject instance.
    pub unsafe fn from_dart(object: *const raw::DartCObject) -> Self {
        use raw::DartCObjectType;
        let object = &*object;
        match object.ty {
            DartCObjectType::Null => DartValue::Null,
            DartCObjectType::Bool => DartValue::Bool(object.value.as_bool),
            DartCObjectType::Int32 => DartValue::I32(object.value.as_int32),
            DartCObjectType::Int64 => DartValue::I64(object.value.as_int64),
            DartCObjectType::Double => DartValue::Double(object.value.as_double),
            DartCObjectType::String => {
                DartValue::String(CStr::from_ptr(object.value.as_string).into())
            }
            DartCObjectType::Array => {
                let array = &object.value.as_array;
                let mut res = Vec::new();
                res.reserve(array.length as usize);
                for i in 0..array.length {
                    let value = array.values.offset(i);
                    let value = *value;
                    res.push(DartValue::from_dart(value));
                }
                DartValue::Array(res)
            }
            DartCObjectType::TypedData => {
                let typed_data = &object.value.as_typed_data;
                Self::from_typed_data(typed_data.ty, typed_data.values, typed_data.length)
            }
            DartCObjectType::ExternalTypedData => {
                let typed_data = &object.value.as_external_typed_data;
                Self::from_typed_data(typed_data.ty, typed_data.data, typed_data.length)
            }
            DartCObjectType::SendPort => DartValue::SendPort(object.value.as_send_port),
            DartCObjectType::Capability => DartValue::Capability(object.value.as_capability),
            DartCObjectType::NativePointer => {
                todo!("Implement NativePointer value");
            }
            DartCObjectType::Unsupported => DartValue::Unsupported,
            DartCObjectType::NumberOfTypes => {
                panic!("CObjectType::NumberOfTypes is not a valid type");
            }
        }
    }

    unsafe fn vec_from_data<T: Copy>(values: *const u8, len: isize) -> Vec<T> {
        let slice = std::slice::from_raw_parts(values as *const T, len as usize);
        slice.to_vec()
    }

    unsafe fn from_typed_data(ty: raw::DartTypedDataType, ptr: *const u8, len: isize) -> DartValue {
        use raw::DartTypedDataType;
        match ty {
            DartTypedDataType::ByteData => DartValue::U8List(Self::vec_from_data(ptr, len)),
            DartTypedDataType::Int8 => DartValue::I8List(Self::vec_from_data(ptr, len)),
            DartTypedDataType::Uint8 => DartValue::U8List(Self::vec_from_data(ptr, len)),
            DartTypedDataType::Uint8Clamped => todo!("Uint8Clamped typed data is not implemented"),
            DartTypedDataType::Int16 => DartValue::I16List(Self::vec_from_data(ptr, len)),
            DartTypedDataType::Uint16 => DartValue::U16List(Self::vec_from_data(ptr, len)),
            DartTypedDataType::Int32 => DartValue::I32List(Self::vec_from_data(ptr, len)),
            DartTypedDataType::Uint32 => DartValue::U32List(Self::vec_from_data(ptr, len)),
            DartTypedDataType::Int64 => DartValue::I64List(Self::vec_from_data(ptr, len)),
            DartTypedDataType::Uint64 => DartValue::U64List(Self::vec_from_data(ptr, len)),
            DartTypedDataType::Float32 => DartValue::F32List(Self::vec_from_data(ptr, len)),
            DartTypedDataType::Float64 => DartValue::F64List(Self::vec_from_data(ptr, len)),
            DartTypedDataType::Float32x4 => todo!("Float32x4 typed data is not implemented"),
            DartTypedDataType::Invalid => panic!("Invalid TypedDataType"),
        }
    }
}

/// Low level representation that can be used by FFI functions.
pub mod raw {
    pub type DartPort = i64;
    use super::{c_char, c_void, CString};

    pub type DartHandleFinalizer =
        unsafe extern "C" fn(isolate_callback_data: *mut c_void, peer: *mut c_void);

    #[repr(i32)]
    #[derive(Copy, Clone, PartialEq, Debug)]
    pub enum DartTypedDataType {
        ByteData = 0,
        Int8 = 1,
        Uint8 = 2,
        Uint8Clamped = 3,
        Int16 = 4,
        Uint16 = 5,
        Int32 = 6,
        Uint32 = 7,
        Int64 = 8,
        Uint64 = 9,
        Float32 = 10,
        Float64 = 11,
        Float32x4 = 12,
        Invalid = 13,
    }

    #[repr(i32)]
    #[derive(PartialEq, Debug, Clone, Copy)]
    pub enum DartCObjectType {
        Null = 0,
        Bool = 1,
        Int32 = 2,
        Int64 = 3,
        Double = 4,
        String = 5,
        Array = 6,
        TypedData = 7,
        ExternalTypedData = 8,
        SendPort = 9,
        Capability = 10,
        NativePointer = 11,
        Unsupported = 12,
        NumberOfTypes = 13,
    }

    #[repr(C)]
    pub struct DartCObject {
        pub ty: DartCObjectType,
        pub value: DartCObjectValue,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub union DartCObjectValue {
        pub as_bool: bool,
        pub as_int32: i32,
        pub as_int64: i64,
        pub as_double: f64,
        pub as_string: *mut c_char,
        pub as_send_port: DartCObjectSendPort,
        pub as_capability: DartCObjectCapability,
        pub as_array: DartCObjectArray,
        pub as_typed_data: DartCObjectTypedData,
        pub as_external_typed_data: DartCObjectExternalTypedData,
        pub as_native_pointer: DartCObjectNativePointer,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Hash)]
    pub struct DartCObjectSendPort {
        pub id: DartPort,
        pub origin_id: DartPort,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Hash)]
    pub struct DartCObjectCapability {
        pub id: i64,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct DartCObjectArray {
        pub length: isize,
        pub values: *mut *mut DartCObject,
        // Keep vector capacity here so that we can free it later;
        pub capacity: isize,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct DartCObjectTypedData {
        pub ty: DartTypedDataType,
        pub length: isize, // in elements, not bytes
        pub values: *mut u8,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone, PartialEq)]
    pub struct DartCObjectExternalTypedData {
        pub ty: DartTypedDataType,
        pub length: isize, // in elements, not bytes
        pub data: *mut u8,
        pub peer: *mut c_void,
        pub callback: DartHandleFinalizer,
    }

    unsafe impl Send for DartCObjectExternalTypedData {}

    #[repr(C)]
    #[derive(Debug, Copy, Clone, PartialEq)]
    pub struct DartCObjectNativePointer {
        pub ptr: isize,
        pub size: isize,
        pub callback: DartHandleFinalizer,
    }

    impl DartCObject {
        /// Implements manual clean-up. This is required when sending CObject to dart port fails.
        pub fn cleanup(&mut self) {
            match self.ty {
                DartCObjectType::Array => unsafe {
                    let array = &mut self.value.as_array;
                    for i in 0..array.length {
                        let value = array.values.offset(i);
                        let value = *value;
                        let value = &mut *value;
                        value.cleanup();
                    }
                },
                DartCObjectType::ExternalTypedData => unsafe {
                    let cb = self.value.as_external_typed_data.callback;
                    cb(std::ptr::null_mut(), self.value.as_external_typed_data.peer);
                },
                DartCObjectType::NativePointer => unsafe {
                    let cb = self.value.as_native_pointer.callback;
                    cb(
                        std::ptr::null_mut(),
                        self.value.as_native_pointer.ptr as *mut c_void,
                    );
                },
                _ => {}
            }
        }
    }

    impl Drop for DartCObject {
        fn drop(&mut self) {
            match self.ty {
                DartCObjectType::String => {
                    unsafe { CString::from_raw(self.value.as_string) };
                }
                DartCObjectType::Array => unsafe {
                    let array = Vec::<*mut DartCObject>::from_raw_parts(
                        self.value.as_array.values,
                        self.value.as_array.length as usize,
                        self.value.as_array.capacity as usize,
                    );
                    let _ = array.into_iter().map(|o| Box::from_raw(o));
                },
                _ => {}
            }
        }
    }
}

pub trait IntoDart {
    /// Consumes `Self` and Performs the conversion.
    fn into_dart(self) -> raw::DartCObject;
}

impl IntoDart for () {
    fn into_dart(self) -> raw::DartCObject {
        raw::DartCObject {
            ty: raw::DartCObjectType::Null,
            value: raw::DartCObjectValue { as_bool: false },
        }
    }
}

impl IntoDart for i32 {
    fn into_dart(self) -> raw::DartCObject {
        raw::DartCObject {
            ty: raw::DartCObjectType::Int32,
            value: raw::DartCObjectValue { as_int32: self },
        }
    }
}

impl IntoDart for i64 {
    fn into_dart(self) -> raw::DartCObject {
        raw::DartCObject {
            ty: raw::DartCObjectType::Int64,
            value: raw::DartCObjectValue { as_int64: self },
        }
    }
}

impl IntoDart for f64 {
    fn into_dart(self) -> raw::DartCObject {
        raw::DartCObject {
            ty: raw::DartCObjectType::Double,
            value: raw::DartCObjectValue { as_double: self },
        }
    }
}

impl IntoDart for bool {
    fn into_dart(self) -> raw::DartCObject {
        raw::DartCObject {
            ty: raw::DartCObjectType::Bool,
            value: raw::DartCObjectValue { as_bool: self },
        }
    }
}

impl IntoDart for String {
    fn into_dart(self) -> raw::DartCObject {
        let s = CString::new(self).unwrap_or_default();
        s.into_dart()
    }
}

impl IntoDart for &str {
    fn into_dart(self) -> raw::DartCObject {
        let s = CString::new(self).unwrap_or_default();
        s.into_dart()
    }
}

impl IntoDart for CString {
    fn into_dart(self) -> raw::DartCObject {
        raw::DartCObject {
            ty: raw::DartCObjectType::String,
            value: raw::DartCObjectValue {
                as_string: self.into_raw(),
            },
        }
    }
}

extern "C" fn free_vec<T>(_isolate_callback_data: *mut c_void, peer: *mut c_void) {
    let _vec: Box<Vec<T>> = unsafe { Box::from_raw(peer as *mut _) };
}

pub trait IntoDartVec<T> {
    fn dart_type() -> raw::DartTypedDataType;
}

impl IntoDartVec<u8> for u8 {
    fn dart_type() -> raw::DartTypedDataType {
        raw::DartTypedDataType::Uint8
    }
}

impl IntoDartVec<i8> for i8 {
    fn dart_type() -> raw::DartTypedDataType {
        raw::DartTypedDataType::Int8
    }
}

impl IntoDartVec<u16> for u16 {
    fn dart_type() -> raw::DartTypedDataType {
        raw::DartTypedDataType::Uint16
    }
}

impl IntoDartVec<i16> for i16 {
    fn dart_type() -> raw::DartTypedDataType {
        raw::DartTypedDataType::Int16
    }
}

impl IntoDartVec<u32> for u32 {
    fn dart_type() -> raw::DartTypedDataType {
        raw::DartTypedDataType::Uint32
    }
}

impl IntoDartVec<i32> for i32 {
    fn dart_type() -> raw::DartTypedDataType {
        raw::DartTypedDataType::Int32
    }
}

impl IntoDartVec<u64> for u64 {
    fn dart_type() -> raw::DartTypedDataType {
        raw::DartTypedDataType::Uint64
    }
}

impl IntoDartVec<i64> for i64 {
    fn dart_type() -> raw::DartTypedDataType {
        raw::DartTypedDataType::Int64
    }
}

impl IntoDartVec<f32> for f32 {
    fn dart_type() -> raw::DartTypedDataType {
        raw::DartTypedDataType::Float32
    }
}

impl IntoDartVec<f64> for f64 {
    fn dart_type() -> raw::DartTypedDataType {
        raw::DartTypedDataType::Float64
    }
}

pub struct TypedList<T>(pub T);

impl<T> IntoDart for TypedList<Vec<T>>
where
    T: IntoDartVec<T>,
{
    fn into_dart(self) -> raw::DartCObject {
        let mut vec = self.0;
        let data = vec.as_mut_ptr();
        let len = vec.len();
        // We need to know capacity to free the memory later so we move the
        // entire vector to heap
        let vec = Box::into_raw(Box::new(vec));
        raw::DartCObject {
            ty: raw::DartCObjectType::ExternalTypedData,
            value: raw::DartCObjectValue {
                as_external_typed_data: raw::DartCObjectExternalTypedData {
                    ty: T::dart_type(),
                    length: len as isize,
                    data: data as *mut u8,
                    peer: vec as *mut c_void,
                    callback: free_vec::<T>,
                },
            },
        }
    }
}

impl<T> IntoDart for Vec<T>
where
    T: IntoDart,
{
    fn into_dart(self) -> raw::DartCObject {
        let mut vec: Vec<_> = self
            .into_iter()
            .map(IntoDart::into_dart)
            .map(Box::new)
            .map(Box::into_raw)
            .collect();
        let len = vec.len();
        let capacity = vec.capacity();
        let data = vec.as_mut_ptr();
        let _ = ManuallyDrop::new(vec);
        raw::DartCObject {
            ty: raw::DartCObjectType::Array,
            value: raw::DartCObjectValue {
                as_array: raw::DartCObjectArray {
                    length: len as isize,
                    values: data,
                    capacity: capacity as isize,
                },
            },
        }
    }
}

impl IntoDart for raw::DartCObjectSendPort {
    fn into_dart(self) -> raw::DartCObject {
        raw::DartCObject {
            ty: raw::DartCObjectType::SendPort,
            value: raw::DartCObjectValue { as_send_port: self },
        }
    }
}

impl IntoDart for raw::DartCObjectCapability {
    fn into_dart(self) -> raw::DartCObject {
        raw::DartCObject {
            ty: raw::DartCObjectType::Capability,
            value: raw::DartCObjectValue {
                as_capability: self,
            },
        }
    }
}

impl IntoDart for raw::DartCObjectNativePointer {
    fn into_dart(self) -> raw::DartCObject {
        raw::DartCObject {
            ty: raw::DartCObjectType::NativePointer,
            value: raw::DartCObjectValue {
                as_native_pointer: self,
            },
        }
    }
}

impl IntoDart for raw::DartCObjectExternalTypedData {
    fn into_dart(self) -> raw::DartCObject {
        raw::DartCObject {
            ty: raw::DartCObjectType::ExternalTypedData,
            value: raw::DartCObjectValue {
                as_external_typed_data: self,
            },
        }
    }
}

impl IntoDart for DartValue {
    fn into_dart(self) -> raw::DartCObject {
        match self {
            Self::Null => ().into_dart(),
            Self::Bool(value) => value.into_dart(),
            Self::I32(value) => value.into_dart(),
            Self::I64(value) => value.into_dart(),
            Self::Double(value) => value.into_dart(),
            Self::String(value) => value.into_dart(),
            Self::Array(value) => value.into_dart(),
            Self::I8List(value) => TypedList(value).into_dart(),
            Self::U8List(value) => TypedList(value).into_dart(),
            Self::I16List(value) => TypedList(value).into_dart(),
            Self::U16List(value) => TypedList(value).into_dart(),
            Self::I32List(value) => TypedList(value).into_dart(),
            Self::U32List(value) => TypedList(value).into_dart(),
            Self::I64List(value) => TypedList(value).into_dart(),
            Self::U64List(value) => TypedList(value).into_dart(),
            Self::F32List(value) => TypedList(value).into_dart(),
            Self::F64List(value) => TypedList(value).into_dart(),
            Self::SendPort(value) => value.into_dart(),
            Self::Capability(value) => value.into_dart(),
            Self::NativePointer(value) => value.into_dart(),
            Self::ExternalTypedData(value) => value.into_dart(),
            Self::Unsupported => raw::DartCObject {
                ty: raw::DartCObjectType::Unsupported,
                value: raw::DartCObjectValue { as_bool: false },
            },
        }
    }
}
