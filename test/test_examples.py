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

"""Validator tests over the example messages.

Each test builds a message, runs the validator and checks the outcome: valid,
the violations in order, or the kind of error.
"""

from __future__ import annotations

import time
from typing import Any as AnyType

import protobuf
import pytest
from protobuf import Oneof
from protobuf.wkt import Any, Api, FieldMask, SourceContext, Timestamp

import protovalidate
from protovalidate import Violation

from ._utils import ValidatorProtocol, check_valid
from .conftest import make_validator
from .gen.tests.example.v1 import end_to_end_pb, predefined_pb, rules_pb, validator_pb

validators: list[ValidatorProtocol] = [
    protovalidate,  # global module singleton
    make_validator(),
]


def check_invalid(
    validator: ValidatorProtocol, msg: protobuf.Message
) -> list[Violation]:
    """Asserts that the message has violations and returns them."""
    with pytest.raises(protovalidate.ValidationError):
        validator.validate(msg)
    violations = validator.collect_violations(msg)
    assert len(violations) > 0
    return violations


def rule_ids(violations: list[Violation]) -> list[str]:
    return [violation.proto.rule_id for violation in violations]


# Custom CEL rules, oneofs, containers, well-known types and recursion


@pytest.mark.parametrize("validator", validators)
def test_has_msg_exprs(validator: ValidatorProtocol) -> None:
    check_valid(validator, validator_pb.HasMsgExprs(x=2, y=43))
    check_invalid(validator, validator_pb.HasMsgExprs(x=9, y=8))


@pytest.mark.parametrize("validator", validators)
def test_recursive(validator: ValidatorProtocol) -> None:
    check_valid(
        validator,
        validator_pb.SelfRecursive(x=123, turtle=validator_pb.SelfRecursive(x=456)),
    )
    check_valid(validator, validator_pb.LoopRecursiveA(b=validator_pb.LoopRecursiveB()))


@pytest.mark.parametrize("validator", validators)
def test_oneof(validator: ValidatorProtocol) -> None:
    check_valid(validator, validator_pb.MsgHasOneof(o=Oneof(field="x", value="foo")))
    check_valid(validator, validator_pb.MsgHasOneof(o=Oneof(field="y", value=42)))
    check_valid(
        validator,
        validator_pb.MsgHasOneof(
            o=Oneof(field="msg", value=validator_pb.HasMsgExprs(x=4, y=50))
        ),
    )
    check_invalid(validator, validator_pb.MsgHasOneof())


@pytest.mark.parametrize("validator", validators)
def test_repeated(validator: ValidatorProtocol) -> None:
    check_valid(
        validator,
        validator_pb.MsgHasRepeated(
            x=[1, 2, 3],
            y=["foo", "bar"],
            z=[
                validator_pb.HasMsgExprs(x=4, y=55),
                validator_pb.HasMsgExprs(x=4, y=60),
            ],
        ),
    )


@pytest.mark.parametrize("validator", validators)
def test_map(validator: ValidatorProtocol) -> None:
    check_invalid(
        validator,
        validator_pb.MsgHasMap(
            int32map={-1: 1, 2: 2},
            string_map={"foo": "foo", "bar": "bar", "baz": "baz"},
            message_map={0: validator_pb.LoopRecursiveA()},
        ),
    )


@pytest.mark.parametrize("validator", validators)
def test_transitive_field_rules(validator: ValidatorProtocol) -> None:
    check_valid(
        validator,
        validator_pb.TransitiveFieldRule(mask=FieldMask(paths=["foo", "bar"])),
    )


@pytest.mark.parametrize("validator", validators)
def test_multiple_steps_transitive_field_rules(validator: ValidatorProtocol) -> None:
    check_valid(
        validator,
        validator_pb.MultipleStepsTransitiveFieldRules(
            api=Api(source_context=SourceContext(file_name="path/file"))
        ),
    )


@pytest.mark.parametrize("validator", validators)
def test_field_of_type_any(validator: ValidatorProtocol) -> None:
    check_valid(
        validator,
        validator_pb.FieldOfTypeAny(any=Any.pack(validator_pb.Simple(s="foo"))),
    )


@pytest.mark.parametrize("validator", validators)
def test_cel_map_on_a_repeated(validator: ValidatorProtocol) -> None:
    values = [
        validator_pb.CelMapOnARepeated.Value(name=n) for n in ("foo", "bar", "baz")
    ]
    check_valid(validator, validator_pb.CelMapOnARepeated(values=values))
    values.append(validator_pb.CelMapOnARepeated.Value(name="foo"))
    check_invalid(validator, validator_pb.CelMapOnARepeated(values=values))


