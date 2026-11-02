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

//! The description of a oneof that the validator passes to a runtime.

use std::borrow::Cow;
use std::fmt;

use super::{Field, Message, Runtime};

/// A oneof of the message being validated. The validator passes it to
/// [`Message::which_oneof`] to ask which of its members is set.
pub struct Oneof<R: Runtime> {
    name: Cow<'static, str>,
    index: u32,
    members: Vec<Field<R>>,
}

impl<R: Runtime> Clone for Oneof<R> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            index: self.index,
            members: self.members.clone(),
        }
    }
}

impl<R: Runtime> fmt::Debug for Oneof<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Oneof")
            .field("name", &self.name)
            .field("index", &self.index)
            .field("members", &self.members)
            .finish()
    }
}

impl<R: Runtime> Oneof<R> {
    pub(crate) fn new(name: String, index: u32, members: Vec<Field<R>>) -> Self {
        Self {
            name: Cow::Owned(name),
            index,
            members,
        }
    }

    /// The oneof's name in the `.proto` source.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The oneof's index in its message's `DescriptorProto.oneof_decl`,
    /// which counts synthetic oneofs.
    #[must_use]
    pub fn index(&self) -> u32 {
        self.index
    }

    /// The oneof's fields, in declaration order.
    #[must_use]
    pub fn members(&self) -> &[Field<R>] {
        &self.members
    }
}

/// Returns the field number of the member of `oneof` that is set, by
/// calling [`Message::has`] on each member in turn. This is the default
/// implementation of [`Message::which_oneof`], for a runtime that does not
/// track which member is set.
///
/// # Errors
///
/// The runtime could not tell whether a member is set.
pub fn which_oneof_by_has<R: Runtime>(
    message: &(impl Message<R> + ?Sized),
    oneof: &Oneof<R>,
) -> Result<Option<u32>, R::Error> {
    for member in oneof.members() {
        if message.has(member)? {
            return Ok(Some(member.number()));
        }
    }
    Ok(None)
}
