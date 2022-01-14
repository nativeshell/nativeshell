use std::{cell::RefCell, rc::Weak};

use crate::shell::{api_model::Screen, screen_manager::ScreenManagerDelegate, Point};

use super::error::{PlatformError, PlatformResult};

pub struct PlatformScreenManager {}

impl PlatformScreenManager {
    pub fn new(delegate: Weak<RefCell<dyn ScreenManagerDelegate>>) -> Self {
        Self {}
    }

    pub fn get_screens(&self) -> PlatformResult<Vec<Screen>> {
        Err(PlatformError::NotImplemented)
    }

    pub fn get_main_screen(&self) -> PlatformResult<i64> {
        Err(PlatformError::NotImplemented)
    }

    pub fn logical_to_system(&self, offset: Point) -> PlatformResult<Point> {
        Err(PlatformError::NotImplemented)
    }

    pub fn system_to_logical(&self, offset: Point) -> PlatformResult<Point> {
        Err(PlatformError::NotImplemented)
    }
}
