pub(crate) mod channel {
    // Flutter channel for windows bound messages; All messages that concern windows are
    // dispatched on this channel
    pub const DISPATCHER: &str = "nativeshell/window-dispatcher";

    // Window sub channels (delivered by dispatcher)
    pub mod win {
        pub const WINDOW_MANAGER: &str = ".window.window-manager";
        pub const DROP_TARGET: &str = ".window.drop-target";
        pub const DRAG_SOURCE: &str = ".window.drag-source";
    }

    // Flutter channel for managing platform menus
    pub const MENU_MANAGER: &str = "nativeshell/menu-manager";

    // Flutter channel for keyboard layout notifications
    pub const KEYBOARD_MAP_MANAGER: &str = "nativeshell/keyboard-map-manager";

    // Flutter channel for managing hot keys
    pub const HOT_KEY_MANAGER: &str = "nativeshell/hot-key-manager";
}

pub const CURRENT_API_VERSION: i32 = 1;

pub(crate) mod method {

    pub mod window_manager {
        pub const GET_API_VERSION: &str = "WindowManager.getApiVersion";

        // Request creation of new window
        pub const CREATE_WINDOW: &str = "WindowManager.createWindow";

        // Initializes current isolate window
        pub const INIT_WINDOW: &str = "WindowManager.initWindow";
    }

    pub mod window {

        // Request to show the window (may be delayed until window itself calls readyToShow)
        pub const SHOW: &str = "Window.show";

        // Request to show the window modally (will return result after window closes)
        pub const SHOW_MODAL: &str = "Window.showModal";

        // Called by window itself after the layout is ready and window is prepared to be shown
        pub const READY_TO_SHOW: &str = "Window.readyToShow";

        // Hide the window
        pub const HIDE: &str = "Window.hide";

        // Bring window front and request focus
        pub const ACTIVATE: &str = "Window.activate";

        // Close the window; This will terminate the isolate
        pub const CLOSE: &str = "Window.close";

        pub const CLOSE_WITH_RESULT: &str = "Window.closeWithResult";

        // All positions, unless otherwise noted are in logical coordinates with top left origin

        pub const SET_GEOMETRY: &str = "Window.setGeometry";
        pub const GET_GEOMETRY: &str = "Window.getGeometry";
        pub const SUPPORTED_GEOMETRY: &str = "Window.supportedGeometry";

        pub const SET_STYLE: &str = "Window.setStyle";
        pub const SET_TITLE: &str = "Window.setTitle";
        pub const PERFORM_WINDOW_DRAG: &str = "Window.performWindowDrag";

        pub const SHOW_POPUP_MENU: &str = "Window.showPopupMenu";
        pub const HIDE_POPUP_MENU: &str = "Window.hidePopupMenu";

        // Windows only
        pub const SHOW_SYSTEM_MENU: &str = "Window.showSystemMenu";

        // MacOS only - associates given menu with current windon; The menu will
        // be displayed  when window gets active
        pub const SET_WINDOW_MENU: &str = "Window.setWindowMenu";

        pub const SAVE_POSITION_TO_STRING: &str = "Window.savePositionToString";
        pub const RESTORE_POSITION_FROM_STRING: &str = "Window.restorePositionFromString";
    }

    pub mod drag_driver {
        pub const DRAGGING_UPDATED: &str = "DragDriver.draggingUpdated";
        pub const DRAGGING_EXITED: &str = "DragDriver.draggingExited";
        pub const PERFORM_DROP: &str = "DragDriver.performDrop";
    }

    pub mod drag_source {
        pub const BEGIN_DRAG_SESSION: &str = "DragSource.beginDragSession";
        pub const DRAG_SESSION_ENDED: &str = "DragSource.dragSessionEnded";
    }

    pub mod menu {
        pub const CREATE_OR_UPDATE: &str = "Menu.createOrUpdate";
        pub const DESTROY: &str = "Menu.destroy";
        pub const ON_ACTION: &str = "Menu.onAction";
        pub const ON_OPEN: &str = "Menu.onOpen";
        pub const SET_APP_MENU: &str = "Menu.setAppMenu";
    }

    pub mod menu_bar {
        // Menubar - move to previous menu
        pub const MOVE_TO_PREVIOUS_MENU: &str = "Menubar.moveToPreviousMenu";
        pub const MOVE_TO_NEXT_MENU: &str = "Menubar.moveToNextMenu";
    }

    pub mod keyboard_map {
        pub const GET: &str = "KeyboardMap.get";
        pub const ON_CHANGED: &str = "KeyboardMap.onChanged";
    }

    pub mod hot_key {
        pub const CREATE: &str = "HotKey.create";
        pub const DESTROY: &str = "HotKey.destroy";
        pub const ON_PRESSED: &str = "HotKey.onPressed";
    }
}

pub(crate) mod event {
    pub mod window {
        // Called when window has been properly initialized and can receive messages
        pub const INITIALIZE: &str = "event:Window.initialize";

        // Called when window became visible or hidden (boolean argument)
        pub const VISIBILITY_CHANGED: &str = "event:Window.visibilityChanged";

        // Delivered when user requested closing the window; Target window is responsible
        // for actually closing the window
        pub const CLOSE_REQUEST: &str = "event:Window.closeRequest";

        // Delivered when window is actually closed
        pub const CLOSE: &str = "event:Window.close";
    }
}

pub(crate) mod drag_data {
    pub mod key {
        pub const FILES: &str = "drag-data:internal:files";
        pub const URLS: &str = "drag-data:internal:urls";
    }
}
