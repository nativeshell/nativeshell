use crate::{value::Value, DartValue};

const VALUE_NULL: u8 = 255 - 0;
const VALUE_TRUE: u8 = 255 - 1;
const VALUE_FALSE: u8 = 255 - 2;
const VALUE_INT64: u8 = 255 - 3;
const VALUE_FLOAT64: u8 = 255 - 4;
const VALUE_SMALL_STRING: u8 = 255 - 5;

// Deserialization
const VALUE_STRING: u8 = 255 - 6;
const VALUE_INT8LIST: u8 = 255 - 7;
const VALUE_UINT8LIST: u8 = 255 - 8;
const VALUE_INT16LIST: u8 = 255 - 9;
const VALUE_UINT16LIST: u8 = 255 - 10;
const VALUE_INT32LIST: u8 = 255 - 11;
const VALUE_UINT32LIST: u8 = 255 - 12;
const VALUE_INT64LIST: u8 = 255 - 13;
const VALUE_FLOAT32LIST: u8 = 255 - 14;
const VALUE_FLOAT64LIST: u8 = 255 - 15;

// Serialization
const VALUE_ATTACHMENT: u8 = VALUE_STRING; // this will be passed directly as Dart_CObject
const VALUE_NATIVE_POINTER: u8 = VALUE_ATTACHMENT - 1;

const VALUE_LIST: u8 = 255 - 16;
const VALUE_MAP: u8 = 255 - 17;
const VALUE_LAST: u8 = VALUE_MAP;

pub(super) struct Deserializer {}

impl Deserializer {
    pub unsafe fn deserialize(buf: &[u8]) -> Value {
        let mut reader = Reader::new(buf);
        Self::read_value(&mut reader)
    }

    unsafe fn read_value(reader: &mut Reader) -> Value {
        if reader.ended() {
            panic!("Malformed stream");
        }
        let t = reader.read_u8();
        if t < VALUE_LAST {
            return Value::I64(t as i64);
        }
        match t {
            VALUE_NULL => Value::Null,
            VALUE_FALSE => Value::Bool(false),
            VALUE_TRUE => Value::Bool(true),
            VALUE_INT64 => Value::I64(reader.read_i64()),
            VALUE_FLOAT64 => {
                reader.align_to(8);
                Value::F64(reader.read_f64())
            }
            VALUE_SMALL_STRING => {
                let len = reader.read_size();
                Value::String(reader.read_string(len))
            }
            VALUE_STRING => {
                let vec = Self::read_vec::<u8>(reader);
                Value::String(String::from_utf8_unchecked(vec))
            }
            VALUE_INT8LIST => Value::I8List(Self::read_vec::<i8>(reader)),
            VALUE_UINT8LIST => Value::U8List(Self::read_vec::<u8>(reader)),
            VALUE_INT16LIST => Value::I16List(Self::read_vec::<i16>(reader)),
            VALUE_UINT16LIST => Value::U16List(Self::read_vec::<u16>(reader)),
            VALUE_INT32LIST => Value::I32List(Self::read_vec::<i32>(reader)),
            VALUE_UINT32LIST => Value::U32List(Self::read_vec::<u32>(reader)),
            VALUE_INT64LIST => Value::I64List(Self::read_vec::<i64>(reader)),
            VALUE_FLOAT32LIST => Value::F32List(Self::read_vec::<f32>(reader)),
            VALUE_FLOAT64LIST => Value::F64List(Self::read_vec::<f64>(reader)),
            VALUE_LIST => {
                let len = reader.read_size();
                let mut list = Vec::new();
                list.reserve(len);
                for _ in 0..len {
                    let value = Self::read_value(reader);
                    list.push(value);
                }
                Value::List(list)
            }
            VALUE_MAP => {
                let len = reader.read_size();
                let mut map = Vec::<(Value, Value)>::new();
                for _ in 0..len {
                    let k = Self::read_value(reader);
                    let v = Self::read_value(reader);
                    map.push((k, v));
                }
                Value::Map(map.into())
            }
            _ => {
                panic!("Unsupported value type: {}", t);
            }
        }
    }

