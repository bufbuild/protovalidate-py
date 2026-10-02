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

import math

import protobuf
import pytest
from protobuf import Oneof
from protobuf.wkt import Duration, Int32Value, Timestamp

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
        "field xxx not found in message tests.example.v1.ProtovalidateOneofUnknownFieldName",
    )


@pytest.mark.parametrize("validator", validators)
def test_protovalidate_mistyped_rule(validator: ValidatorProtocol) -> None:
    """Tests that a compilation error is thrown when a rule's type does not match the field's type."""
    msg = validations_pb.ProtovalidateMistypedRule()

    check_compilation_errors(
        validator,
        msg,
        'expected rule "buf.validate.FieldRules.string", got "buf.validate.FieldRules.duration" on field "tests.example.v1.ProtovalidateMistypedRule.val"',
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


@pytest.mark.parametrize("validator", validators)
def test_nested_compilation_error_reached_lazily(validator: ValidatorProtocol) -> None:
    """A nested type's rule that does not compile fails only a validation that reaches it."""
    check_valid(validator, validations_pb.NestedMistypedRule())

    check_compilation_errors(
        validator,
        validations_pb.NestedMistypedRule(
            child=validations_pb.ProtovalidateMistypedRule()
        ),
        'expected rule "buf.validate.FieldRules.string", got "buf.validate.FieldRules.duration" on field "tests.example.v1.ProtovalidateMistypedRule.val"',
    )


@pytest.mark.parametrize("validator", validators)
def test_nested_message_rule_error(validator: ValidatorProtocol) -> None:
    """A field whose message type's own rules do not compile fails with that, set or not."""
    msg = validations_pb.NestedMessageRuleError()

    with pytest.raises(protovalidate.CompilationError) as exc_info:
        validator.validate(msg)
    assert str(exc_info.value).startswith(
        "failed to compile embedded type tests.example.v1.MessageRuleError "
        "for tests.example.v1.NestedMessageRuleError.child: "
    )


@pytest.mark.parametrize("validator", validators)
def test_violation_before_compilation_error(validator: ValidatorProtocol) -> None:
    """A compilation error is reported when its field is reached, so failing fast on an earlier violation never gets there."""
    msg = validations_pb.ViolationBeforeError(a="x")

    expected_violation = Violation(
        message="must be at least 5 characters",
        rule_id="string.min_len",
        field_value="x",
        rule_value=5,
    )

    violations = validator.collect_violations(msg, fail_fast=True)
    compare_violations(violations, [expected_violation])

    check_compilation_errors(
        validator,
        msg,
        'expected rule "buf.validate.FieldRules.string", '
        'got "buf.validate.FieldRules.duration" '
        'on field "tests.example.v1.ViolationBeforeError.b"',
    )


@pytest.mark.parametrize("validator", validators)
@pytest.mark.parametrize(
    ("msg", "expected"),
    [
        (
            validations_pb.RepeatedFieldScalarRule(val=["abcd"]),
            'expected rule "buf.validate.FieldRules.repeated", got "buf.validate.FieldRules.string" on field "tests.example.v1.RepeatedFieldScalarRule.val"',
        ),
        (
            validations_pb.RepeatedFieldWrapperRule(val=[Int32Value(value=100)]),
            'expected rule "buf.validate.FieldRules.repeated", got "buf.validate.FieldRules.int32" on field "tests.example.v1.RepeatedFieldWrapperRule.val"',
        ),
        (
            validations_pb.RepeatedFieldEnumRule(val=[1]),
            'expected rule "buf.validate.FieldRules.repeated", got "buf.validate.FieldRules.enum" on field "tests.example.v1.RepeatedFieldEnumRule.val"',
        ),
        (
            validations_pb.RepeatedFieldDurationRule(val=[Duration(seconds=2)]),
            'expected rule "buf.validate.FieldRules.repeated", got "buf.validate.FieldRules.duration" on field "tests.example.v1.RepeatedFieldDurationRule.val"',
        ),
        (
            validations_pb.MapFieldScalarRule(val={"abcd": "abcd"}),
            'expected rule "buf.validate.FieldRules.map", got "buf.validate.FieldRules.string" on field "tests.example.v1.MapFieldScalarRule.val"',
        ),
        (
            validations_pb.RepeatedUniqueMessages(val=[validations_pb.Embed(val=1)]),
            "repeated.unique is not supported for message items",
        ),
        (
            validations_pb.TimestampOutOfRange(val=Timestamp(seconds=1)),
            (
                "timestamp rule value out of range: seconds 253402300800 nanos 0 "
                "is not between 0001-01-01T00:00:00Z and 9999-12-31T23:59:59.999999999Z"
            ),
        ),
    ],
)
def test_rules_that_do_not_apply_to_field(
    validator: ValidatorProtocol, msg: protobuf.Message, expected: str
) -> None:
    """Rules that cannot apply to their field fail to compile rather than test the wrong value."""
    check_compilation_errors(validator, msg, expected)


@pytest.mark.parametrize("validator", validators)
def test_cel_unique_on_messages(validator: ValidatorProtocol) -> None:
    """`unique()` on a list of messages has no overload, and says so rather than passing."""
    msg = validations_pb.CelUniqueMessages(
        val=[validations_pb.Embed(val=1), validations_pb.Embed(val=1)]
    )
    with pytest.raises(protovalidate.EvaluationError):
        validator.validate(msg)
    with pytest.raises(protovalidate.EvaluationError):
        validator.collect_violations(msg)


@pytest.mark.parametrize("validator", validators)
def test_timestamp_rule_order(validator: ValidatorProtocol) -> None:
    """`const` is checked before `lt_now`, as validate.proto declares them."""
    msg = validations_pb.TimestampRuleOrder(val=Timestamp(seconds=32503680000))
    violations = validator.collect_violations(msg)
    assert [v.proto.rule_id for v in violations] == [
        "timestamp.const",
        "timestamp.lt_now",
    ]


@pytest.mark.parametrize("validator", validators)
def test_enum_rule_order(validator: ValidatorProtocol) -> None:
    """Enum rules are checked in the order validate.proto declares them."""
    msg = validations_pb.EnumRuleOrder(val=5)
    violations = validator.collect_violations(msg)
    assert [v.proto.rule_id for v in violations] == [
        "enum.const",
        "enum.defined_only",
        "enum.in",
        "enum.not_in",
    ]


@pytest.mark.parametrize("validator", validators)
def test_hostname_length_excludes_trailing_dot(validator: ValidatorProtocol) -> None:
    longest = ".".join(["a" * 63] * 3 + ["a" * 61])
    assert len(longest) == 253
    check_valid(validator, validations_pb.Hostname(val=longest))
    check_valid(validator, validations_pb.Hostname(val=longest + "."))

    too_long = ".".join(["a" * 63] * 3 + ["a" * 62])
    assert len(too_long) == 254
    for val in (too_long, too_long + "."):
        violations = validator.collect_violations(validations_pb.Hostname(val=val))
        assert [v.proto.rule_id for v in violations] == ["string.hostname"]


@pytest.mark.parametrize("validator", validators)
def test_infinity_in_messages(validator: ValidatorProtocol) -> None:
    """Infinite rule values print as CEL's `Infinity`, not Rust's `inf`."""
    violations = validator.collect_violations(validations_pb.DoubleInfinity(val=1.0))
    assert [v.proto.message for v in violations] == ["must equal Infinity"]
    check_valid(validator, validations_pb.DoubleInfinity(val=math.inf))

    violations = validator.collect_violations(
        validations_pb.DoubleInfiniteRange(val=math.inf)
    )
    assert [v.proto.message for v in violations] == [
        "must be greater than -Infinity and less than Infinity"
    ]
    check_valid(validator, validations_pb.DoubleInfiniteRange(val=1.0))


@pytest.mark.parametrize("validator", validators)
def test_pattern_is_re2(validator: ValidatorProtocol) -> None:
    """Patterns follow RE2, the dialect the other implementations and CEL's `matches()` use."""
    check_valid(validator, validations_pb.PatternAsciiDigits(val="123"))
    violations = validator.collect_violations(
        validations_pb.PatternAsciiDigits(val="\u0661\u0662\u0663")
    )
    assert [v.proto.rule_id for v in violations] == ["string.pattern"]

    check_valid(validator, validations_pb.PatternQuotedLiteral(val="a.c"))
    violations = validator.collect_violations(
        validations_pb.PatternQuotedLiteral(val="abc")
    )
    assert [v.proto.rule_id for v in violations] == ["string.pattern"]

    with pytest.raises(protovalidate.CompilationError) as exc_info:
        validator.validate(validations_pb.PatternRepeatTooLarge(val="a"))
    assert str(exc_info.value).startswith("failed to compile program string.pattern: ")


@pytest.mark.parametrize("validator", validators)
@pytest.mark.parametrize(
    "msg_type", [validations_pb.UniqueWrappers, validations_pb.CelUniqueWrappers]
)
def test_unique_wrappers(
    validator: ValidatorProtocol, msg_type: type[protobuf.Message]
) -> None:
    """`unique` compares wrapper elements by the scalars they hold."""
    check_valid(validator, msg_type(val=[Int32Value(value=1), Int32Value(value=2)]))
    violations = validator.collect_violations(
        msg_type(val=[Int32Value(value=1), Int32Value(value=1)])
    )
    assert len(violations) == 1
