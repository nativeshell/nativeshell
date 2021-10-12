use super::{all_bindings::*, flutter_sys::FlutterDesktopGetDpiForMonitor};
use crate::shell::{IPoint, IRect, Point, Rect};
use lazy_static::lazy_static;
use std::{
    cell::{Ref, RefCell},
    cmp::{self, Ordering},
};

#[derive(Clone, Debug)]
pub struct PhysicalDisplay {
    physical: IRect,
    scale: f64,
    handle: isize,
}

#[derive(Clone, Debug)]
pub struct Display {
    pub physical: IRect,
    pub logical: Rect,
    pub scale: f64,
    pub handle: isize,
}

pub struct Displays {
    pub displays: Vec<Display>,
}

fn equals(x1: f64, x2: f64) -> bool {
    (x1 - x2).abs() < f64::EPSILON
}

// Takes series of displays with physical bounds and calculates logical bounds for them
impl Displays {
    pub fn new(displays: Vec<PhysicalDisplay>) -> Self {
        let mut w = Work::new(&displays);
        w.perform();
        Self {
            displays: w
                .state
                .iter()
                .map(|d| Display {
                    physical: d.original.physical.clone(),
                    logical: d.adjusted_logical.clone(),
                    scale: d.original.scale,
                    handle: d.original.handle,
                })
                .collect(),
        }
    }

    pub fn display_for_physical_point(&self, point: &IPoint) -> Option<&Display> {
        self.displays
            .iter()
            .find(|d| d.physical.is_inside(point))
            .or_else(|| {
                self.displays.iter().min_by(|a, b| {
                    a.physical
                        .center()
                        .distance(point)
                        .partial_cmp(&b.physical.center().distance(point))
                        .unwrap()
                })
            })
    }

    pub fn display_for_logical_point(&self, point: &Point) -> Option<&Display> {
        self.displays
            .iter()
            .find(|d| d.logical.is_inside(point))
            .or_else(|| {
                self.displays.iter().min_by(|a, b| {
                    a.logical
                        .center()
                        .distance(point)
                        .partial_cmp(&b.logical.center().distance(point))
                        .unwrap()
                })
            })
    }

    pub fn convert_physical_to_logical(&self, point: &IPoint) -> Option<Point> {
        let display = self.display_for_physical_point(point);
        match display {
            Some(display) => {
                let local = display.physical.to_local(point);
                Some(Point::xy(
                    local.x as f64 / display.scale + display.logical.x,
                    local.y as f64 / display.scale + display.logical.y,
                ))
            }
            None => None,
        }
    }

    pub fn convert_logical_to_physical(&self, point: &Point) -> Option<IPoint> {
        let display = self.display_for_logical_point(point);
        match display {
            Some(display) => {
                let local = display.logical.to_local(point);
                Some(IPoint::xy(
                    (local.x * display.scale) as i32 + display.physical.x,
                    (local.y * display.scale) as i32 + display.physical.y,
                ))
            }
            None => None,
        }
    }

    pub fn get_displays() -> Ref<'static, Displays> {
        if GLOBAL.displays.borrow().is_none() {
            GLOBAL
                .displays
                .borrow_mut()
                .replace(Displays::new(displays_from_system()));
        }
        Ref::map(GLOBAL.displays.borrow(), |d| d.as_ref().unwrap())
    }

    pub fn displays_changed() {
        GLOBAL.displays.borrow_mut().take();
    }
}

extern "system" fn enum_monitors(
    hmonitor: HMONITOR,
    _hdc: HDC,
    rect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    unsafe {
        let displays = &mut *(lparam.0 as *mut Vec<PhysicalDisplay>);
        let rect = &*(rect);
        displays.push(PhysicalDisplay {
            physical: IRect::xywh(
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
            ),
            scale: FlutterDesktopGetDpiForMonitor(hmonitor.0) as f64 / 96.0,
            handle: hmonitor.0,
        });
    }
    true.into()
}

