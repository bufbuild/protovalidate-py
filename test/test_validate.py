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

import protobuf
import pytest
from protobuf import Oneof

import protovalidate
from protovalidate import Violation

from ._utils import (
    ValidatorProtocol,
    check_compilation_errors,
    check_valid,
    compare_violations,
)
from .conftest import make_validator
from .gen.tests.example.v1 import validations_pb

validators: list[ValidatorProtocol] = [
    protovalidate,  # global module singleton
    make_validator(),
]


@pytest.mark.parametrize("validator", validators)
def test_ninf(validator: ValidatorProtocol) -> None:
    msg = validations_pb.DoubleFinite()
    msg.val = float("-inf")

    expected_violation = Violation(
        message="must be finite",
        rule_id="double.finite",
        field_value=msg.val,
        rule_value=True,
    )

    check_invalid(validator, msg, [expected_violation])


@pytest.mark.parametrize("validator", validators)
def test_map_key(validator: ValidatorProtocol) -> None:
    msg = validations_pb.MapKeys()
    msg.val[1] = "a"

    expected_violation = Violation(
        message="must be less than 0",
        rule_id="sint64.lt",
        for_key=True,
        field_value=1,
        rule_value=0,
    )

    check_invalid(validator, msg, [expected_violation])


@pytest.mark.parametrize("validator", validators)
def test_sfixed64_valid(validator: ValidatorProtocol) -> None:
    msg = validations_pb.SFixed64ExLTGT(val=11)

    check_valid(validator, msg)


@pytest.mark.parametrize("validator", validators)
def test_oneofs(validator: ValidatorProtocol) -> None:
    msg = validations_pb.Oneof(o=Oneof(field="y", value=123))

    check_valid(validator, msg)


@pytest.mark.parametrize("validator", validators)
def test_protovalidate_oneof_valid(validator: ValidatorProtocol) -> None:
    msg = validations_pb.ProtovalidateOneof()
    msg.a = "A"

    check_valid(validator, msg)


@pytest.mark.parametrize("validator", validators)
def test_protovalidate_oneof_violation(validator: ValidatorProtocol) -> None:
    msg = validations_pb.ProtovalidateOneof()
    msg.a = "A"
    msg.b = "B"

    expected_violation = Violation(
        message="only one of a, b can be set", rule_id="message.oneof"
    )

    check_invalid(validator, msg, [expected_violation])


@pytest.mark.parametrize("validator", validators)
def test_protovalidate_oneof_required_violation(validator: ValidatorProtocol) -> None:
    msg = validations_pb.ProtovalidateOneofRequired()

    expected_violation = Violation(
        message="one of a, b must be set", rule_id="message.oneof"
    )

    check_invalid(validator, msg, [expected_violation])


@pytest.mark.parametrize("validator", validators)
def test_protovalidate_oneof_unknown_field_name(validator: ValidatorProtocol) -> None:
    """Tests that a compilation error is thrown when specifying a oneof rule with an invalid field name."""
    msg = validations_pb.ProtovalidateOneofUnknownFieldName()

    check_compilation_errors(
        validator,
        msg,
        'field "xxx" not found in message tests.example.v1.ProtovalidateOneofUnknownFieldName',
    )


@pytest.mark.parametrize("validator", validators)
def test_protovalidate_mistyped_rule(validator: ValidatorProtocol) -> None:
    """Tests that a compilation error is thrown when a rule's type does not match the field's type."""
    msg = validations_pb.ProtovalidateMistypedRule()

    check_compilation_errors(
        validator, msg, "duration field validator on non-duration field"
    )


@pytest.mark.parametrize("validator", validators)
def test_repeated(validator: ValidatorProtocol) -> None:
    msg = validations_pb.RepeatedEmbedSkip(val=[validations_pb.Embed(val=-1)])

    check_valid(validator, msg)


@pytest.mark.parametrize("validator", validators)
def test_maps(validator: ValidatorProtocol) -> None:
    msg = validations_pb.MapMinMax()

    expected_violation = Violation(
        message="map must be at least 2 entries",
        rule_id="map.min_pairs",
        field_value={},
        rule_value=2,
    )

    check_invalid(validator, msg, [expected_violation])


@pytest.mark.parametrize("validator", validators)
def test_timestamp(validator: ValidatorProtocol) -> None:
    msg = validations_pb.TimestampGTNow()

    check_valid(validator, msg)


@pytest.mark.parametrize("validator", validators)
def test_multiple_validations(validator: ValidatorProtocol) -> None:
    """Test that a message with multiple violations correctly returns all of them."""
    msg = validations_pb.MultipleValidations()
    msg.title = "bar"
    msg.name = "blah"

    expected_violation1 = Violation(
        message="does not have prefix `foo`",
        rule_id="string.prefix",
        field_value=msg.title,
        rule_value="foo",
    )

    expected_violation2 = Violation(
        message="must be at least 5 characters",
        rule_id="string.min_len",
        field_value=msg.name,
        rule_value=5,
    )

    check_invalid(validator, msg, [expected_violation1, expected_violation2])


@pytest.mark.parametrize("validator", validators)
def test_concatenated_values(validator: ValidatorProtocol) -> None:
    msg = validations_pb.ConcatenatedValues(bar=["a", "b", "c"], baz=["d", "e", "f"])

    check_valid(validator, msg)


@pytest.mark.parametrize("validator", validators)
def test_fail_fast(validator: ValidatorProtocol) -> None:
    """Test that fail fast correctly fails on first violation."""
    msg = validations_pb.MultipleValidations()
    msg.title = "bar"
    msg.name = "blah"

    expected_violation = Violation(
        message="does not have prefix `foo`",
        rule_id="string.prefix",
        field_value=msg.title,
        rule_value="foo",
    )

    # Test validate
    with pytest.raises(protovalidate.ValidationError) as cm:
        validator.validate(msg, fail_fast=True)
    e = cm.value
    assert str(e) == f"invalid {type(msg).desc().name}"
    compare_violations(e.violations, [expected_violation])  # ty: ignore

    # Test collect_violations
    violations = validator.collect_violations(msg, fail_fast=True)
    compare_violations(violations, [expected_violation])


def check_invalid(
    validator: ValidatorProtocol, msg: protobuf.Message, expected: list[Violation]
) -> None:
    # Test validate
    with pytest.raises(protovalidate.ValidationError) as exc_info:
        validator.validate(msg)
    e = exc_info.value
    if isinstance(msg, protobuf.Message):
        assert str(e) == f"invalid {type(msg).desc().name}"
    else:
        assert str(e) == f"invalid {msg.DESCRIPTOR.name}"
    compare_violations(e.violations, expected)  # ty: ignore

    # Test collect_violations
    violations = validator.collect_violations(msg)
    compare_violations(violations, expected)
