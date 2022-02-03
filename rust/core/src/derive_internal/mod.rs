/// Internal helpers for deriving TryFromValue and IntoValue
/// We need these traits to be specialized for <T> and Option<T>, see
/// https://lukaskalbertodt.github.io/2019/12/05/generalized-autoref-based-specialization.html
/// for details on how this works.
use std::{
    convert::{TryFrom, TryInto},
    result::Result,
};

use crate::{TryFromError, Value};

pub struct Wrap<'a, T>(pub &'a mut T);

pub trait Assign {
    fn assign(&mut self, value: Value) -> Result<(), TryFromError>;
    fn set_optional_to_none(&mut self);
}

impl<'a, T: TryFrom<Value, Error = E>, E> Assign for Wrap<'a, Option<T>>
where
    E: Into<TryFromError>,
{
    fn assign(&mut self, value: Value) -> Result<(), TryFromError> {
        self.0.replace(value.try_into().map_err(|e: E| e.into())?);
        Ok(())
    }
    fn set_optional_to_none(&mut self) {}
}

impl<'a, T: TryFrom<Value, Error = E>, E> Assign for &mut Wrap<'a, Option<Option<T>>>
where
    E: Into<TryFromError>,
{
    fn assign(&mut self, value: Value) -> Result<(), TryFromError> {
        match value {
            Value::Null => self.0.replace(Option::<T>::None),
            v => self.0.replace(Some(v.try_into().map_err(|e: E| e.into())?)),
        };
        Ok(())
    }
    fn set_optional_to_none(&mut self) {
        if self.0.is_none() {
            self.0.replace(None);
        }
    }
}
