use std::convert::{TryFrom, TryInto};

use nativeshell_core::{TryFromError, Value};
use nativeshell_derive::{IntoValue, TryFromValue};

#[derive(PartialEq, Debug, Clone)]
struct Unserializable {}

#[derive(PartialEq, IntoValue, TryFromValue, Debug, Clone)]
enum Enum1DefaultTag<
    T: std::fmt::Debug + PartialEq + Into<Value> + TryFrom<Value, Error = E>,
    E: Into<TryFromError> + PartialEq,
> {
    Unit1,
    #[nativeshell(rename = "yyy")]
    Unit2,
    SingleValue(String),
    DoubleValue(String, String),
    SingleValueT(T),
    DoubleValueT(T, T),
    #[nativeshell(skip)]
    _Unserializable(Unserializable),
    #[nativeshell(rename = "_Xyz")]
    Xyz {
        #[nativeshell(rename = "xabc")]
        a: String,
        #[nativeshell(skip)]
        b: i64,
        c: Value,
        d: T,
    },
}

#[test]
fn test_enum_1() -> Result<(), TryFromError> {
    {
        let v1: Enum1DefaultTag<i64, _> = Enum1DefaultTag::Unit1;
        let sv1: Value = v1.clone().into();
        assert_eq!(sv1, "Unit1".into());
        let v1d: Enum1DefaultTag<i64, _> = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1: Enum1DefaultTag<i64, _> = Enum1DefaultTag::Unit2;
        let sv1: Value = v1.clone().into();
        assert_eq!(sv1, "yyy".into());
        let v1d: Enum1DefaultTag<i64, _> = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1: Enum1DefaultTag<i64, _> = Enum1DefaultTag::SingleValue("Hello".into());
        let sv1: Value = v1.clone().into();
        assert_eq!(
            sv1,
            Value::Map(vec![("SingleValue".into(), "Hello".into())].into())
        );
        let v1d: Enum1DefaultTag<i64, _> = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1: Enum1DefaultTag<i64, _> =
            Enum1DefaultTag::DoubleValue("Hello".into(), "Hello2".into());
        let sv1: Value = v1.clone().into();
        assert_eq!(
            sv1,
            Value::Map(
                vec![(
                    "DoubleValue".into(),
                    Value::List(vec!["Hello".into(), "Hello2".into()].into())
                )]
                .into()
            )
        );
        let v1d: Enum1DefaultTag<i64, _> = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1: Enum1DefaultTag<String, _> = Enum1DefaultTag::SingleValueT("String".into());
        let sv1: Value = v1.clone().into();
        assert_eq!(
            sv1,
            Value::Map(vec![("SingleValueT".into(), "String".into())].into())
        );
        let v1d: Enum1DefaultTag<String, _> = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1: Enum1DefaultTag<Value, _> = Enum1DefaultTag::SingleValueT(Value::I64(10));
        let sv1: Value = v1.clone().into();
        assert_eq!(
            sv1,
            Value::Map(vec![("SingleValueT".into(), Value::I64(10))].into())
        );
        let v1d: Enum1DefaultTag<Value, _> = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1: Enum1DefaultTag<String, _> =
            Enum1DefaultTag::DoubleValueT("Hello".into(), "Hello2".into());
        let sv1: Value = v1.clone().into();
        assert_eq!(
            sv1,
            Value::Map(
                vec![(
                    "DoubleValueT".into(),
                    Value::List(vec!["Hello".into(), "Hello2".into()].into())
                )]
                .into()
            )
        );
        let v1d: Enum1DefaultTag<String, _> = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1: Enum1DefaultTag<Value, _> =
            Enum1DefaultTag::DoubleValueT(10i64.into(), 11i64.into());
        let sv1: Value = v1.clone().into();
        assert_eq!(
            sv1,
            Value::Map(
                vec![(
                    "DoubleValueT".into(),
                    Value::List(vec![Value::I64(10), Value::I64(11)])
                )]
                .into()
            )
        );
        let v1d: Enum1DefaultTag<Value, _> = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let mut v1 = Enum1DefaultTag::Xyz {
            a: "String".into(),
            b: 10.into(),
            c: Value::F64(10.5),
            d: 15i64,
        };
        let sv1: Value = v1.clone().into();
        assert_eq!(
            sv1,
            Value::Map(
                vec![(
                    "_Xyz".into(),
                    Value::Map(
                        vec![
                            ("xabc".into(), "String".into()),
                            ("d".into(), 15i64.into()),
                            ("c".into(), Value::F64(10.5)),
                        ]
                        .into()
                    )
                )]
                .into()
            )
        );
        let v1d: Enum1DefaultTag<i64, _> = sv1.try_into()?;
        assert_ne!(v1, v1d);
        match &mut v1 {
            Enum1DefaultTag::Xyz {
                a: _,
                b,
                c: _,
                d: _,
            } => {
                *b = 0;
            }
            _ => {}
        }
        assert_eq!(v1, v1d);
    }
    Ok(())
}

