//! Versioned classification, evidence recording, and protected path enforcement.

pub mod catalogue;
pub mod class;
#[cfg(test)]
pub mod destructive_safety;
pub mod evidence;
pub mod matcher;
pub mod plan;
pub mod protected;
#[cfg(test)]
pub mod tests;