fn displays_from_system() -> Vec<PhysicalDisplay> {
    unsafe {
        let displays = Vec::<PhysicalDisplay>::new();
        EnumDisplayMonitors(
            HDC(0),
            std::ptr::null_mut(),
            Some(enum_monitors),
            LPARAM(&displays as *const _ as isize),
        );
        displays
    }
}

struct Global {
    displays: RefCell<Option<Displays>>,
}

unsafe impl Sync for Global {}

lazy_static! {
    static ref GLOBAL: Global = Global {
        displays: RefCell::new(None),
    };
}

struct DisplayState {
    original: PhysicalDisplay,
    adjusted_physical: IRect,
    adjusted_logical: Rect,
}

struct Work {
    state: Vec<DisplayState>,
}

impl Work {
    fn new(displays: &[PhysicalDisplay]) -> Self {
        return Work {
            state: displays
                .iter()
                .map(|d| DisplayState {
                    original: d.clone(),
                    adjusted_physical: Default::default(),
                    adjusted_logical: Default::default(),
                })
                .collect(),
        };
    }

    // move physical displays so that minimum is at 0 0
    fn adjust(&mut self) {
        let mut min = (std::i32::MAX, std::i32::MAX);
        for d in &mut self.state {
            min.0 = cmp::min(min.0, d.original.physical.x);
            min.1 = cmp::min(min.1, d.original.physical.y);
        }
        for d in &mut self.state {
            d.adjusted_physical = IRect {
                x: d.original.physical.x - min.0,
                y: d.original.physical.y - min.1,
                width: d.original.physical.width,
                height: d.original.physical.height,
            }
        }
    }

    // sort physical displays
    fn sort(&mut self) {
        self.state.sort_by(|a, b| {
            let res = a.adjusted_physical.x.cmp(&b.adjusted_physical.x);
            match res {
                Ordering::Equal => a.adjusted_physical.y.cmp(&b.adjusted_physical.y),
                _ => res,
            }
        });
    }

    fn compute_initial_logical(&mut self) {
        for d in &mut self.state {
            d.adjusted_logical = Rect {
                x: d.adjusted_physical.x as f64,
                y: d.adjusted_physical.y as f64,
                width: d.adjusted_physical.width as f64 / d.original.scale,
                height: d.adjusted_physical.height as f64 / d.original.scale,
            }
        }
    }