@pytest.mark.parametrize("validator", validators)
def test_repeated_item_cel(validator: ValidatorProtocol) -> None:
    check_valid(validator, validator_pb.RepeatedItemCel(paths=["foo"]))
    violations = check_invalid(
        validator, validator_pb.RepeatedItemCel(paths=["foo", " bar"])
    )
    assert violations[0].proto.rule_id == "paths.no_space"


@pytest.mark.parametrize("validator", validators)
def test_issue211(validator: ValidatorProtocol) -> None:
    check_valid(
        validator, validator_pb.Issue211(value=Timestamp(seconds=int(time.time()) + 60))
    )


@pytest.mark.parametrize("validator", validators)
def test_issue141(validator: ValidatorProtocol) -> None:
    check_invalid(validator, validator_pb.FieldWithIssue())
    check_invalid(
        validator,
        validator_pb.OneTwo(
            field1=validator_pb.F1(field=validator_pb.FieldWithIssue())
        ),
    )
    check_invalid(
        validator,
        validator_pb.TwoOne(
            field1=validator_pb.F1(field=validator_pb.FieldWithIssue())
        ),
    )


@pytest.mark.parametrize("validator", validators)
def test_issue148(validator: ValidatorProtocol) -> None:
    check_valid(validator, predefined_pb.Issue148(test=1))


@pytest.mark.parametrize("validator", validators)
def test_issue187(validator: ValidatorProtocol) -> None:
    check_valid(validator, predefined_pb.Issue187(false_field=False, true_field=True))


@pytest.mark.parametrize("validator", validators)
def test_issue307_map(validator: ValidatorProtocol) -> None:
    check_valid(
        validator,
        validator_pb.SelfReferentialMap(
            name="child",
            children={
                "a": validator_pb.SelfReferentialMap(name="child"),
                "b": validator_pb.SelfReferentialMap(name="child"),
            },
        ),
    )
    check_invalid(
        validator,
        validator_pb.SelfReferentialMap(
            name="parent",
            children={
                "a": validator_pb.SelfReferentialMap(name="parent"),
                "b": validator_pb.SelfReferentialMap(),
            },
        ),
    )


@pytest.mark.parametrize("validator", validators)
def test_issue307_repeated(validator: ValidatorProtocol) -> None:
    check_valid(
        validator,
        validator_pb.SelfReferentialRepeated(
            name="child",
            children=[
                validator_pb.SelfReferentialRepeated(name="child"),
                validator_pb.SelfReferentialRepeated(name="child"),
            ],
        ),
    )
    check_invalid(
        validator,
        validator_pb.SelfReferentialRepeated(
            name="parent",
            children=[
                validator_pb.SelfReferentialRepeated(name="parent"),
                validator_pb.SelfReferentialRepeated(),
            ],
        ),
    )


@pytest.mark.parametrize("validator", validators)
def test_issue296(validator: ValidatorProtocol) -> None:
    check_valid(validator, predefined_pb.Issue296(fm=FieldMask(paths=["a"])))


# Several rules on one field, and uniqueness


@pytest.mark.parametrize("validator", validators)
def test_native_bytes(validator: ValidatorProtocol) -> None:
    violations = check_invalid(
        validator, rules_pb.BenchTestBytes(b1=b"\x32\x33", b=b"\x03\x04")
    )
    assert [
        (v.proto.field.elements[0].field_name, v.proto.message) for v in violations
    ] == [
        ("b1", "must not be in list [23, 45, 67]"),
        ("b", "must be in list [23, 45, 67]"),
    ]


@pytest.mark.parametrize("validator", validators)
def test_native_bytes_broken(validator: ValidatorProtocol) -> None:
    violations = check_invalid(
        validator, rules_pb.TestByteBroken(broken=b"greetings and salutations")
    )
    assert len(violations) == 2


@pytest.mark.parametrize("validator", validators)
def test_native_unique_enums(validator: ValidatorProtocol) -> None:
    check_invalid(validator, rules_pb.TestUnique(enums=[1, 1, 2, 3]))
    check_valid(validator, rules_pb.TestUnique(enums=[1, 2, 3]))


@pytest.mark.parametrize("validator", validators)
def test_native_unique_strings(validator: ValidatorProtocol) -> None:
    check_invalid(validator, rules_pb.TestUnique(strings=["a", "b", "a"]))
    check_valid(validator, rules_pb.TestUnique(strings=["a", "b", "c"]))


