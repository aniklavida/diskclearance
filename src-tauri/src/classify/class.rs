use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Primary safety classification for scanned filesystem findings.
///
/// Safety classification is strictly typed and impossible for the frontend
/// interface to override.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum SafetyClass {
    /// Generated data with a verified owner and declarative rule.
    /// May be recommended for cleanup. Never removed silently.
    Rebuildable,
    /// User files, old downloads, duplicate copies, uncertain leftovers,
    /// and heuristic matches. Never selected by default at any level of the stack.
    Review,
    /// System and user-critical assets that cannot enter a plan at all.
    /// Structurally unable to be represented as a plan item.
    Protected,
}

impl SafetyClass {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Rebuildable => "rebuildable",
            Self::Review => "review",
            Self::Protected => "protected",
        }
    }

    /// Returns true if this safety class may be selected by default.
    ///
    /// Review and Protected items are NEVER selected by default anywhere
    /// in the core's output.
    pub const fn is_default_selected(&self) -> bool {
        match self {
            Self::Rebuildable => true,
            Self::Review => false,
            Self::Protected => false,
        }
    }

    /// Returns true if this finding is protected from any plan inclusion.
    pub const fn is_protected(&self) -> bool {
        matches!(self, Self::Protected)
    }

    /// Converts this safety class to a plannable class, returning a typed rejection
    /// if the item is Protected.
    pub fn to_plannable(&self) -> Result<PlannableClass, ProtectedClassRejection> {
        match self {
            Self::Rebuildable => Ok(PlannableClass::Rebuildable),
            Self::Review => Ok(PlannableClass::Review),
            Self::Protected => Err(ProtectedClassRejection),
        }
    }
}

/// A safety class that is permitted to be included in a review plan.
///
/// Notice that `Protected` is structurally absent from this enum, making it
/// impossible to represent a protected plan item at the type level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum PlannableClass {
    Rebuildable,
    Review,
}

impl PlannableClass {
    pub const fn safety_class(&self) -> SafetyClass {
        match self {
            Self::Rebuildable => SafetyClass::Rebuildable,
            Self::Review => SafetyClass::Review,
        }
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Rebuildable => "rebuildable",
            Self::Review => "review",
        }
    }
}

/// Error returned when attempting to convert a Protected safety class to a PlannableClass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProtectedClassRejection;

impl std::fmt::Display for ProtectedClassRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Protected items are structurally barred from entering a deletion plan"
        )
    }
}

impl std::error::Error for ProtectedClassRejection {}

/// Zero-sized marker type for compile-time enforcement of Rebuildable safety class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RebuildableMarker;

/// Zero-sized marker type for compile-time enforcement of Review safety class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReviewMarker;

/// Zero-sized marker type for compile-time enforcement of Protected safety class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProtectedMarker;

/// Trait implemented only by safety classes that can legally be represented in a plan.
///
/// Crucially, `ProtectedMarker` does NOT implement `PlannableSafetyMarker`, making
/// any compile-time attempt to construct a plan with a Protected item a compiler error.
pub trait PlannableSafetyMarker: Send + Sync + 'static {
    fn plannable_class() -> PlannableClass;
    fn safety_class() -> SafetyClass;
}

impl PlannableSafetyMarker for RebuildableMarker {
    fn plannable_class() -> PlannableClass {
        PlannableClass::Rebuildable
    }

    fn safety_class() -> SafetyClass {
        SafetyClass::Rebuildable
    }
}

impl PlannableSafetyMarker for ReviewMarker {
    fn plannable_class() -> PlannableClass {
        PlannableClass::Review
    }

    fn safety_class() -> SafetyClass {
        SafetyClass::Review
    }
}

/// Returns the subset of IDs that are selected by default.
///
/// Enforces the invariant that Review and Protected items are never selected by default.
pub fn filter_default_selected<T: Clone>(items: &[(T, SafetyClass)]) -> Vec<T> {
    items
        .iter()
        .filter_map(|(id, class)| {
            if class.is_default_selected() {
                Some(id.clone())
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safety_class_default_selection_rules() {
        assert!(SafetyClass::Rebuildable.is_default_selected());
        assert!(!SafetyClass::Review.is_default_selected());
        assert!(!SafetyClass::Protected.is_default_selected());
    }

    #[test]
    fn test_protected_cannot_convert_to_plannable() {
        assert_eq!(
            SafetyClass::Rebuildable.to_plannable(),
            Ok(PlannableClass::Rebuildable)
        );
        assert_eq!(
            SafetyClass::Review.to_plannable(),
            Ok(PlannableClass::Review)
        );
        assert_eq!(
            SafetyClass::Protected.to_plannable(),
            Err(ProtectedClassRejection)
        );
    }

    #[test]
    fn test_filter_default_selected_excludes_review_and_protected() {
        let items = vec![
            ("item-1".to_string(), SafetyClass::Rebuildable),
            ("item-2".to_string(), SafetyClass::Review),
            ("item-3".to_string(), SafetyClass::Protected),
            ("item-4".to_string(), SafetyClass::Review),
            ("item-5".to_string(), SafetyClass::Rebuildable),
        ];

        let selected = filter_default_selected(&items);
        assert_eq!(selected, vec!["item-1".to_string(), "item-5".to_string()]);
    }
}
