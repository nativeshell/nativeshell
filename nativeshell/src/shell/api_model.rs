use std::collections::HashMap;

use crate::codec::Value;

use super::{status_item_manager::StatusItemHandle, HotKeyHandle, MenuHandle, Point, Rect, Size};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum GeometryPreference {
    PreferFrame,
    PreferContent,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct WindowGeometry {
    pub frame_origin: Option<Point>,
    pub frame_size: Option<Size>,
    pub content_origin: Option<Point>,
    pub content_size: Option<Size>,

    pub min_frame_size: Option<Size>,
    pub max_frame_size: Option<Size>,
    pub min_content_size: Option<Size>,
    pub max_content_size: Option<Size>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WindowGeometryRequest {
    pub geometry: WindowGeometry,
    pub preference: GeometryPreference,
}

impl WindowGeometryRequest {
    // Returns geometry with redundand fields removed (useful when caller
    // supports all fields)
    pub fn filtered_by_preference(self) -> WindowGeometry {
        let mut geometry = self.geometry;

        match self.preference {
            GeometryPreference::PreferFrame => {
                if geometry.frame_origin.is_some() {
                    geometry.content_origin = None;
                }
                if geometry.frame_size.is_some() {
                    geometry.content_size = None;
                }
                if geometry.min_frame_size.is_some() {
                    geometry.min_content_size = None;
                }
                if geometry.max_frame_size.is_some() {
                    geometry.max_content_size = None;
                }
            }
            GeometryPreference::PreferContent => {
                if geometry.content_origin.is_some() {
                    geometry.frame_origin = None;
                }
                if geometry.content_size.is_some() {
                    geometry.frame_size = None;
                }
                if geometry.min_content_size.is_some() {
                    geometry.min_frame_size = None;
                }
                if geometry.max_content_size.is_some() {
                    geometry.max_frame_size = None;
                }
            }
        }

        geometry
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WindowActivateRequest {
    pub activate_application: bool,
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WindowDeactivateRequest {
    pub deactivate_application: bool,
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PopupMenuRequest {
    pub handle: MenuHandle,
    pub position: Point,

    // Windows only, used for menu bar implementation; is specified this
    // rect will keep receiving mouse events
    pub tracking_rect: Option<Rect>,

    // Windows only, menu will not obscure the specified rect
    pub item_rect: Option<Rect>,

    // Windows only, first item will be pre-selected; Use during keyboard navigation
    // in menubar
    pub preselect_first: bool,
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PopupMenuResponse {
    pub item_selected: bool,
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HidePopupMenuRequest {
    pub handle: MenuHandle,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct WindowGeometryFlags {
    pub frame_origin: bool,
    pub frame_size: bool,
    pub content_origin: bool,
    pub content_size: bool,
    pub min_frame_size: bool,
    pub max_frame_size: bool,
    pub min_content_size: bool,
    pub max_content_size: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct DragData {
    pub properties: HashMap<String, Value>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Copy, Clone, PartialEq)]
pub enum DragEffect {
    None,
    Copy,
    Link,
    Move,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DraggingInfo {
    pub location: Point,
    pub data: DragData,
    pub allowed_effects: Vec<DragEffect>,
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DragResult {
    pub effect: DragEffect,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImageData {
    pub width: i32,
    pub height: i32,
    pub bytes_per_row: i32,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DragRequest {
    pub image: ImageData,
    pub rect: Rect,
    pub allowed_effects: Vec<DragEffect>,
    pub data: DragData,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum WindowFrame {
    Regular,
    NoTitle,
    NoFrame,
}

impl Default for WindowFrame {
    fn default() -> Self {
        WindowFrame::Regular
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct WindowStyle {
    pub frame: WindowFrame,
    pub can_resize: bool,
    pub can_close: bool,
    pub can_minimize: bool,
    pub can_maximize: bool,
    pub can_full_screen: bool,
    pub always_on_top: bool,
    pub always_on_top_level: Option<i64>,
    pub traffic_light_offset: Option<Point>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum BoolTransition {
    No,
    NoToYes,
    Yes,
    YesToNo,
}

impl Default for BoolTransition {
    fn default() -> Self {
        BoolTransition::No
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WindowStateFlags {
    pub maximized: BoolTransition,
    pub minimized: BoolTransition,
    pub full_screen: BoolTransition,
    pub active: bool,
}

impl WindowStateFlags {
    pub fn is_minimized(&self) -> bool {
        self.minimized == BoolTransition::Yes || self.minimized == BoolTransition::NoToYes
    }
    pub fn is_maximized(&self) -> bool {
        self.maximized == BoolTransition::Yes || self.maximized == BoolTransition::NoToYes
    }
    pub fn is_full_screen(&self) -> bool {
        self.full_screen == BoolTransition::Yes || self.full_screen == BoolTransition::NoToYes
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct WindowCollectionBehavior {
    pub can_join_all_spaces: bool,
    pub move_to_active_space: bool,
    pub managed: bool,
    pub transient: bool,
    pub stationary: bool,
    pub participates_in_cycle: bool,
    pub ignores_cycle: bool,
    pub full_screen_primary: bool,
    pub full_screen_auxiliary: bool,
    pub full_screen_none: bool,
    pub allows_tiling: bool,
    pub disallows_tiling: bool,
}

//
// Menu
//

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum MenuItemRole {
    About,
    Hide,
    HideOtherApplications,
    ShowAll,
    QuitApplication,
    MinimizeWindow,
    ZoomWindow,
    BringAllToFront,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum MenuRole {
    Window,
    Services,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Accelerator {
    pub label: String,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub control: bool,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum CheckStatus {
    None,
    CheckOn,
    CheckOff,
    RadioOn,
    RadioOff,
}

impl Default for CheckStatus {
    fn default() -> Self {
        CheckStatus::None
    }
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MenuItem {
    pub id: i64,
    pub title: String,
    pub enabled: bool,
    pub separator: bool,
    pub check_status: CheckStatus,
    pub role: Option<MenuItemRole>,
    pub submenu: Option<MenuHandle>,
    pub accelerator: Option<Accelerator>,
}

impl PartialEq for MenuItem {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(serde::Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Menu {
    pub role: Option<MenuRole>,
    pub items: Vec<MenuItem>,
}
#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MenuCreateRequest {
    pub handle: Option<MenuHandle>,
    pub menu: Menu,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuDestroyRequest {
    pub handle: MenuHandle,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuAction {
    pub handle: MenuHandle,
    pub id: i64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuOpen {
    pub handle: MenuHandle,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SetMenuRequest {
    pub handle: Option<MenuHandle>,
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Key {
    pub platform: i64,
    pub physical: i64,
    pub logical: Option<i64>,
    pub logical_shift: Option<i64>,
    pub logical_alt: Option<i64>,
    pub logical_alt_shift: Option<i64>,
    pub logical_meta: Option<i64>,
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardMap {
    pub keys: Vec<Key>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HotKeyCreateRequest {
    pub accelerator: Accelerator,
    pub platform_key: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotKeyDestroyRequest {
    pub handle: HotKeyHandle,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotKeyPressed {
    pub handle: HotKeyHandle,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Screen {
    pub id: i64,
    pub frame: Rect,
    pub work_area: Rect,
    pub scaling_factor: f64,
}

//
// StatusItem
//
#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StatusItemCreateRequest {}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusItemDestroyRequest {
    pub handle: StatusItemHandle,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StatusItemSetImageRequest {
    pub handle: StatusItemHandle,
    pub image: Vec<ImageData>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StatusItemSetHintRequest {
    pub handle: StatusItemHandle,
    pub hint: String,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StatusItemSetHighlightedRequest {
    pub handle: StatusItemHandle,
    pub highlighted: bool,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StatusItemShowMenuRequest {
    pub handle: StatusItemHandle,
    pub menu: MenuHandle,
    pub offset: Point,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StatusItemGetGeometryRequest {
    pub handle: StatusItemHandle,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StatusItemGetScreenIdRequest {
    pub handle: StatusItemHandle,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StatusItemActionType {
    LeftMouseDown,
    LeftMouseUp,
    RightMouseDown,
    RightMouseUp,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusItemAction {
    pub handle: StatusItemHandle,
    pub action: StatusItemActionType,
    pub position: Point,
}
