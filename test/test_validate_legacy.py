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

import pytest

pytest.importorskip("google.protobuf", reason="optional dependency not installed")


from typing import TYPE_CHECKING

import protovalidate
from protovalidate import Violation

from ._utils import ValidatorProtocol, check_valid, compare_violations
from .conftest import make_validator
from .gen.tests.example.v1 import validations_pb2

if TYPE_CHECKING:
    from google.protobuf import message as google_message

validators: list[ValidatorProtocol] = [
    protovalidate,  # global module singleton
    make_validator(),
]


@pytest.mark.parametrize("validator", validators)
def test_legacy_message_valid(validator: ValidatorProtocol) -> None:
    """A google.protobuf message validates through the legacy conversion path."""
    msg = validations_pb2.DoubleFinite()
    msg.val = 1.0

    check_valid(validator, msg)


@pytest.mark.parametrize("validator", validators)
def test_legacy_message_invalid(validator: ValidatorProtocol) -> None:
    msg = validations_pb2.DoubleFinite()
    msg.val = float("-inf")

    expected_violation = Violation(
        message="must be finite",
        rule_id="double.finite",
        field_value=msg.val,
        rule_value=True,
    )

    with pytest.raises(protovalidate.ValidationError) as exc_info:
        validator.validate(msg)
    e = exc_info.value
    assert str(e) == f"invalid {msg.DESCRIPTOR.name}"
    compare_violations(e.violations, [expected_violation])  # ty: ignore

    violations = validator.collect_violations(msg)
    compare_violations(violations, [expected_violation])


@pytest.mark.parametrize("validator", validators)
def test_legacy_message_map_key(validator: ValidatorProtocol) -> None:
    msg = validations_pb2.MapKeys()
    msg.val[1] = "a"

    expected_violation = Violation(
        message="must be less than 0",
        rule_id="sint64.lt",
        for_key=True,
        field_value=1,
        rule_value=0,
    )

    violations = validator.collect_violations(msg)
    compare_violations(violations, [expected_violation])


def check_invalid(
    validator: ValidatorProtocol, msg: google_message.Message, expected: list[Violation]
) -> None:
    # Test validate
    with pytest.raises(protovalidate.ValidationError) as exc_info:
        validator.validate(msg)
    e = exc_info.value
    assert str(e) == f"invalid {msg.DESCRIPTOR.name}"
    compare_violations(e.violations, expected)  # ty: ignore

    # Test collect_violations
    violations = validator.collect_violations(msg)
    compare_violations(violations, expected)
