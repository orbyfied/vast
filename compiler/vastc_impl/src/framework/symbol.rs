use crate::framework::registry::Named;

/// Canonical compile-time symbol name, including the package, declaring type and name. 
/// This construct is used across all generic symbols, including types, functions, fields, etc.
/// It serves as the registry key for all loaded generic symbol types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolName {
  
}

/// Common super-trait for all compile-time symbol entity types.
pub trait Symbol : Named<SymbolName> {
  
}