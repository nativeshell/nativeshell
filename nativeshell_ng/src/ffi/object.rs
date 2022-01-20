use std::{
    ffi::CString,
    mem::ManuallyDrop,
    os::raw::{c_char, c_void},
};

use std::ffi::CStr;

#[derive(Clone, Debug)]
pub enum Value {
    Null,
    Bool(bool),
    I32(i32),
    I64(i64),
    Double(f64),
    String(CString),
    Array(Vec<Value>),
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
    SendPort(raw::CObjectSendPort),
    Capability(raw::CObjectCapability),
    Unsupported,
}

impl Value {
    pub unsafe fn from_dart(object: *const raw::CObject) -> Self {
        use raw::CObjectType;
        let object = &*object;
        match object.ty {
            CObjectType::Null => Value::Null,
            CObjectType::Bool => Value::Bool(object.value.as_bool),
            CObjectType::Int32 => Value::I32(object.value.as_int32),
            CObjectType::Int64 => Value::I64(object.value.as_int64),
            CObjectType::Double => Value::Double(object.value.as_double),
            CObjectType::String => Value::String(CStr::from_ptr(object.value.as_string).into()),
            CObjectType::Array => {
                let array = &object.value.as_array;
                let mut res = Vec::new();
                res.reserve(array.length as usize);
                for i in 0..array.length {
                    let value = array.values.offset(i);
                    let value = *value;
                    res.push(Value::from_dart(value));
                }
                Value::Array(res)
            }
            CObjectType::TypedData => {
                let typed_data = &object.value.as_typed_data;
                Self::from_typed_data(typed_data.ty, typed_data.values, typed_data.length)
            }
            CObjectType::ExternalTypedData => {
                let typed_data = &object.value.as_external_typed_data;
                Self::from_typed_data(typed_data.ty, typed_data.data, typed_data.length)
            }
            CObjectType::SendPort => Value::SendPort(object.value.as_send_port.clone()),
            CObjectType::Capability => Value::Capability(object.value.as_capability.clone()),
            CObjectType::NativePointer => {
                todo!("Implement NativePointer value");
            }
            CObjectType::Unsupported => Value::Unsupported,
            CObjectType::NumberOfTypes => {
                panic!("CObjectType::NumberOfTypes is not a valid type");
            }
        }
    }

    unsafe fn vec_from_data<T: Copy>(values: *const u8, len: isize) -> Vec<T> {
        let slice = std::slice::from_raw_parts(values as *const T, len as usize);
        slice.to_vec()
    }

    unsafe fn from_typed_data(ty: raw::TypedDataType, ptr: *const u8, len: isize) -> Value {
        use raw::TypedDataType;
        match ty {
            TypedDataType::ByteData => Value::U8List(Self::vec_from_data(ptr, len)),
            TypedDataType::Int8 => Value::I8List(Self::vec_from_data(ptr, len)),
            TypedDataType::Uint8 => Value::U8List(Self::vec_from_data(ptr, len)),
            TypedDataType::Uint8Clamped => todo!("Uint8Clamped typed data is not implemented"),
            TypedDataType::Int16 => Value::I16List(Self::vec_from_data(ptr, len)),
            TypedDataType::Uint16 => Value::U16List(Self::vec_from_data(ptr, len)),
            TypedDataType::Int32 => Value::I32List(Self::vec_from_data(ptr, len)),
            TypedDataType::Uint32 => Value::U32List(Self::vec_from_data(ptr, len)),
            TypedDataType::Int64 => Value::I64List(Self::vec_from_data(ptr, len)),
            TypedDataType::Uint64 => Value::U64List(Self::vec_from_data(ptr, len)),
            TypedDataType::Float32 => Value::F32List(Self::vec_from_data(ptr, len)),
            TypedDataType::Float64 => Value::F64List(Self::vec_from_data(ptr, len)),
            TypedDataType::Float32x4 => todo!("Float32x4 typed data is not implemented"),
            TypedDataType::Invalid => panic!("Invalid TypedDataType"),
        }
    }
}