#[derive(PartialEq, IntoValue, TryFromValue, Debug, Clone)]
#[nativeshell(tag = "t")]
enum Enum2CustomTag {
    Abc,
    #[nativeshell(rename = "_Def")]
    Def,
    #[nativeshell(rename_all = "UPPERCASE")]
    Xyz {
        x: i64,
        s: String,
    },
}

#[test]
fn test_enum_2() -> Result<(), TryFromError> {
    {
        let v1 = Enum2CustomTag::Abc;
        let sv1: Value = v1.clone().into();
        assert_eq!(sv1, Value::Map(vec![("t".into(), "Abc".into())].into()));
        let v1d: Enum2CustomTag = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1 = Enum2CustomTag::Def;
        let sv1: Value = v1.clone().into();
        assert_eq!(sv1, Value::Map(vec![("t".into(), "_Def".into())].into()));
        let v1d: Enum2CustomTag = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1 = Enum2CustomTag::Xyz {
            x: 15,
            s: "Hello".into(),
        };
        let sv1: Value = v1.clone().into();
        assert_eq!(
            sv1,
            Value::Map(
                vec![
                    ("t".into(), "Xyz".into()),
                    ("X".into(), 15.into()),
                    ("S".into(), "Hello".into())
                ]
                .into()
            )
        );
        let v1d: Enum2CustomTag = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    Ok(())
}

#[derive(PartialEq, IntoValue, TryFromValue, Debug, Clone)]
#[nativeshell(tag = "t", content = "c")]
#[nativeshell(rename_all = "UPPERCASE")]
enum Enum3CustomTagContent {
    Abc,
    #[nativeshell(rename = "_Def")]
    Def,
    SingleValue(i64),
    #[nativeshell(rename = "_DoubleValue")]
    DoubleValue(f64, f64),
    Xyz {
        x: i64,
        s: String,
        z1: Option<i64>,
        #[nativeshell(skip_if_null)]
        z2: Option<i64>,
        z3: Option<f64>,
    },
}

#[test]
fn test_enum_3() -> Result<(), TryFromError> {
    {
        let v1 = Enum3CustomTagContent::Abc;
        let sv1: Value = v1.clone().into();
        assert_eq!(sv1, Value::Map(vec![("t".into(), "ABC".into())].into()));
        let v1d: Enum3CustomTagContent = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1 = Enum3CustomTagContent::Def;
        let sv1: Value = v1.clone().into();
        assert_eq!(sv1, Value::Map(vec![("t".into(), "_Def".into())].into()));
        let v1d: Enum3CustomTagContent = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1 = Enum3CustomTagContent::SingleValue(10);
        let sv1: Value = v1.clone().into();
        assert_eq!(
            sv1,
            Value::Map(
                vec![
                    ("t".into(), "SINGLEVALUE".into()), //
                    ("c".into(), 10i64.into()),
                ]
                .into()
            )
        );
        let v1d: Enum3CustomTagContent = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1 = Enum3CustomTagContent::DoubleValue(10.5, 11.5);
        let sv1: Value = v1.clone().into();
        assert_eq!(
            sv1,
            Value::Map(
                vec![
                    ("t".into(), "_DoubleValue".into()), //
                    (
                        "c".into(),
                        Value::List(vec![10.5f64.into(), 11.5f64.into()])
                    ),
                ]
                .into()
            )
        );
        let v1d: Enum3CustomTagContent = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1 = Enum3CustomTagContent::Xyz {
            x: 15,
            s: "hello".into(),
            z1: None,
            z2: None,
            z3: Some(10.5f64),
        };
        let sv1: Value = v1.clone().into();
        assert_eq!(
            sv1,
            Value::Map(
                vec![
                    ("t".into(), "XYZ".into()), //
                    (
                        "c".into(),
                        Value::Map(
                            vec![
                                ("x".into(), 15.into()), //
                                ("s".into(), "hello".into()),
                                ("z1".into(), Value::Null),
                                ("z3".into(), 10.5.into())
                            ]
                            .into()
                        )
                    ),
                ]
                .into()
            )
        );
        let v1d: Enum3CustomTagContent = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    Ok(())
}

#[derive(Clone, PartialEq, Debug, IntoValue, TryFromValue)]
struct NewType1(i64);
#[derive(Clone, PartialEq, Debug, IntoValue, TryFromValue)]
struct NewType2(Value);
#[derive(Clone, PartialEq, Debug, IntoValue, TryFromValue)]
struct NewType3(Option<i64>);

#[derive(Clone, PartialEq, Debug, IntoValue, TryFromValue)]
struct NewType4(NewType1);
#[derive(Clone, PartialEq, Debug, IntoValue, TryFromValue)]
struct NewTypeInStruct {
    v: NewType1,
}

#[derive(Clone, PartialEq, Debug, IntoValue, TryFromValue)]
struct NewTypeStruct {
    x: i64,
    y: String,
}
#[derive(Clone, PartialEq, Debug, IntoValue, TryFromValue)]
struct NewTypeWithStruct(NewTypeStruct);

#[derive(Clone, PartialEq, Debug, IntoValue, TryFromValue)]
struct NewTypeGeneric<
    T: std::fmt::Debug + PartialEq + Into<Value> + TryFrom<Value, Error = E>,
    E: Into<TryFromError> + PartialEq,
>(T);

#[test]
fn test_new_type() -> Result<(), TryFromError> {
    {
        let v1 = NewType1(10);
        let sv1: Value = v1.clone().into();
        assert_eq!(sv1, Value::I64(10));
        let v1d: NewType1 = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1 = NewType2(Value::String("Hello".into()));
        let sv1: Value = v1.clone().into();
        assert_eq!(sv1, Value::String("Hello".into()));
        let v1d: NewType2 = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1 = NewType3(Some(10.into()));
        let sv1: Value = v1.clone().into();
        assert_eq!(sv1, Value::I64(10));
        let v1d: NewType3 = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1 = NewType3(None);
        let sv1: Value = v1.clone().into();
        assert_eq!(sv1, Value::Null);
        let v1d: NewType3 = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1 = NewType4(NewType1(15));
        let sv1: Value = v1.clone().into();
        assert_eq!(sv1, Value::I64(15));
        let v1d: NewType4 = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1 = NewTypeInStruct { v: NewType1(15) };
        let sv1: Value = v1.clone().into();
        assert_eq!(sv1, Value::Map(vec![("v".into(), 15.into())].into()));
        let v1d: NewTypeInStruct = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1 = NewTypeWithStruct(NewTypeStruct {
            x: 10,
            y: "Hello".into(),
        });
        let sv1: Value = v1.clone().into();
        assert_eq!(
            sv1,
            Value::Map(
                vec![
                    ("x".into(), 10.into()), //
                    ("y".into(), "Hello".into())
                ]
                .into()
            )
        );
        let v1d: NewTypeWithStruct = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1 = NewTypeGeneric(10i64);
        let sv1: Value = v1.clone().into();
        assert_eq!(sv1, 10i64.into());
        let v1d: NewTypeGeneric<i64, _> = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    Ok(())
}

#[derive(Clone, PartialEq, Debug, IntoValue, TryFromValue)]
struct Tuple1(i64, Option<String>, Value);

#[derive(Clone, PartialEq, Debug, IntoValue, TryFromValue)]
struct TupleGeneric<
    T: std::fmt::Debug + PartialEq + Into<Value> + TryFrom<Value, Error = E>,
    E: Into<TryFromError> + PartialEq,
>(i64, Option<T>, Value, T);

#[test]
fn test_tuple() -> Result<(), TryFromError> {
    {
        let v1 = Tuple1(10, None, Value::Bool(false));
        let sv1: Value = v1.clone().into();
        assert_eq!(
            sv1,
            Value::List(vec![10i64.into(), Value::Null, false.into()])
        );
        let v1d: Tuple1 = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1 = Tuple1(10, Some("Hello".into()), Value::Bool(false));
        let sv1: Value = v1.clone().into();
        assert_eq!(
            sv1,
            Value::List(vec![10i64.into(), "Hello".into(), false.into()])
        );
        let v1d: Tuple1 = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1 = TupleGeneric(10, Some("Hello".into()), Value::Bool(false), "S2".into());
        let sv1: Value = v1.clone().into();
        assert_eq!(
            sv1,
            Value::List(vec![
                10i64.into(),
                "Hello".into(),
                false.into(),
                "S2".into()
            ])
        );
        let v1d: TupleGeneric<String, _> = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1 = TupleGeneric(10, None, Value::Bool(false), "S2".into());
        let sv1: Value = v1.clone().into();
        assert_eq!(
            sv1,
            Value::List(vec![10i64.into(), Value::Null, false.into(), "S2".into()])
        );
        let v1d: TupleGeneric<String, _> = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    Ok(())
}

#[derive(Clone, PartialEq, Debug, IntoValue, TryFromValue)]
enum EnumInStruct1 {
    Value,
    Value2,
}

#[derive(Clone, PartialEq, Debug, IntoValue, TryFromValue)]
struct Struct1 {
    s1: String,
    i: Option<i64>,
    #[nativeshell(rename = "abc")]
    v: Value,
    v2: Option<Value>,
    e: EnumInStruct1,
    e2: Option<EnumInStruct1>,
    #[nativeshell(skip_if_null)]
    e3: Option<EnumInStruct1>,
    e4: Option<EnumInStruct1>,
}

#[derive(Clone, PartialEq, Debug, IntoValue, TryFromValue)]
#[nativeshell(rename_all = "UPPERCASE")]
struct StructGeneric<
    T: std::fmt::Debug + PartialEq + Into<Value> + TryFrom<Value, Error = E>,
    E: Into<TryFromError> + PartialEq,
> {
    t: T,
    t2: Option<T>,
    #[nativeshell(skip_if_null)]
    t3: Option<T>,
    #[nativeshell(rename = "_T4")]
    t4: Option<T>,
}

#[test]
fn test_struct() -> Result<(), TryFromError> {
    {
        let v1 = Struct1 {
            s1: "Hello".into(),
            i: Some(5),
            v: Value::I64(10),
            v2: None,
            e: EnumInStruct1::Value,
            e2: Some(EnumInStruct1::Value2),
            e3: None,
            e4: None,
        };
        let sv1: Value = v1.clone().into();
        assert_eq!(
            sv1,
            Value::Map(
                vec![
                    ("s1".into(), "Hello".into()), //
                    ("i".into(), 5.into()),
                    ("abc".into(), 10.into()),
                    ("v2".into(), Value::Null),
                    ("e".into(), "Value".into()),
                    ("e2".into(), "Value2".into()),
                    ("e4".into(), Value::Null),
                ]
                .into()
            ),
        );
        let v1d: Struct1 = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    {
        let v1: StructGeneric<String, _> = StructGeneric {
            t: "Hello".into(),
            t2: Some("Hello2".into()),
            t3: None,
            t4: None,
        };
        let sv1: Value = v1.clone().into();
        assert_eq!(
            sv1,
            Value::Map(
                vec![
                    ("T".into(), "Hello".into()), //
                    ("T2".into(), "Hello2".into()),
                    ("_T4".into(), Value::Null),
                ]
                .into()
            ),
        );
        let v1d: StructGeneric<String, _> = sv1.try_into()?;
        assert_eq!(v1d, v1);
    }
    Ok(())
}
