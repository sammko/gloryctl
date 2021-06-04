mod device;
pub mod protocol; // TODO this is only pub for debugging

pub use device::{buttonmap::ButtonAction, macros, rgb, Color, Config, DataReport, GloriousDevice};
