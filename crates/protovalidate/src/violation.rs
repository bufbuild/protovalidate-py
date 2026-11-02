// Copyright (c) 2023-2026 Buf Technologies, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fmt::{self, Write as _};
use std::sync::OnceLock;

use crate::pb::buffa::{FieldPath, Subscript, Violation as ViolationPb};

/// A single rule violation, together with its Protobuf form.
#[derive(Clone, Debug)]
pub struct Violation {
    proto: ViolationPb,
    /// Rendered on first use.
    field_path: OnceLock<String>,
    rule_path: OnceLock<String>,
}

impl PartialEq for Violation {
    fn eq(&self, other: &Self) -> bool {
        self.proto == other.proto
    }
}

impl Violation {
    pub(crate) fn new(proto: ViolationPb) -> Self {
        Self {
            field_path: OnceLock::new(),
            rule_path: OnceLock::new(),
            proto,
        }
    }

    /// The id of the rule that failed, such as `string.email`, or the `id` of
    /// a custom CEL rule.
    #[must_use]
    pub fn rule_id(&self) -> &str {
        self.proto.rule_id.as_deref().unwrap_or_default()
    }

    /// The human-readable message describing the failure.
    #[must_use]
    pub fn message(&self) -> &str {
        self.proto.message.as_deref().unwrap_or_default()
    }

    /// Whether the violation is about a map key rather than its value.
    #[must_use]
    pub fn for_key(&self) -> bool {
        self.proto.for_key.unwrap_or_default()
    }

    /// The path to the field that failed, such as `user.emails[0]`.
    ///
    /// It is empty for a message-level rule, which does not apply to a field.
    #[must_use]
    pub fn field_path(&self) -> &str {
        self.field_path
            .get_or_init(|| path_string(self.proto.field.as_option()))
    }

    /// The path to the rule that failed, relative to the rules of the field
    /// that [`field_path`](Self::field_path) names.
    #[must_use]
    pub fn rule_path(&self) -> &str {
        self.rule_path
            .get_or_init(|| path_string(self.proto.rule.as_option()))
    }

    /// The violation as a `buf.validate.Violation`.
    #[must_use]
    pub fn proto(&self) -> &ViolationPb {
        &self.proto
    }
}

/// Formats as `<field path>: <message>`. An empty message is replaced by
/// `[<rule id>]`, and an empty field path is omitted.
impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.field_path().is_empty() {
            write!(f, "{}: ", self.field_path())?;
        }
        match (self.message(), self.rule_id()) {
            ("", "") => f.write_str("[unknown]"),
            ("", rule_id) => write!(f, "[{rule_id}]"),
            (message, _) => f.write_str(message),
        }
    }
}

/// Formats a path as `user.emails[0]`, matching the other protovalidate
/// implementations.
fn path_string(path: Option<&FieldPath>) -> String {
    let mut rendered = String::new();
    let Some(path) = path else {
        return rendered;
    };
    for (i, element) in path.elements.iter().enumerate() {
        let name = element.field_name.as_deref().unwrap_or_default();
        // Extension names are already bracketed, as in `[pkg.ext]`.
        if i > 0 && !name.starts_with('[') {
            rendered.push('.');
        }
        rendered.push_str(name);
        match &element.subscript {
            None => {}
            Some(Subscript::Index(index) | Subscript::UintKey(index)) => {
                let _ = write!(rendered, "[{index}]");
            }
            Some(Subscript::IntKey(key)) => {
                let _ = write!(rendered, "[{key}]");
            }
            Some(Subscript::BoolKey(key)) => {
                let _ = write!(rendered, "[{key}]");
            }
            // The key is quoted so that a key containing `.` or `]` is
            // unambiguous.
            Some(Subscript::StringKey(key)) => {
                rendered.push_str("[\"");
                for c in key.chars() {
                    match c {
                        '\\' => rendered.push_str("\\\\"),
                        '"' => rendered.push_str("\\\""),
                        '\r' => rendered.push_str("\\r"),
                        '\n' => rendered.push_str("\\n"),
                        c => rendered.push(c),
                    }
                }
                rendered.push_str("\"]");
            }
        }
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::{Violation, ViolationPb};
    use crate::pb::buffa::{FieldPath, FieldPathElement, Subscript};

    fn element(name: &str, subscript: Option<Subscript>) -> FieldPathElement {
        FieldPathElement {
            field_name: Some(name.to_owned()),
            subscript,
            ..Default::default()
        }
    }

    fn violation(field: Vec<FieldPathElement>, message: &str, rule_id: &str) -> Violation {
        Violation::new(ViolationPb {
            field: Some(FieldPath {
                elements: field,
                ..Default::default()
            })
            .into(),
            message: Some(message.to_owned()),
            rule_id: Some(rule_id.to_owned()),
            ..Default::default()
        })
    }

    fn rendered(elements: Vec<FieldPathElement>) -> String {
        violation(elements, "", "").field_path().to_owned()
    }

    #[test]
    fn a_message_level_rule_has_no_path() {
        assert_eq!(Violation::new(ViolationPb::default()).field_path(), "");
        assert_eq!(rendered(vec![]), "");
    }

    #[test]
    fn nested_fields_join_with_dots() {
        assert_eq!(rendered(vec![element("user", None)]), "user");
        assert_eq!(
            rendered(vec![element("user", None), element("email", None)]),
            "user.email"
        );
        assert_eq!(
            rendered(vec![element("user", None), element("[pkg.ext]", None)]),
            "user[pkg.ext]"
        );
    }

    #[test]
    fn subscripts_follow_the_field_they_index() {
        assert_eq!(
            rendered(vec![
                element("user", None),
                element("emails", Some(Subscript::Index(2))),
            ]),
            "user.emails[2]"
        );
        assert_eq!(
            rendered(vec![element("scores", Some(Subscript::IntKey(-3)))]),
            "scores[-3]"
        );
        assert_eq!(
            rendered(vec![element("counts", Some(Subscript::UintKey(7)))]),
            "counts[7]"
        );
        assert_eq!(
            rendered(vec![element("flags", Some(Subscript::BoolKey(true)))]),
            "flags[true]"
        );
    }

    #[test]
    fn string_keys_are_quoted() {
        assert_eq!(
            rendered(vec![element(
                "attrs",
                Some(Subscript::StringKey("x\\\"\r\n".to_owned())),
            )]),
            r#"attrs["x\\\"\r\n"]"#
        );
    }

    #[test]
    fn display_prefixes_the_field_and_prefers_the_message() {
        assert_eq!(
            violation(
                vec![element("email", None)],
                "must be an email",
                "string.email"
            )
            .to_string(),
            "email: must be an email"
        );
        assert_eq!(
            violation(vec![], "first_name needs last_name", "").to_string(),
            "first_name needs last_name"
        );
    }

    #[test]
    fn display_falls_back_to_the_rule_id() {
        assert_eq!(
            violation(vec![element("email", None)], "", "string.email").to_string(),
            "email: [string.email]"
        );
        assert_eq!(violation(vec![], "", "").to_string(), "[unknown]");
    }

    #[test]
    fn the_proto_form_is_kept() {
        let violation = violation(vec![], "must be an email", "string.email");
        assert_eq!(violation.rule_id(), "string.email");
        assert_eq!(violation.proto().rule_id.as_deref(), Some("string.email"));
    }
}