@pytest.mark.parametrize("validator", validators)
def test_native_unique_bytes(validator: ValidatorProtocol) -> None:
    check_invalid(validator, rules_pb.TestUnique(bytes=[b"a", b"b", b"a"]))
    check_valid(validator, rules_pb.TestUnique(bytes=[b"a", b"b", b"c"]))


@pytest.mark.parametrize("validator", validators)
def test_multi_rule(validator: ValidatorProtocol) -> None:
    violations = check_invalid(validator, rules_pb.MultiRule(many=1))
    assert len(violations) == 2
    assert violations[0].field_value == 1


# One rule per message: a good value, then a bad one with its violation

END_TO_END: list[tuple[type[protobuf.Message], AnyType, AnyType, str, str]] = [
    (end_to_end_pb.Int32Gt, 1, 0, "int32.gt", "must be greater than 0"),
    (
        end_to_end_pb.Uint64Gte,
        10,
        9,
        "uint64.gte",
        "must be greater than or equal to 10",
    ),
    (end_to_end_pb.DoubleLt, 50.0, 100.0, "double.lt", "must be less than 100"),
    (
        end_to_end_pb.StringMinLen,
        "abc",
        "ab",
        "string.min_len",
        "must be at least 3 characters",
    ),
    (
        end_to_end_pb.StringPrefix,
        "hello world",
        "world",
        "string.prefix",
        "does not have prefix `hello`",
    ),
    (end_to_end_pb.BoolConst, True, False, "bool.const", "must equal true"),
    (
        end_to_end_pb.BytesMinLen,
        b"\x01\x02",
        b"\x01",
        "bytes.min_len",
        "must be at least 2 bytes",
    ),
    (
        end_to_end_pb.RepeatedMinItems,
        [1, 2, 3],
        [1],
        "repeated.min_items",
        "must contain at least 2 item(s)",
    ),
    (
        end_to_end_pb.RepeatedMaxItems,
        [1],
        [1, 2, 3],
        "repeated.max_items",
        "must contain no more than 2 item(s)",
    ),
    (
        end_to_end_pb.RepeatedUnique,
        [1, 2, 3],
        [1, 2, 1],
        "repeated.unique",
        "repeated value must contain unique items",
    ),
    (
        end_to_end_pb.RepeatedUniqueItems,
        [2, 4, 6],
        [2, 1, 3],
        "int32.gte",
        "must be greater than or equal to 2",
    ),
    (
        end_to_end_pb.RepeatedUniqueMaxItems,
        [2, 4, 6],
        [2, 6, 3, 5],
        "repeated.max_items",
        "must contain no more than 3 item(s)",
    ),
    (end_to_end_pb.EnumConst, 1, 2, "enum.const", "must equal 1"),
    (end_to_end_pb.EnumIn, 1, 3, "enum.in", "must be in list [1, 2]"),
]


@pytest.mark.parametrize("validator", validators)
@pytest.mark.parametrize(("msg_type", "good", "bad", "rule_id", "message"), END_TO_END)
def test_end_to_end(
    validator: ValidatorProtocol,
    msg_type: type[protobuf.Message],
    good: AnyType,
    bad: AnyType,
    rule_id: str,
    message: str,
) -> None:
    check_valid(validator, msg_type(value=good))
    violations = check_invalid(validator, msg_type(value=bad))
    assert [(v.proto.rule_id, v.proto.message) for v in violations] == [
        (rule_id, message)
    ]


@pytest.mark.parametrize("validator", validators)
@pytest.mark.parametrize(
    ("msg_type", "good", "bad", "rule_id", "message"),
    [
        (
            end_to_end_pb.MapMaxPairs,
            {"a": "1", "b": "2"},
            {"a": "1", "b": "2", "c": "3"},
            "map.max_pairs",
            "map must be at most 2 entries",
        )
    ],
)
def test_end_to_end_map(
    validator: ValidatorProtocol,
    msg_type: type[protobuf.Message],
    good: dict[str, str],
    bad: dict[str, str],
    rule_id: str,
    message: str,
) -> None:
    check_valid(validator, msg_type(entries=good))
    violations = check_invalid(validator, msg_type(entries=bad))
    assert [(v.proto.rule_id, v.proto.message) for v in violations] == [
        (rule_id, message)
    ]


