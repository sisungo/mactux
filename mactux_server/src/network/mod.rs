//! Networking.

mod abs;

use abs::AbstractNamespace;

/// A network namespace.
#[derive(Debug)]
pub struct NetNamespace {
    pub abs: AbstractNamespace,
}