pub mod raw {
    pub type Port = i64;
    use super::{c_char, c_void, CString};

    pub type HandleFinalizer =
        unsafe extern "C" fn(isolate_callback_data: *mut c_void, peer: *mut c_void);

    #[repr(i32)]
    #[derive(Copy, Clone, PartialEq, Debug)]
    pub enum TypedDataType {
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
    pub enum CObjectType {
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
    pub struct CObject {
        pub ty: CObjectType,
        pub value: CObjectValue,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub union CObjectValue {
        pub as_bool: bool,
        pub as_int32: i32,
        pub as_int64: i64,
        pub as_double: f64,
        pub as_string: *mut c_char,
        pub as_send_port: CObjectSendPort,
        pub as_capability: CObjectCapability,
        pub as_array: CObjectArray,
        pub as_typed_data: CObjectTypedData,
        pub as_external_typed_data: CObjectExternalTypedData,
        pub as_native_pointer: CObjectNativePointer,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct CObjectSendPort {
        pub id: Port,
        pub origin_id: Port,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct CObjectCapability {
        pub id: i64,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct CObjectArray {
        pub length: isize,
        pub values: *mut *mut CObject,
        // Keep vector capacity here so that we can free it later;
        pub capacity: isize,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct CObjectTypedData {
        pub ty: TypedDataType,
        pub length: isize, // in elements, not bytes
        pub values: *mut u8,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct CObjectExternalTypedData {
        pub ty: TypedDataType,
        pub length: isize, // in elements, not bytes
        pub data: *mut u8,
        pub peer: *mut c_void,
        pub callback: HandleFinalizer,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct CObjectNativePointer {
        pub ptr: isize,
        pub size: isize,
        pub callback: HandleFinalizer,
    }

    impl CObject {
        /// Implements manual clean-up. This is required when sending CObject to dart port fails.
        pub fn cleanup(&mut self) {
            match self.ty {
                CObjectType::Array => unsafe {
                    let array = &mut self.value.as_array;
                    for i in 0..array.length {
                        let value = array.values.offset(i);
                        let value = *value;
                        let value = &mut *value;
                        value.cleanup();
                    }
                },
                CObjectType::ExternalTypedData => unsafe {
                    let cb = self.value.as_external_typed_data.callback;
                    cb(std::ptr::null_mut(), self.value.as_external_typed_data.peer);
                },
                CObjectType::NativePointer => unsafe {
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

    impl Drop for CObject {
        fn drop(&mut self) {
            match self.ty {
                CObjectType::String => {
                    unsafe { CString::from_raw(self.value.as_string) };
                }
                CObjectType::Array => unsafe {
                    let array = Vec::<*mut CObject>::from_raw_parts(
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
    fn into_dart(self) -> raw::CObject;
}

impl IntoDart for () {
    fn into_dart(self) -> raw::CObject {
        raw::CObject {
            ty: raw::CObjectType::Null,
            value: raw::CObjectValue { as_bool: false },
        }
    }
}

impl IntoDart for i32 {
    fn into_dart(self) -> raw::CObject {
        raw::CObject {
            ty: raw::CObjectType::Int32,
            value: raw::CObjectValue { as_int32: self },
        }
    }
}

impl IntoDart for i64 {
    fn into_dart(self) -> raw::CObject {
        raw::CObject {
            ty: raw::CObjectType::Int64,
            value: raw::CObjectValue { as_int64: self },
        }
    }
}

impl IntoDart for f64 {
    fn into_dart(self) -> raw::CObject {
        raw::CObject {
            ty: raw::CObjectType::Double,
            value: raw::CObjectValue { as_double: self },
        }
    }
}

impl IntoDart for bool {
    fn into_dart(self) -> raw::CObject {
        raw::CObject {
            ty: raw::CObjectType::Bool,
            value: raw::CObjectValue { as_bool: self },
        }
    }
}

impl IntoDart for String {
    fn into_dart(self) -> raw::CObject {
        let s = CString::new(self).unwrap_or_default();
        s.into_dart()
    }
}

impl IntoDart for &str {
    fn into_dart(self) -> raw::CObject {
        let s = CString::new(self).unwrap_or_default();
        s.into_dart()
    }
}

impl IntoDart for CString {
    fn into_dart(self) -> raw::CObject {
        raw::CObject {
            ty: raw::CObjectType::String,
            value: raw::CObjectValue {
                as_string: self.into_raw(),
            },
        }
    }
}

extern "C" fn free_vec<T>(_isolate_callback_data: *mut c_void, peer: *mut c_void) {
    let _vec: Box<Vec<T>> = unsafe { Box::from_raw(peer as *mut _) };
}

pub trait IntoDartVec<T> {
    fn dart_type() -> raw::TypedDataType;
}

impl IntoDartVec<u8> for u8 {
    fn dart_type() -> raw::TypedDataType {
        raw::TypedDataType::Uint8
    }
}

impl IntoDartVec<i8> for i8 {
    fn dart_type() -> raw::TypedDataType {
        raw::TypedDataType::Int8
    }
}

impl IntoDartVec<u16> for u16 {
    fn dart_type() -> raw::TypedDataType {
        raw::TypedDataType::Uint16
    }
}

impl IntoDartVec<i16> for i16 {
    fn dart_type() -> raw::TypedDataType {
        raw::TypedDataType::Int16
    }
}

impl IntoDartVec<u32> for u32 {
    fn dart_type() -> raw::TypedDataType {
        raw::TypedDataType::Uint32
    }
}

impl IntoDartVec<i32> for i32 {
    fn dart_type() -> raw::TypedDataType {
        raw::TypedDataType::Int32
    }
}

impl IntoDartVec<u64> for u64 {
    fn dart_type() -> raw::TypedDataType {
        raw::TypedDataType::Uint64
    }
}

impl IntoDartVec<i64> for i64 {
    fn dart_type() -> raw::TypedDataType {
        raw::TypedDataType::Int64
    }
}

impl IntoDartVec<f32> for f32 {
    fn dart_type() -> raw::TypedDataType {
        raw::TypedDataType::Float32
    }
}

impl IntoDartVec<f64> for f64 {
    fn dart_type() -> raw::TypedDataType {
        raw::TypedDataType::Float64
    }
}

pub struct TypedList<T>(pub T);

impl<T> IntoDart for TypedList<Vec<T>>
where
    T: IntoDartVec<T>,
{
    fn into_dart(self) -> raw::CObject {
        let mut vec = self.0;
        let data = vec.as_mut_ptr();
        let len = vec.len();
        // We need to know capacity to free the memory later so we move the
        // entire vector to heap
        let vec = Box::into_raw(Box::new(vec));
        raw::CObject {
            ty: raw::CObjectType::ExternalTypedData,
            value: raw::CObjectValue {
                as_external_typed_data: raw::CObjectExternalTypedData {
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
    fn into_dart(self) -> raw::CObject {
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
        raw::CObject {
            ty: raw::CObjectType::Array,
            value: raw::CObjectValue {
                as_array: raw::CObjectArray {
                    length: len as isize,
                    values: data,
                    capacity: capacity as isize,
                },
            },
        }
    }
}

impl IntoDart for raw::CObjectSendPort {
    fn into_dart(self) -> raw::CObject {
        raw::CObject {
            ty: raw::CObjectType::SendPort,
            value: raw::CObjectValue { as_send_port: self },
        }
    }
}

impl IntoDart for raw::CObjectCapability {
    fn into_dart(self) -> raw::CObject {
        raw::CObject {
            ty: raw::CObjectType::Capability,
            value: raw::CObjectValue {
                as_capability: self,
            },
        }
    }
}

impl IntoDart for Value {
    fn into_dart(self) -> raw::CObject {
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
            Self::Unsupported => raw::CObject {
                ty: raw::CObjectType::Unsupported,
                value: raw::CObjectValue { as_bool: false },
            },
        }
    }
}
