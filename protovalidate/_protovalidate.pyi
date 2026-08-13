# Copyright (c) 2023-2026 Buf Technologies, Inc.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
"""The native protovalidate engine."""

from typing import Any, final

from google.protobuf.message import Message
from protobuf import Message as Message2, Registry

from protovalidate._gen.buf.validate.validate_pb import (
    FieldPath,
    Violation as Violation2,
)

@final
class Validator:
    """Validate Protobuf messages against static rules.

    Both protobuf-py messages and legacy google.protobuf messages are
    accepted; either is handed to the native engine in serialized form.

    Each validator instance caches internal state generated from the static
    rules, so reusing the same instance for multiple validations
    significantly improves performance.
    """
    def __new__(cls, /, registry: Registry | None = None) -> Validator:
        """Create a new validator.

        Parameters:
            registry: An optional Registry used to resolve custom
                predefined-rule extensions. If omitted, only standard rules are applied.
        """
    def collect_violations(
        self, /, message: Message2 | Message, *, fail_fast: bool = False
    ) -> list[Violation]:
        """Validates the given message against the static rules defined in the message's descriptor.

        Compared to `validate`, `collect_violations` simply returns the violations as a list and puts
        the burden of raising an appropriate exception on the caller.

        The violations returned from this method should always be equal to the violations
        raised as part of the ValidationError in the call to `validate`.

        Parameters:
            message: The message to validate.
            fail_fast: If true, validation will stop after the first iteration.

        Returns:
            A list of Violation objects that describe the validation errors.

        Raises:
            CompilationError: If the static rules could not be compiled.
            EvaluationError: If a rule failed while being evaluated.
        """
    def validate(
        self, /, message: Message2 | Message, *, fail_fast: bool = False
    ) -> None:
        """Validate the given message against the static rules defined in the message's descriptor.

        Parameters:
            message: The message to validate.
            fail_fast: If true, validation will stop after the first iteration.

        Raises:
            CompilationError: If the static rules could not be compiled.
            EvaluationError: If a rule failed while being evaluated.
            ValidationError: If the message is invalid. The violations raised as part of this error should
                always be equal to the list of violations returned by `collect_violations`.
        """

@final
class Violation:
    """A singular rule violation."""
    def __new__(
        cls,
        /,
        *,
        field_value: Any | None = None,
        rule_value: Any | None = None,
        field: FieldPath | None = None,
        rule: FieldPath | None = None,
        rule_id: str | None = None,
        message: str | None = None,
        for_key: bool = False,
    ) -> Violation:
        """Create a new violation."""
    @property
    def field_value(self, /) -> Any:
        """The value of the field that violated the rule."""
    @property
    def proto(self, /) -> Violation2:
        """The `buf.validate.Violation` form of this violation."""
    @property
    def rule_value(self, /) -> Any:
        """The value of the rule that was violated."""