    unsafe fn read_vec<T>(reader: &mut Reader) -> Vec<T> {
        let ptr = reader.read_u64();
        let size = reader.read_size() as u64;
        Vec::<T>::from_raw_parts(ptr as *mut T, size as usize, size as usize)
    }
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Reader { buf, pos: 0 }
    }
    fn read_u8(&mut self) -> u8 {
        let n = self.buf[self.pos];
        self.pos += 1;
        n
    }
    fn read_u16(&mut self) -> u16 {
        self.pos += 2;
        let s = &self.buf[self.pos - 2..self.pos];
        u16::from_ne_bytes(clone_into_array(s))
    }
    fn read_u32(&mut self) -> u32 {
        self.pos += 4;
        let s = &self.buf[self.pos - 4..self.pos];
        u32::from_ne_bytes(clone_into_array(s))
    }
    fn read_u64(&mut self) -> u64 {
        self.pos += 8;
        let s = &self.buf[self.pos - 8..self.pos];
        u64::from_ne_bytes(clone_into_array(s))
    }
    fn read_i64(&mut self) -> i64 {
        self.pos += 8;
        let s = &self.buf[self.pos - 8..self.pos];
        i64::from_ne_bytes(clone_into_array(s))
    }
    fn read_f64(&mut self) -> f64 {
        let n = self.read_u64();
        f64::from_bits(n)
    }
    fn read_size(&mut self) -> usize {
        let n = self.read_u8();
        match n {
            254 => self.read_u16() as usize,
            255 => self.read_u32() as usize,
            _ => n as usize,
        }
    }
    fn read_string(&mut self, len: usize) -> String {
        if len == 0 {
            String::from("")
        } else {
            let v = &self.buf[self.pos..self.pos + len];
            self.pos += len;
            String::from_utf8_lossy(v).to_owned().to_string()
        }
    }
    fn align_to(&mut self, align: usize) {
        let m = self.pos % align;
        if m > 0 {
            self.pos += align - m;
        }
    }
    fn ended(&self) -> bool {
        self.pos >= self.buf.len()
    }
}

pub(super) struct Serializer {}

impl Serializer {
    pub fn serialize(value: Value) -> Vec<DartValue> {
        let mut res = Vec::new();
        let mut buf = Vec::new();
        let mut writer = Writer::new(&mut buf);
        Self::write_value(&mut writer, value, &mut res);
        res.push(DartValue::U8List(buf));
        res
    }

    fn write_value(writer: &mut Writer, value: Value, attachments: &mut Vec<DartValue>) {
        match value {
            Value::Null => {
                writer.write_u8(VALUE_NULL);
            }
            Value::Bool(v) => {
                writer.write_u8(if v { VALUE_TRUE } else { VALUE_FALSE });
            }
            Value::I64(n) => {
                if n < VALUE_LAST as i64 {
                    writer.write_u8(n as u8);
                } else {
                    writer.write_u8(VALUE_INT64);
                    writer.write_i64(n);
                }
            }
            Value::F64(n) => {
                writer.align_to(8);
                writer.write_u8(VALUE_FLOAT64);
                writer.write_f64(n);
            }
            Value::String(v) => {
                if v.len() < 50 {
                    writer.write_u8(VALUE_SMALL_STRING);
                    writer.write_size(v.len());
                    writer.write_string(&v);
                } else {
                    Self::write_attachment(writer, v, attachments);
                }
            }
            Value::I8List(v) => {
                Self::write_attachment(writer, v, attachments);
            }
            Value::U8List(v) => {
                Self::write_attachment(writer, v, attachments);
            }
            Value::I16List(v) => {
                Self::write_attachment(writer, v, attachments);
            }
            Value::U16List(v) => {
                Self::write_attachment(writer, v, attachments);
            }
            Value::I32List(v) => {
                Self::write_attachment(writer, v, attachments);
            }
            Value::U32List(v) => {
                Self::write_attachment(writer, v, attachments);
            }
            Value::I64List(v) => {
                Self::write_attachment(writer, v, attachments);
            }
            Value::F32List(v) => {
                Self::write_attachment(writer, v, attachments);
            }
            Value::F64List(v) => {
                Self::write_attachment(writer, v, attachments);
            }
            Value::List(list) => {
                writer.write_u8(VALUE_LIST);
                writer.write_size(list.len());
                list.into_iter().for_each(|v| {
                    Self::write_value(writer, v, attachments);
                });
            }
            Value::Map(map) => {
                writer.write_u8(VALUE_MAP);
                writer.write_size(map.len());
                map.into_iter().for_each(|v| {
                    Self::write_value(writer, v.0, attachments);
                    Self::write_value(writer, v.1, attachments);
                });
            }
            Value::Dart(v) => match &v {
                crate::DartObject::NativePointer(p) => {
                    Self::write_native_pointer(writer, p.pointer, v, attachments)
                }
                _ => Self::write_attachment(writer, v, attachments),
            },
        }
    }

