use crate::shell::ContextRef;

use super::error::PlatformResult;

pub fn init_platform(_context: &ContextRef) -> PlatformResult<()> {
    Ok(())
}
