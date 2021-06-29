use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use gdk::{Event, EventType, Gravity, Rectangle};

use gtk::{
    prelude::{ContainerExt, FixedExt, GtkMenuExt, GtkMenuItemExt, MenuShellExt, WidgetExt},
    Fixed, Menu, MenuBar, MenuDirectionType, MenuItem,
};

use crate::shell::{
    api_model::{PopupMenuRequest, PopupMenuResponse},
    IRect, Point, Rect,
};

use super::{
    error::PlatformResult,
    menu::PlatformMenu,
    utils::{synthetize_button_up, synthetize_leave_event_from_motion},
    window::PlatformWindow,
};

pub(super) struct WindowMenu {
    window: Weak<PlatformWindow>,
    current_menu: RefCell<Option<Rc<PlatformMenu>>>,
    current_request: RefCell<Option<PopupMenuRequest>>,
    menu_item: MenuItem,
    pub(super) menu_bar_container: Fixed,
    menu_bar: MenuBar,
    last_motion_event: RefCell<Option<Event>>,
    pointer_inside_item_rect: Cell<bool>,
}

impl WindowMenu {
    pub fn new(window: Weak<PlatformWindow>) -> Self {
        let res = Self {
            window,
            current_menu: RefCell::new(None),
            current_request: RefCell::new(None),
            menu_item: MenuItem::new(),
            menu_bar: MenuBar::new(),
            menu_bar_container: Fixed::new(),
            last_motion_event: RefCell::new(None),
            pointer_inside_item_rect: Cell::new(false),
        };
        res.init();
        res
    }

    fn init(&self) {
        self.menu_item.add(&Fixed::new()); // just want some empty contents

        self.menu_bar_container.put(&self.menu_bar, 0, 0);
        self.menu_bar.add(&self.menu_item);

        let weak_win = self.window.clone();
        self.menu_bar.connect_selection_done(move |_| {
            if let Some(window) = weak_win.upgrade() {
                window.window_menu.borrow().on_menu_bar_selection_done();
            }
        });
        let weak_win = self.window.clone();
        self.menu_bar.connect_move_current(move |_, dir| {
            if let Some(menu) = weak_win
                .upgrade()
                .and_then(|w| w.window_menu.borrow().current_menu.borrow().clone())
            {
                if dir == MenuDirectionType::Prev {
                    menu.move_to_previous_menu();
                } else if dir == MenuDirectionType::Next {
                    menu.move_to_next_menu();
                }
            }
        });
    }

    pub fn should_forward_event(&self, event: &Event) -> bool {
        let window = self.window.upgrade().unwrap();

        // Store last motion event, this is necessary to get proper hover after
        // closing the menu; Note that this works even without
        if event.event_type() == EventType::MotionNotify {
            self.last_motion_event.borrow_mut().replace(event.clone());
        } else if event.event_type() == EventType::LeaveNotify {
            self.last_motion_event.borrow_mut().take();
        }

        if let Some(rect) = self
            .current_request
            .borrow()
            .as_ref()
            .and_then(|r| r.tracking_rect.as_ref())
        {
            if event.event_type() == EventType::MotionNotify {
                let event_coords = event.root_coords();
                if let Some(event_coords) = event_coords {
                    let window_pos = window.view.borrow().window().unwrap().origin();
                    let point = Point::xy(
                        event_coords.0 - window_pos.1 as f64,
                        event_coords.1 - window_pos.2 as f64,
                    );
                    if rect.is_inside(&point) {
                        self.pointer_inside_item_rect.replace(true);
                        return true;
                    } else if self.pointer_inside_item_rect.replace(false) {
                        let mut leave = synthetize_leave_event_from_motion(event);
                        window.propagate_event(&mut leave);
                    }
                }
            }
            if event.event_type() == EventType::LeaveNotify {
                return true;
            }
        }
        false
    }

