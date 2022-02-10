// Opaque handle for keeping a resource alive while handle exists
pub struct Handle {
    on_cancel: Option<Box<dyn FnOnce()>>,
}

impl Handle {
    pub fn new<F>(on_cancel: F) -> Self
    where
        F: FnOnce() + 'static,
    {
        Self {
            on_cancel: Some(Box::new(on_cancel)),
        }
    }

    pub fn cancel(&mut self) {
        if let Some(on_cancel) = self.on_cancel.take() {
            on_cancel();
        }
    }

    pub fn detach(&mut self) {
        self.on_cancel.take();
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.cancel();
    }
}
