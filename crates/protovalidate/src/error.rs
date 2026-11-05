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

use std::convert::Infallible;
use std::fmt;

use buffa::Message as _;

use crate::Violation;
use crate::validate::{Violation as ViolationPb, Violations};

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
/// [`Validation`](Self::Validation) means that the message failed
/// validation, and carries the violations. Every other variant means that
/// validation could not complete. `E` is the error that the runtime returns
/// when it cannot read the message. See
/// [`Runtime::Error`](crate::protobuf::Runtime::Error).
#[derive(Debug)]
#[non_exhaustive]
pub enum Error<E = Infallible> {
    /// The message broke one or more of its rules.
    Validation(ValidationError),
    /// The runtime could not read the message.
    Read(E),
    /// A descriptor could not be registered.
    Descriptor(DescriptorError),
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
            Self::Descriptor(error) => error.fmt(f),
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
            Self::Validation(error) => Some(error),
            Self::Read(error) => Some(error),
            Self::Descriptor(error) => Some(error),
            _ => None,
        }
    }
}

impl<E> From<DescriptorError> for Error<E> {
    fn from(error: DescriptorError) -> Self {
        Self::Descriptor(error)
    }
}

/// One or more rule violations, carried by [`Error::Validation`].
#[derive(Debug)]
pub struct ValidationError {
    /// Never empty.
    violations: Vec<Violation>,
}

impl ValidationError {
    pub(crate) fn new(violations: Vec<ViolationPb>) -> Self {
        Self {
            violations: violations.into_iter().map(Violation::new).collect(),
        }
    }

    /// The violations, in the order they were found.
    #[must_use]
    pub fn violations(&self) -> &[Violation] {
        &self.violations
    }

    /// Encodes the violations as a `buf.validate.Violations`.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        Violations {
            violations: self
                .violations
                .iter()
                .map(|violation| violation.proto().clone())
                .collect(),
            ..Default::default()
        }
        .encode_to_vec()
    }
}

impl std::error::Error for ValidationError {}

/// The first violation, followed by the number of others, as in
/// `user.email: must be a valid email address, and 2 more violations`.
///
/// The alternate form (`{:#}`) lists every violation on its own line
/// instead:
///
/// ```text
/// validation failed:
///  - user.email: must be a valid email address
///  - user.age: must be less than 150
/// ```
impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.write_str("validation failed:")?;
            for violation in &self.violations {
                write!(f, "\n - {violation}")?;
            }
            return Ok(());
        }
        let mut violations = self.violations.iter();
        if let Some(first) = violations.next() {
            first.fmt(f)?;
        }
        match violations.len() {
            0 => Ok(()),
            1 => f.write_str(", and 1 more violation"),
            more => write!(f, ", and {more} more violations"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ValidationError, ViolationPb};

    fn violation(message: &str) -> ViolationPb {
        ViolationPb {
            message: Some(message.to_owned()),
            rule_id: Some("rule".to_owned()),
            ..Default::default()
        }
    }

    #[test]
    fn counts_other_violations() {
        let display = |messages: &[&str]| {
            ValidationError::new(messages.iter().map(|message| violation(message)).collect())
                .to_string()
        };
        assert_eq!(display(&["bad"]), "bad");
        assert_eq!(display(&["bad", "worse"]), "bad, and 1 more violation");
        assert_eq!(
            display(&["bad", "worse", "worst"]),
            "bad, and 2 more violations"
        );
    }

    #[test]
    fn alternate_form_lists_every_violation() {
        let error = ValidationError::new(vec![violation("bad"), violation("worse")]);
        assert_eq!(format!("{error:#}"), "validation failed:\n - bad\n - worse");
        // An `Error` wrapping the validation error formats it the same way.
        assert_eq!(
            format!(
                "{:#}",
                super::Error::<std::convert::Infallible>::Validation(error)
            ),
            "validation failed:\n - bad\n - worse"
        );
    }
}
