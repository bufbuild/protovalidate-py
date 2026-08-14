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

from __future__ import annotations

from typing import TYPE_CHECKING

from protovalidate._gen.buf.validate import validate_pb

if TYPE_CHECKING:
    from protovalidate import Violation

__all__ = ["CompilationError", "EvaluationError", "ValidationError"]


class CompilationError(Exception):
    """An error raised when a rule fails to compile."""

    __slots__ = ()


class EvaluationError(RuntimeError):
    """An error raised when a rule fails while being evaluated."""

    __slots__ = ()


class ValidationError(ValueError):
    """An error raised when a message fails to validate.

    Attributes:
        violations: A list of Violation objects that describe the validation errors.
    """

    __slots__ = ("_violations",)

    _violations: list[Violation]

    def __init__(self, msg: str, violations: list[Violation]) -> None:
        super().__init__(msg)
        self._violations = violations

    def to_proto(self) -> validate_pb.Violations:
        """Provides the Protobuf form of the validation errors.

        Returns:
            The validation errors as a `validate_pb.Violations` Protobuf message.
        """
        return validate_pb.Violations(
            violations=[violation.proto for violation in self._violations]
        )

    @property
    def violations(self) -> list[Violation]:
        return self._violations
