use windows::Win32::{
    Foundation::{BOOL, LPARAM, RECT},
    Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
    },
};

use super::flutter_sys::FlutterDesktopGetDpiForMonitor;
use crate::shell::{IPoint, IRect, Point, Rect};
use std::{
    cell::RefCell,
    cmp::{self, Ordering},
    collections::HashMap,
    intrinsics::transmute,
    mem,
};

#[derive(Clone, Debug)]
pub struct PhysicalDisplay {
    physical: IRect,
    scale: f64,
    handle: HMONITOR,
}

#[derive(Clone, Debug)]
pub struct Display {
    pub physical: IRect,
    pub logical: Rect,
    pub scale: f64,
    pub handle: HMONITOR,
    pub work: Rect,
    pub id: i64,
}

#[derive(Clone, Debug)]
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
            displays: w.state.iter().map(Self::display_for_state).collect(),
        }
    }

    fn display_for_state(d: &DisplayState) -> Display {
        let mut monitor_info_ex = MONITORINFOEXW::default();
        let mut monitor_info: &mut MONITORINFO = unsafe { transmute(&mut monitor_info_ex) };
        monitor_info.cbSize = mem::size_of::<MONITORINFOEXW>() as u32;
        unsafe { GetMonitorInfoW(d.original.handle, monitor_info as *mut _) };
        // Only used as key in map, don't care about null termination
        let name = String::from_utf16_lossy(&monitor_info_ex.szDevice);
        let physical = d.original.physical.clone();
        let logical = d.adjusted_logical.clone();
        let scale = d.original.scale;
        let work = Rect::xywh(
            logical.x + (monitor_info.rcWork.left - physical.x) as f64 / scale,
            logical.y + (monitor_info.rcWork.top - physical.y) as f64 / scale,
            (monitor_info.rcWork.right - monitor_info.rcWork.left) as f64 / scale,
            (monitor_info.rcWork.bottom - monitor_info.rcWork.top) as f64 / scale,
        );
        let id = AUX_STATE.with(|s| {
            let mut s = s.borrow_mut();
            if !s.name_to_id.contains_key(&name) {
                let id = s.next_screen_id;
                s.next_screen_id += 1;
                s.name_to_id.insert(name.clone(), id);
            }
            *s.name_to_id.get(&name).unwrap()
        });
        Display {
            physical,
            logical,
            scale,
            handle: d.original.handle,
            work,
            id,
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

    pub fn get_displays() -> Displays {
        DISPLAYS.with(|displays| {
            let mut displays = displays.borrow_mut();
            if displays.is_none() {
                displays.replace(Displays::new(displays_from_system()));
            }
            displays.as_ref().unwrap().clone()
        })
    }

    pub fn on_displays_changed<F: Fn() + 'static>(f: F) {
        AUX_STATE.with(|s| {
            let mut s = s.borrow_mut();
            s.on_displays_changed.push(Box::new(f));
        });
    }

    pub fn displays_changed() {
        DISPLAYS.with(|displays| displays.borrow_mut().take());
        AUX_STATE.with(|s| {
            let s = s.borrow();
            for c in &s.on_displays_changed {
                c();
            }
        })
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
            handle: hmonitor,
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

thread_local! {
    static DISPLAYS: RefCell<Option<Displays>> = RefCell::new(None);
    static AUX_STATE: RefCell<AuxState> = RefCell::new(AuxState::new());
}

struct AuxState {
    name_to_id: HashMap<String, i64>,
    next_screen_id: i64,
    on_displays_changed: Vec<Box<dyn Fn()>>,
}

impl AuxState {
    fn new() -> AuxState {
        AuxState {
            name_to_id: HashMap::new(),
            next_screen_id: 1, // reserve 0 for invalid screen Id
            on_displays_changed: Vec::new(),
        }
    }
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
    use windows::Win32::Graphics::Gdi::HMONITOR;

    use crate::shell::{IPoint, IRect, Point, Rect};

    use super::{Displays, PhysicalDisplay};

    #[test]
    fn test1() {
        let d = vec![
            PhysicalDisplay {
                physical: IRect::xywh(0, 0, 1920, 1080),
                scale: 2.0,
                handle: HMONITOR(0),
            },
            PhysicalDisplay {
                physical: IRect::xywh(1920, 0, 1920, 1080),
                scale: 1.0,
                handle: HMONITOR(0),
            },
            PhysicalDisplay {
                physical: IRect::xywh(-1920, 0, 1920, 1080),
                scale: 2.0,
                handle: HMONITOR(0),
            },
            PhysicalDisplay {
                physical: IRect::xywh(-(1920 + 1024), 0, 1024, 1024),
                scale: 1.0,
                handle: HMONITOR(0),
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
                handle: HMONITOR(0),
            },
            PhysicalDisplay {
                physical: IRect::xywh(0, 0, 1920, 1080),
                scale: 1.25,
                handle: HMONITOR(0),
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
