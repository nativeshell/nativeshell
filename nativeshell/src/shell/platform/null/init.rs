use std::rc::Rc;

use crate::shell::Context;

use super::error::PlatformResult;

pub fn init_platform(_context: Rc<Context>) -> PlatformResult<()> {
    Ok(())
}
