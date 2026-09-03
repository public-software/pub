#![doc = include_str!("../SPEC.md")]
#![forbid(unsafe_code)]

/// One conformance case: an implementation given `input` must produce `expected`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Case {
    /// The requirement the case holds an implementation to, e.g. `{{COMPONENT}}-1`.
    pub requirement: &'static str,
    /// A name unique among the cases.
    pub name: &'static str,
    /// The input, in the encoding the specification's Conformance section fixes.
    pub input: &'static str,
    /// The expected output, in the same encoding.
    pub expected: &'static str,
}

/// Every conformance case of the specification, in requirement order.
pub const CASES: &[Case] = &[Case {
    requirement: "{{COMPONENT}}-1",
    name: "identity",
    input: "",
    expected: "",
}];

/// The cases as an iterator, for implementations that test against them.
pub fn cases() -> impl Iterator<Item = &'static Case> {
    CASES.iter()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn there_is_at_least_one_case() {
        assert!(cases().next().is_some());
    }

    #[test]
    fn case_names_are_unique() {
        let names: BTreeSet<&str> = cases().map(|c| c.name).collect();
        assert_eq!(names.len(), CASES.len());
    }

    #[test]
    fn every_case_names_a_requirement_of_this_specification() {
        for case in cases() {
            assert!(case.requirement.starts_with("{{COMPONENT}}-"), "{}", case.requirement);
        }
    }
}