@pytest.mark.parametrize("validator", validators)
@pytest.mark.parametrize(
    ("msg_type", "value", "expected"),
    [
        (end_to_end_pb.EnumDefinedOnlyIn, 1, []),
        (end_to_end_pb.EnumDefinedOnlyIn, 0, ["enum.in"]),
        (end_to_end_pb.EnumDefinedOnlyIn, 99, ["enum.defined_only", "enum.in"]),
        (end_to_end_pb.EnumDefinedOnlyConst, 1, []),
        (end_to_end_pb.EnumDefinedOnlyConst, 2, ["enum.const"]),
        (end_to_end_pb.EnumDefinedOnlyConst, 99, ["enum.const", "enum.defined_only"]),
        (end_to_end_pb.EnumDefinedOnlyNotIn, 1, []),
        (end_to_end_pb.EnumDefinedOnlyNotIn, 0, ["enum.not_in"]),
        (end_to_end_pb.EnumDefinedOnlyNotIn, 99, ["enum.defined_only"]),
    ],
)
def test_enum_combined_rules(
    validator: ValidatorProtocol,
    msg_type: type[protobuf.Message],
    value: int,
    expected: list[str],
) -> None:
    """`defined_only` combined with another enum rule, in validate.proto's order."""
    assert rule_ids(validator.collect_violations(msg_type(value=value))) == expected


# One bad value per rule, checking the rule id and the rule value a violation
# reports.

RULE_VALUES: list[tuple[type[protobuf.Message], AnyType, str, AnyType]] = [
    (end_to_end_pb.Int32Const, 4, "int32.const", 5),
    (end_to_end_pb.Int32In, 9, "int32.in", [1, 2, 3]),
    (end_to_end_pb.Int32NotIn, 7, "int32.not_in", [7]),
    (end_to_end_pb.Int32Gt, -1, "int32.gt", 0),
    (end_to_end_pb.Int32GtLt, 20, "int32.gt_lt", 0),
    (end_to_end_pb.StringConst, "b", "string.const", "a"),
    (end_to_end_pb.StringIn, "z", "string.in", ["a", "b"]),
    (end_to_end_pb.StringNotIn, "z", "string.not_in", ["z"]),
    (end_to_end_pb.StringPrefix, "bar", "string.prefix", "hello"),
    (end_to_end_pb.StringSuffix, "foo", "string.suffix", "bar"),
    (end_to_end_pb.StringContains, "absent", "string.contains", "mid"),
    (end_to_end_pb.StringNotContains, "badger", "string.not_contains", "bad"),
    (end_to_end_pb.StringPattern, "123", "string.pattern", "^[a-z]+$"),
    (end_to_end_pb.StringMinLen, "ab", "string.min_len", 3),
    (end_to_end_pb.StringMinBytes, "ab", "string.min_bytes", 5),
    (end_to_end_pb.StringEmail, "not-an-email", "string.email", True),
    (end_to_end_pb.StringUuid, "not-a-uuid", "string.uuid", True),
    (
        end_to_end_pb.StringHeaderName,
        "bad header",
        "string.well_known_regex.header_name",
        1,
    ),
    (end_to_end_pb.BytesConst, b"\x02", "bytes.const", b"\x01"),
    (end_to_end_pb.BytesIn, b"\x09", "bytes.in", [b"\x01", b"\x02"]),
    (end_to_end_pb.BytesNotIn, b"\x07", "bytes.not_in", [b"\x07"]),
    (end_to_end_pb.BytesMinLen, b"\x01", "bytes.min_len", 2),
    (end_to_end_pb.BytesPrefix, b"\x0b", "bytes.prefix", b"\x0a"),
    (end_to_end_pb.BytesIp, b"\x01\x02\x03", "bytes.ip", True),
    (end_to_end_pb.BoolConst, False, "bool.const", True),
    (end_to_end_pb.RepeatedMinItems, [1], "repeated.min_items", 2),
    (end_to_end_pb.RepeatedUnique, [1, 1], "repeated.unique", True),
    (end_to_end_pb.EnumConst, 2, "enum.const", 1),
    (end_to_end_pb.EnumIn, 0, "enum.in", [1, 2]),
    (end_to_end_pb.EnumNotIn, 0, "enum.not_in", [0]),
]


@pytest.mark.parametrize("validator", validators)
@pytest.mark.parametrize(("msg_type", "bad", "rule_id", "rule_value"), RULE_VALUES)
def test_rule_values(
    validator: ValidatorProtocol,
    msg_type: type[protobuf.Message],
    bad: AnyType,
    rule_id: str,
    rule_value: AnyType,
) -> None:
    violations = check_invalid(validator, msg_type(value=bad))
    assert len(violations) == 1
    assert violations[0].proto.rule_id == rule_id
    assert violations[0].rule_value == rule_value