    pub fn show_popup_menu<F>(&self, menu: Rc<PlatformMenu>, request: PopupMenuRequest, on_done: F)
    where
        F: FnOnce(PlatformResult<PopupMenuResponse>) + 'static,
    {
        let current_menu = self.current_menu.borrow().clone();
        if let Some(current_menu) = current_menu {
            current_menu.menu.cancel();
        }

        self.current_menu.borrow_mut().replace(menu.clone());
        self.current_request.borrow_mut().replace(request.clone());
        self.pointer_inside_item_rect.replace(false);

        let window = self.window.upgrade().unwrap();
        let events = window.last_event.borrow();
        let last_button_event = events
            .values()
            .filter(|e| {
                e.event_type() == EventType::ButtonPress
                    || e.event_type() == EventType::ButtonRelease
            })
            .max_by(|e1, e2| e1.time().cmp(&e2.time()))
            .and_then(|e| {
                if e.event_type() == EventType::ButtonPress {
                    Some(e)
                } else {
                    None
                }
            });

        if let Some(event) = last_button_event {
            // menu was shown
            let mut release = synthetize_button_up(event);
            gtk::main_do_event(&mut release);
        }

        // event to make Gtk happy
        let trigger_event = events
            .get(&EventType::ButtonPress)
            .or_else(|| events.get(&EventType::KeyPress));

        // Request has item rect, this is possibly a menu bar
        if let Some(item_rect) = request.item_rect.as_ref() {
            // for menu bar, we need to be able to track events in main window
            // while menu is active; For that we ensure that there is a menu bar
            // in hierarchy, which will cause Gtk to put grab on this window as well.
            //
            // Note that when opening popup menus with mouse button the grab is put on
            // the window as well, but for later menus opened either through keyboard
            // or during hover, having menu_bar in hierarchy is necessary;

            // this activates the menu bar
            self.menu_bar.show();
            self.menu_item.show();
            self.menu_bar.select_item(&self.menu_item);

            // this establishes link to menubar
            self.menu_item.set_submenu(Some(&menu.menu));

            self.position_menu_bar(item_rect);

            // finally
            menu.menu.popup_at_widget(
                &self.menu_item,
                Gravity::SouthWest,
                Gravity::NorthWest,
                trigger_event,
            );

            // enabled during keyboard navigation; this also takes care of enabling
            // mnemonics
            if request.preselect_first {
                for item in menu.menu.children() {
                    if item.is_sensitive() {
                        item.mnemonic_activate(true);
                        break;
                    }
                }
            }
        } else {
            menu.menu.popup_at_rect(
                &window.view.borrow().window().unwrap(),
                &Rectangle {
                    x: request.position.x as i32,
                    y: request.position.y as i32,
                    width: 0,
                    height: 0,
                },
                Gravity::SouthWest,
                Gravity::NorthWest,
                trigger_event,
            );
        }

        // close menu notification

        let win_weak = self.window.clone();

        let menu_clone = menu.menu.clone();
        menu.on_selection_done(move |selected| {
            let window = win_weak.upgrade();
            if let Some(window) = window {
                window
                    .window_menu
                    .borrow()
                    .on_menu_selection_done(&menu_clone);
            }

            on_done(Ok(PopupMenuResponse {
                item_selected: selected,
            }));
        });
    }

    fn position_menu_bar(&self, item_rect: &Rect) {
        let item_rect: IRect = item_rect.clone().into();

        self.menu_item
            .set_size_request(item_rect.width, item_rect.height);

        self.menu_item.preferred_width();
        self.menu_item.preferred_height();

        // call this otherwise size_allocate complains
        self.menu_bar.preferred_width();
        self.menu_bar.preferred_height();

        self.menu_bar_container
            .move_(&self.menu_bar, item_rect.x, item_rect.y);

        // actually allocate the size
        self.menu_bar_container.check_resize();
    }

    fn on_menu_bar_selection_done(&self) {
        let current_menu = self.current_menu.borrow().clone();
        if let Some(current_menu) = current_menu {
            current_menu.trigger_selection_done();
        }
    }

    fn on_menu_selection_done(&self, menu: &Menu) {
        menu.toplevel().unwrap().unrealize();

        let current_menu = {
            let current = self.current_menu.borrow();
            current.as_ref().map(|m| m.menu.clone())
        };

        if current_menu.as_ref() == Some(menu) {
            self.current_menu.borrow_mut().take();
            self.current_request.borrow_mut().take();
            self.menu_item.set_submenu::<gtk::Menu>(None);

            self.menu_bar.hide();
            self.menu_bar.deactivate();

            if let Some(mut event) = self.last_motion_event.borrow_mut().take() {
                gtk::main_do_event(&mut event);
            }
        }
    }

    pub fn hide_popup_menu(&self, menu: Rc<PlatformMenu>) -> PlatformResult<()> {
        menu.menu.cancel();
        Ok(())
    }
}
