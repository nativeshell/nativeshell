use std::{convert::TryFrom, error::Error, ops::Add};

use nativeshell_core::{TryFromError, Value};
use nativeshell_derive::{IntoValue, TryFromValue};

#[derive(IntoValue, TryFromValue, Debug)]
struct Address {
    street: String,
    city: String,
    number: i64,
    number2: Option<i64>,
}

#[derive(IntoValue, TryFromValue, Debug)]
// #[nativeshell(rename_all = "camelCase")]
struct Person<T: Into<Value> + TryFrom<Value, Error = TryFromError>> {
    #[nativeshell(rename = "my_name")]
    name: String,
    age: T,
    address: Address,
    work_address: Option<Address>,
    test: Option<i64>,
    test2: Option<i64>,
    v: Value,
}

#[derive(IntoValue, TryFromValue, Debug)]
struct Person2<T: Into<Value> + TryFrom<Value, Error = TryFromError>> {
    #[nativeshell(skip)]
    name: String,
    age: T,
    address: Address,
    work_address: Option<Address>,
    test: Option<i64>,
    test2: Option<i64>,
    v: Value,
    new_field: Option<i64>,
    // another_field: i64,
}

mod tests {
    use std::{
        convert::{TryFrom, TryInto},
        ops::Add,
    };

    use nativeshell_core::Value;

    use crate::{Address, Person, Person2};

    struct Wrap<'a, T>(&'a mut T);

    trait Assign {
        fn assign(&mut self, value: nativeshell_core::Value);
    }

    impl<'a, T: TryFrom<Value>> Assign for Wrap<'a, Option<T>> {
        fn assign(&mut self, value: nativeshell_core::Value) {
            self.0.replace(value.try_into().ok().unwrap());
        }
    }

    impl<'a, T: TryFrom<Value>> Assign for &mut Wrap<'a, Option<Option<T>>> {
        fn assign(&mut self, value: nativeshell_core::Value) {
            match value {
                nativeshell_core::Value::Null => self.0.replace(Option::<T>::None),
                v => self.0.replace(Some(v.try_into().ok().unwrap())),
            };
        }
    }

    #[test]
    fn test1() {
        let mut x = Option::<Option<String>>::None;
        let mut y = Option::<String>::None;
        let mut z = Option::<Option<String>>::None;

        (&mut &mut Wrap(&mut x)).assign(Value::String("ABCD".into()));
        (&mut &mut Wrap(&mut y)).assign(Value::String("XYZ".into()));
        (&mut &mut Wrap(&mut z)).assign(Value::Null);

        println!("X {:?}", x);
        println!("Y {:?}", y);
        println!("Z {:?}", z);

        let address = Address {
            street: "Street".into(),
            city: "City".into(),
            number: 10,
            number2: None,
        };
        let person = Person {
            name: "My Name".into(),
            age: 19,
            test: Some(5i64),
            test2: None,
            work_address: Some(address),
            address: Address {
                street: "Racianska".into(),
                city: "Bratislava".into(),
                number: 69,
                number2: None,
            },
            v: Value::F64(43.0),
        };
        let v = Value::from(person);
        println!("V: ${:#?}", v);

        let p1: Person<i64> = v.clone().try_into().unwrap();
        println!("Person: {:#?}", p1);

        let p2: Person2<i64> = v.try_into().unwrap();
        println!("Person2: {:#?}", p2);

        let address = Address {
            street: "Racianska".into(),
            city: "Bratislava".into(),
            number: 69,
            number2: Some(33),
        };
        let address_value = Value::from(address);
        let address2: Address = address_value.try_into().unwrap();
        let s = Option::<i32>::None;
        println!(">>> ${:?}", address2);
    }
}

#[derive(IntoValue)]
// enum My1<T: std::fmt::Debug> {
// #[nativeshell(tag = "tag", content = "c")]
// #[nativeshell(tag = "t", content = "c", rename_all = "camelCase")]
enum My1<T: std::fmt::Debug + Into<Value>> {
    Abc,
    SingleValue(String),
    Double(String, String),
    FFF(T),
    #[nativeshell(rename = "boom")]
    Xyz {
        #[nativeshell(rename = "xabc")]
        a: String,
        #[nativeshell(skip)]
        b: i64,
        c: Value,
        d: T,
    },
}

#[derive(IntoValue)]
struct XYZ(String, i64, f64);

#[derive(IntoValue)]
struct XYZA(String);

// fn my<T>(m1: My1<T>) -> Value {
//     match m1 {
//         My1::Abc => Value::String("abc".into()),
//         My1::Def(i) => todo!(),
//         My1::Xyz { a, b } => {
//             let mut v = Vec::<(nativeshell_core::Value, nativeshell_core::Value)>::new();
//             v.push(("a".into(), a.into()));
//             v.push(("b".into(), b.into()));
//             nativeshell_core::Value::Map(v.into())
//         }
//         My1::FFF(f) => todo!(),
//     }
// }
// fn my(m1: My1) -> Value {
//     match m1 {
//         My1::Abc => todo!(),
//         My1::Def(v1, v2) => todo!(),
//         My1::Xyz { a, b } => todo!(),
//         // My1::Abc => Value::String("abc".into()),
//         // My1::Def(i) => todo!(),
//         // My1::Xyz { a, b } => {
//         //     let mut v = Vec::<(nativeshell_core::Value, nativeshell_core::Value)>::new();
//         //     v.push(("a".into(), a.into()));
//         //     v.push(("b".into(), b.into()));
//         //     nativeshell_core::Value::Map(v.into())
//         // }
//         // My1::FFF(f) => todo!(),
//     }
// }
