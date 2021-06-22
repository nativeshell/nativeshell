use std::ops::{Add, Div, Mul, Sub};

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct _Rect<T> {
    pub x: T,
    pub y: T,
    pub width: T,
    pub height: T,
}

impl<T> _Rect<T>
where
    T: Add<Output = T>
        + Div<Output = T>
        + Sub<Output = T>
        + Mul<Output = T>
        + Copy
        + PartialOrd
        + From<i32>
        + Into<f64>
        + _CastNumber<f64>,
{
    pub fn xywh(x: T, y: T, width: T, height: T) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn origin_size(origin: &_Point<T>, size: &_Size<T>) -> Self {
        Self {
            x: origin.x,
            y: origin.y,
            width: size.width,
            height: size.height,
        }
    }

    pub fn x2(&self) -> T {
        self.x + self.width
    }

    pub fn y2(&self) -> T {
        self.y + self.height
    }

    pub fn is_inside(&self, point: &_Point<T>) -> bool {
        point.x >= self.x && point.x < self.x2() && point.y >= self.y && point.y < self.y2()
    }

    pub fn center(&self) -> _Point<T> {
        _Point::xy(
            self.x + self.width / T::from(2),
            self.y + self.height / T::from(2),
        )
    }

    pub fn origin(&self) -> _Point<T> {
        self.top_left()
    }

    pub fn top_left(&self) -> _Point<T> {
        _Point::xy(self.x, self.y)
    }

    pub fn bottom_right(&self) -> _Point<T> {
        _Point::xy(self.x2(), self.y2())
    }

    pub fn size(&self) -> _Size<T> {
        _Size {
            width: self.width,
            height: self.height,
        }
    }

    pub fn to_local(&self, origin: &_Point<T>) -> _Point<T> {
        _Point::xy(origin.x - self.x, origin.y - self.y)
    }

    pub fn translated(&self, delta: &_Point<T>) -> Self {
        Self {
            x: self.x + delta.x,
            y: self.y + delta.y,
            width: self.width,
            height: self.height,
        }
    }

    pub fn scaled(&self, factor: f64) -> Self {
        let scaled_x: f64 = self.x.into() * factor;
        let scaled_y: f64 = self.y.into() * factor;
        let scaled_width: f64 = self.width.into() * factor;
        let scaled_height: f64 = self.height.into() * factor;
        Self {
            x: T::cast_number(scaled_x),
            y: T::cast_number(scaled_y),
            width: T::cast_number(scaled_width),
            height: T::cast_number(scaled_height),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct _Point<T> {
    pub x: T,
    pub y: T,
}

impl<T> _Point<T>
where
    T: Sub<Output = T> + Mul<Output = T> + Add<Output = T> + Into<f64> + Copy + _CastNumber<f64>,
{
    pub fn xy(x: T, y: T) -> Self {
        Self { x, y }
    }

    pub fn distance(&self, p: &_Point<T>) -> f64 {
        let x = p.x - self.x;
        let y = p.y - self.y;
        let d: f64 = (x * x + y * y).into();
        d.sqrt()
    }

    pub fn translated(&self, delta: &_Point<T>) -> Self {
        Self {
            x: self.x + delta.x,
            y: self.y + delta.y,
        }
    }

    pub fn scaled(&self, factor: f64) -> Self {
        let scaled_x: f64 = self.x.into() * factor;
        let scaled_y: f64 = self.y.into() * factor;
        Self {
            x: T::cast_number(scaled_x),
            y: T::cast_number(scaled_y),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct _Size<T> {
    pub width: T,
    pub height: T,
}

impl<T> _Size<T>
where
    T: Sub<Output = T>
        + Mul<Output = T>
        + Add<Output = T>
        + Into<f64>
        + _CastNumber<f64>
        + Copy
        + Default,
{
    pub fn wh(width: T, height: T) -> Self {
        Self { width, height }
    }

    pub fn scaled(&self, factor: f64) -> Self {
        let scaled_width: f64 = self.width.into() * factor;
        let scaled_height: f64 = self.height.into() * factor;
        Self {
            width: T::cast_number(scaled_width),
            height: T::cast_number(scaled_height),
        }
    }
}

impl<T> std::ops::Sub<_Size<T>> for _Size<T>
where
    T: Sub<Output = T> + Copy,
{
    type Output = _Size<T>;

    fn sub(self, rhs: _Size<T>) -> Self::Output {
        Self::Output {
            width: self.width - rhs.width,
            height: self.height - rhs.height,
        }
    }
}

impl<'a, 'b, T> std::ops::Sub<&'b _Size<T>> for &'a _Size<T>
where
    T: Sub<Output = T> + Copy,
{
    type Output = _Size<T>;

    fn sub(self, rhs: &'b _Size<T>) -> Self::Output {
        Self::Output {
            width: self.width - rhs.width,
            height: self.height - rhs.height,
        }
    }
}

impl<'a, 'b, T> std::ops::Add<&'b _Size<T>> for &'a _Size<T>
where
    T: Add<Output = T> + Copy,
{
    type Output = _Size<T>;

    fn add(self, rhs: &'b _Size<T>) -> Self::Output {
        Self::Output {
            width: self.width + rhs.width,
            height: self.height + rhs.height,
        }
    }
}

impl<T> std::ops::Add<_Size<T>> for _Size<T>
where
    T: Add<Output = T> + Copy,
{
    type Output = _Size<T>;

    fn add(self, rhs: _Size<T>) -> Self::Output {
        Self::Output {
            width: self.width + rhs.width,
            height: self.height + rhs.height,
        }
    }
}

pub trait _CastNumber<T>: Sized {
    fn cast_number(_: T) -> Self;
}

impl _CastNumber<f64> for i32 {
    fn cast_number(i: f64) -> Self {
        i as i32
    }
}

impl _CastNumber<f64> for f64 {
    fn cast_number(i: f64) -> Self {
        i
    }
}

impl _CastNumber<i32> for i32 {
    fn cast_number(i: i32) -> Self {
        i
    }
}

impl _CastNumber<i32> for f64 {
    fn cast_number(i: i32) -> Self {
        i as f64
    }
}

impl From<IPoint> for Point {
    fn from(point: IPoint) -> Self {
        Self::xy(point.x as f64, point.y as f64)
    }
}

// account of rounding errors that happen with fraction scaling
fn round_epsilon(v: f64) -> f64 {
    (v + 0.000001).floor()
}

impl From<Point> for IPoint {
    fn from(point: Point) -> Self {
        Self::xy(round_epsilon(point.x) as i32, round_epsilon(point.y) as i32)
    }
}

impl From<ISize> for Size {
    fn from(size: ISize) -> Self {
        Self::wh(size.width as f64, size.height as f64)
    }
}

impl From<Size> for ISize {
    fn from(size: Size) -> Self {
        Self::wh(
            round_epsilon(size.width) as i32,
            round_epsilon(size.height) as i32,
        )
    }
}

impl From<IRect> for Rect {
    fn from(rect: IRect) -> Self {
        Rect::xywh(
            rect.x as f64,
            rect.y as f64,
            rect.width as f64,
            rect.height as f64,
        )
    }
}

impl From<Rect> for IRect {
    fn from(rect: Rect) -> Self {
        IRect::xywh(
            round_epsilon(rect.x) as i32,
            round_epsilon(rect.y) as i32,
            round_epsilon(rect.width) as i32,
            round_epsilon(rect.height) as i32,
        )
    }
}

pub type Rect = _Rect<f64>;
pub type IRect = _Rect<i32>;
pub type Point = _Point<f64>;
pub type IPoint = _Point<i32>;
pub type Size = _Size<f64>;
pub type ISize = _Size<i32>;
