pub mod transactions;
pub mod client;

/// General type declarations, so when we want to change them, we can just change them in one spot,
/// instead of having to deal with changing it everywhere.
///
/// This breaks a bit of the containment generally found in models, but in my opinion makes the
/// code much more maintainable

/// The type of client ids
pub type ClientID = u16;

/// The type of transaction ids
pub type TransactionID = u32;

/// The type for the amounts transacted in the system
/// Use regular longs as floats have precision misshapes even
/// with 64 bits which can lead to non precise accounts.
/// Instead, we multiply the float by the precision we want and then
/// use the long version in every
pub type MoneyType = u64;

/// No value type for the type state builders,
/// indicates that the corresponding field has not yet been filled
#[derive(Default)]
pub struct NoVal {}