    fn write_native_pointer<T: Into<DartValue>>(
        writer: &mut Writer,
        pointer: isize,
        v: T,
        attachments: &mut Vec<DartValue>,
    ) {
        writer.write_u8(VALUE_NATIVE_POINTER);
        writer.write_i64(pointer as i64);
        writer.write_size(attachments.len()); // current index
        attachments.push(v.into());
    }

    fn write_attachment<T: Into<DartValue>>(
        writer: &mut Writer,
        v: T,
        attachments: &mut Vec<DartValue>,
    ) {
        writer.write_u8(VALUE_ATTACHMENT);
        writer.write_size(attachments.len()); // current index
        attachments.push(v.into());
    }
}

struct Writer<'a>(&'a mut Vec<u8>);

#[allow(unused)]
impl<'a> Writer<'a> {
    fn new(v: &'a mut Vec<u8>) -> Self {
        Writer(v)
    }
    fn write_u8(&mut self, n: u8) {
        self.0.push(n);
    }
    fn write_u16(&mut self, n: u16) {
        self.0.extend_from_slice(&n.to_ne_bytes());
    }
    fn write_u32(&mut self, n: u32) {
        self.0.extend_from_slice(&n.to_ne_bytes());
    }
    fn write_i32(&mut self, n: i32) {
        self.0.extend_from_slice(&n.to_ne_bytes());
    }
    fn write_u64(&mut self, n: u64) {
        self.0.extend_from_slice(&n.to_ne_bytes());
    }
    fn write_i64(&mut self, n: i64) {
        self.0.extend_from_slice(&n.to_ne_bytes());
    }
    fn write_f64(&mut self, n: f64) {
        self.write_u64(n.to_bits());
    }
    fn write_size(&mut self, n: usize) {
        if n < 254 {
            self.write_u8(n as u8);
        } else if n <= u16::max_value() as usize {
            self.write_u8(254);
            self.write_u16(n as u16);
        } else if n < u32::max_value() as usize {
            self.write_u8(255);
            self.write_u32(n as u32);
        } else {
            // flutter only support 32 bit value
            panic!("Not implemented");
        }
    }
    fn write_string(&mut self, s: &str) {
        self.0.extend_from_slice(s.as_bytes());
    }
    fn align_to(&mut self, align: usize) {
        let m = self.0.len() % align;
        if m == 0 {
            return;
        }
        let m = align - m;
        for _ in 0..m {
            self.write_u8(0);
        }
    }
}

fn clone_into_array<A, T>(slice: &[T]) -> A
where
    A: Default + AsMut<[T]>,
    T: Clone,
{
    let mut a = A::default();
    a.as_mut().clone_from_slice(slice);
    a
}
