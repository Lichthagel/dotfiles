pub mod assets;
pub mod error;
pub mod mappings;
pub mod model;
pub mod platform;
pub mod registry;

use std::ffi::OsString;
use std::io::Write;

use error::Result;

pub fn run<I, T>(_args: I, _io: &mut dyn Write) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    Ok(())
}