    // remove gaps from adjusted_logical rects
    fn squeeze(&mut self) {
        'outer: loop {
            for i in 0..self.state.len() {
                let d = &self.state[i];

                let min_x = self
                    .state
                    .iter()
                    .filter(|d2| {
                        d2.adjusted_logical.x < d.adjusted_logical.x
                            && d2.adjusted_logical.x2() <= d.adjusted_logical.x2()
                            // vertical intersection
                            && d2.adjusted_logical.y < d.adjusted_logical.y2()
                            && d2.adjusted_logical.y2() > d.adjusted_logical.y
                    })
                    .map(|d2| d2.adjusted_logical.x2())
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(d.adjusted_logical.x);

                let min_y = self
                    .state
                    .iter()
                    .filter(|d2| {
                        d2.adjusted_logical.y < d.adjusted_logical.y
                            && d2.adjusted_logical.y2() <= d.adjusted_logical.y2()
                            // horizontal intersection
                            && d2.adjusted_logical.x < d.adjusted_logical.x2()
                            && d2.adjusted_logical.x2() > d.adjusted_logical.x
                    })
                    .map(|d2| d2.adjusted_logical.y2())
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(d.adjusted_logical.y);
                if !equals(d.adjusted_logical.x, min_x) || !equals(d.adjusted_logical.y, min_y) {
                    let d = &mut self.state[i];
                    d.adjusted_logical.x = min_x;
                    d.adjusted_logical.y = min_y;
                    continue 'outer;
                }
            }
            break;
        }
    }

    // move adjusted logical rects so that main display starts at 0.0, 0.0
    fn adjust_back(&mut self) {
        let mut delta = (0.0, 0.0);
        let main_display = self
            .state
            .iter()
            .find(|d| d.original.physical.x == 0 && d.original.physical.y == 0);
        if let Some(main_display) = main_display {
            delta = (
                main_display.adjusted_logical.x,
                main_display.adjusted_logical.y,
            )
        }
        for d in &mut self.state {
            d.adjusted_logical.x -= delta.0;
            d.adjusted_logical.y -= delta.1;
        }
    }

    fn perform(&mut self) {
        if self.state.is_empty() {
            return;
        }
        let first = self.state.first().unwrap();
        if self
            .state
            .iter()
            .all(|d| equals(d.original.scale, first.original.scale))
        {
            // all screens have same scaling factor, simply scale all rects
            for d in &mut self.state {
                d.adjusted_logical = Rect {
                    x: d.original.physical.x as f64 / d.original.scale,
                    y: d.original.physical.y as f64 / d.original.scale,
                    width: d.original.physical.width as f64 / d.original.scale,
                    height: d.original.physical.height as f64 / d.original.scale,
                };
            }
        } else {
            self.adjust();
            self.sort();
            self.compute_initial_logical();
            self.squeeze();
            self.adjust_back();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::shell::{IPoint, IRect, Point, Rect};

    use super::{Displays, PhysicalDisplay};

    #[test]
    fn test1() {
        let d = vec![
            PhysicalDisplay {
                physical: IRect::xywh(0, 0, 1920, 1080),
                scale: 2.0,
                handle: 0,
            },
            PhysicalDisplay {
                physical: IRect::xywh(1920, 0, 1920, 1080),
                scale: 1.0,
                handle: 0,
            },
            PhysicalDisplay {
                physical: IRect::xywh(-1920, 0, 1920, 1080),
                scale: 2.0,
                handle: 0,
            },
            PhysicalDisplay {
                physical: IRect::xywh(-(1920 + 1024), 0, 1024, 1024),
                scale: 1.0,
                handle: 0,
            },
        ];

        let displays = Displays::new(d);

        assert_eq!(
            displays.convert_physical_to_logical(&IPoint::xy(0, 0)),
            Some(Point::xy(0.0, 0.0))
        );

        assert_eq!(
            displays.convert_logical_to_physical(&Point::xy(0.0, 0.0)),
            Some(IPoint::xy(0, 0))
        );

        assert_eq!(
            displays.convert_physical_to_logical(&IPoint::xy(500, 500)),
            Some(Point::xy(250.0, 250.0))
        );

        assert_eq!(
            displays.convert_logical_to_physical(&Point::xy(250.0, 250.0)),
            Some(IPoint::xy(500, 500))
        );

        assert_eq!(
            displays.convert_physical_to_logical(&IPoint::xy(2000, 500)),
            Some(Point::xy(1040.0, 500.0))
        );

        assert_eq!(
            displays.convert_logical_to_physical(&Point::xy(1040.0, 500.0)),
            Some(IPoint::xy(2000, 500))
        );

        let d = displays.display_for_physical_point(&IPoint::xy(-3500, 500));
        assert_eq!(d.unwrap().physical.x, -2944);

        // for r in &displays.displays {
        //     println!(
        //         "{} {} {} {}",
        //         r.logical.x, r.logical.y, r.logical.width, r.logical.height
        //     );
        // }
    }

    #[test]
    fn test2() {
        let d = vec![
            PhysicalDisplay {
                physical: IRect::xywh(-3840, 14, 3840, 2160),
                scale: 2.0,
                handle: 0,
            },
            PhysicalDisplay {
                physical: IRect::xywh(0, 0, 1920, 1080),
                scale: 1.25,
                handle: 0,
            },
        ];
        let displays = Displays::new(d);
        assert_eq!(
            displays.displays[0].logical,
            Rect::xywh(-1920.0, 14.0, 1920.0, 1080.0)
        );
        assert_eq!(
            displays.displays[1].logical,
            Rect::xywh(0.0, 0.0, 1536.0, 864.0)
        );
        // for r in displays.displays {
        //     println!(
        //         "{} {} {} {}",
        //         r.logical.x, r.logical.y, r.logical.width, r.logical.height
        //     );
        // }
    }
}
