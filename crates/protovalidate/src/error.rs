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

use buffa::Message as _;

use crate::validate::__buffa::oneof::field_path_element::Subscript;
use crate::validate::{FieldPath, Violation as ViolationPb, Violations};

/// A descriptor that could not be registered.
#[derive(Debug)]
pub struct DescriptorError {
    message: String,
}

impl DescriptorError {
    pub(crate) fn new(message: String) -> Self {
        Self { message }
    }
}

impl fmt::Display for DescriptorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for DescriptorError {}

/// A failed validation.
///
/// [`Validation`](Self::Validation) means the message broke its rules and
/// carries the violations, while every other variant means validation itself
/// did not run to completion. `E` is the runtime's own error, which a read of
/// the message can fail with; see [`Runtime::Error`](crate::protobuf::Runtime::Error).
#[derive(Debug)]
#[non_exhaustive]
pub enum Error<E> {
    /// The message broke one or more of its rules.
    Validation(ValidationError),
    /// The runtime could not read the message, or resolve one of its
    /// message types.
    Read(E),
    /// Validation rules could not be compiled.
    Compilation(String),
    /// A rule failed while being evaluated.
    Evaluation(String),
    /// Bad input: an unparsable payload or an unknown type.
    Argument(String),
    /// A failure that fits no other category.
    Unexpected(String),
}

impl<E: fmt::Display> fmt::Display for Error<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(error) => error.fmt(f),
            Self::Read(error) => write!(f, "could not read message: {error}"),
            Self::Compilation(message) => write!(f, "compilation error: {message}"),
            Self::Evaluation(message) => write!(f, "evaluation error: {message}"),
            Self::Argument(message) => write!(f, "invalid argument: {message}"),
            Self::Unexpected(message) => write!(f, "unexpected error: {message}"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for Error<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read(error) => Some(error),
            _ => None,
        }
    }
}

/// One or more rule violations, carried by [`Error::Validation`].
pub struct ValidationError {
    /// Never empty.
    violations: Vec<ViolationPb>,
}

impl ValidationError {
    pub(crate) fn new(violations: Vec<ViolationPb>) -> Self {
        Self { violations }
    }

    /// Encodes the violations as a `buf.validate.Violations`.
    #[must_use]
    pub fn encode_violations(&self) -> Vec<u8> {
        Violations {
            violations: self.violations.clone(),
            ..Default::default()
        }
        .encode_to_vec()
    }
}

impl std::error::Error for ValidationError {}

impl fmt::Debug for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ValidationError")
            .field("violations_len", &self.violations.len())
            .finish()
    }
}

/// The first violation, followed by the number of others.
/// For example, `user.email: must be a valid email address [string.email], and 2 more violations`.
impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_violation(f, &self.violations[0])?;
        match self.violations.len() - 1 {
            0 => Ok(()),
            1 => f.write_str(", and 1 more violation"),
            more => write!(f, ", and {more} more violations"),
        }
    }
}

/// Writes `violation` as `<field path>: <message> [<rule id>]`, omitting the
/// parts that are empty.
fn write_violation(f: &mut fmt::Formatter<'_>, violation: &ViolationPb) -> fmt::Result {
    let message = violation.message.as_deref().unwrap_or_default();
    let rule_id = violation.rule_id.as_deref().unwrap_or_default();
    let mut separator = "";
    if !violation.field.elements.is_empty() {
        write_field_path(f, &violation.field)?;
        separator = ": ";
    }
    if !message.is_empty() {
        write!(f, "{separator}{message}")?;
        separator = " ";
    }
    if !rule_id.is_empty() {
        write!(f, "{separator}[{rule_id}]")?;
    }
    Ok(())
}

fn write_field_path(f: &mut fmt::Formatter<'_>, path: &FieldPath) -> fmt::Result {
    for (i, element) in path.elements.iter().enumerate() {
        let name = element.field_name.as_deref().unwrap_or_default();
        // Extension names are already bracketed, as in `[pkg.ext]`.
        if i > 0 && !name.starts_with('[') {
            f.write_str(".")?;
        }
        f.write_str(name)?;
        match &element.subscript {
            None => {}
            Some(Subscript::Index(index) | Subscript::UintKey(index)) => write!(f, "[{index}]")?,
            Some(Subscript::IntKey(key)) => write!(f, "[{key}]")?,
            Some(Subscript::BoolKey(key)) => write!(f, "[{key}]")?,
            Some(Subscript::StringKey(key)) => {
                f.write_str("[\"")?;
                for c in key.chars() {
                    match c {
                        '\\' => f.write_str("\\\\")?,
                        '"' => f.write_str("\\\"")?,
                        '\r' => f.write_str("\\r")?,
                        '\n' => f.write_str("\\n")?,
                        c => f.write_char(c)?,
                    }
                }
                f.write_str("\"]")?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use buffa::MessageField;

    use super::{FieldPath, Subscript, ValidationError, ViolationPb};
    use crate::validate::FieldPathElement;

    fn element(name: &str, subscript: Option<Subscript>) -> FieldPathElement {
        FieldPathElement {
            field_name: Some(name.to_owned()),
            subscript,
            ..Default::default()
        }
    }

    fn violation(field: Vec<FieldPathElement>, message: &str, rule_id: &str) -> ViolationPb {
        ViolationPb {
            field: if field.is_empty() {
                MessageField::none()
            } else {
                MessageField::some(FieldPath {
                    elements: field,
                    ..Default::default()
                })
            },
            message: Some(message.to_owned()),
            rule_id: Some(rule_id.to_owned()),
            ..Default::default()
        }
    }

    fn display(violations: Vec<ViolationPb>) -> String {
        ValidationError::new(violations).to_string()
    }

    #[test]
    fn displays_field_path() {
        let field = vec![
            element("a", None),
            element("b", Some(Subscript::Index(0))),
            element("[pkg.ext]", None),
            element("c", Some(Subscript::BoolKey(true))),
            element("d", Some(Subscript::IntKey(-1))),
            element("e", Some(Subscript::UintKey(2))),
            element("f", Some(Subscript::StringKey("x\\\"\r\n".to_owned()))),
        ];
        assert_eq!(
            display(vec![violation(field, "must be set", "required")]),
            r#"a.b[0][pkg.ext].c[true].d[-1].e[2].f["x\\\"\r\n"]: must be set [required]"#,
        );
    }

    #[test]
    fn omits_empty_parts() {
        assert_eq!(
            display(vec![violation(vec![], "must be set", "required")]),
            "must be set [required]",
        );
        assert_eq!(
            display(vec![violation(vec![element("a", None)], "", "custom")]),
            "a: [custom]",
        );
        assert_eq!(display(vec![violation(vec![], "", "custom")]), "[custom]");
        assert_eq!(
            display(vec![violation(vec![element("a", None)], "bad", "")]),
            "a: bad",
        );
    }

    #[test]
    fn counts_other_violations() {
        let first = || violation(vec![element("a", None)], "bad", "rule");
        assert_eq!(
            display(vec![first(), violation(vec![], "", "other")]),
            "a: bad [rule], and 1 more violation",
        );
        assert_eq!(
            display(vec![
                first(),
                violation(vec![], "", "other"),
                violation(vec![], "", "other"),
            ]),
            "a: bad [rule], and 2 more violations",
        );
    }
}
