pub mod error;
mod netlist_external;
mod netlist_internal;

pub use netlist_external::{NetExternal, NetListExternal, NetResultExternal, Swap};
pub use netlist_internal::{NetInternal, NetListInternal, NetResultInternal};
