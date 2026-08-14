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

"""Pydantic mirrors of the benchmark protos, for comparative benchmarking.

Each model expresses the same rules as the `(buf.validate.field)` options on the
corresponding message in `test/proto/bench/v1`. Two principles decide how:

- Where pydantic has a native feature for a rule, use it, even where it behaves
  differently from ours. Pydantic's `UUID` is a real parser where `string.uuid`
  is a regex; that is pydantic being better at the job, and pretending otherwise
  would understate it.
- Where the rule is validation logic we wrote ourselves, reuse *our*
  implementation rather than reimplementing it. `string.hostname` and
  `string.host_and_port` call into `_bench_support`, which holds the last Python
  copy of those predicates (the live ones are now C++), and
  `string.email` reuses our pattern. This keeps the benchmark honest
  about what it measures — the framework around the rule, not the rule — and
  makes drift impossible.

Divergences that survive those two rules are noted at each site.

`to_payload` performs the manual proto -> pydantic-input conversion, producing
plain Python values. `from_proto` runs it through validation. The benchmark
builds the payload up front and times `model_validate` alone, so proto field
access and dict building stay out of the measurement.
"""

from __future__ import annotations

import enum
import ipaddress
from typing import TYPE_CHECKING, Annotated, Any, Literal
from uuid import UUID

from pydantic import AfterValidator, BaseModel, Field
from pydantic.networks import IPv4Address, IPv6Address, IPvAnyAddress

from .gen.bench.v1 import bench_pb, native_pb

if TYPE_CHECKING:
    from collections.abc import Callable


# Rule predicates lifted verbatim from the removed `protovalidate/_funcs.py`,
# except `_is_ip`, which used hand-written parsers that matched CEL semantics and
# pulled in more of that module than is worth carrying; `ipaddress` agrees with
# them on whether a host is an address, which is all it is used for here.
#
# The live implementations are now C++ inside protovalidate-cc and cannot be
# called from Python, so unlike the rest of this file these can drift from what
# the validator does. If the benchmark ever disagrees with it on a hostname or
# host_and_port case, suspect these first.
# Lifted verbatim from the removed `protovalidate/_funcs.py`.
EMAIL_PATTERN = (
    r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?"
    r"(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$"
)


def _is_hostname(val: str) -> bool:
    """Internal implementation."""
    string = val.removesuffix(".").lower()

    # The 253-character limit excludes the optional trailing dot (RFC 3339) which
    # we just stripped.
    if len(string) > 253:
        return False

    all_digits = False
    parts = string.lower().split(sep=".")

    # split hostname on '.' and validate each part
    for part in parts:
        all_digits = True

        # if part is empty, longer than 63 chars, or starts/ends with '-', it is invalid
        part_len = len(part)

        if part_len == 0 or part_len > 63 or part.startswith("-") or part.endswith("-"):
            return False

        for c in part:
            # if the character is not a-z, 0-9, or '-', it is invalid
            if (c < "a" or c > "z") and (c < "0" or c > "9") and c != "-":
                return False

            all_digits = all_digits and "0" <= c <= "9"

    # the last part cannot be all numbers
    return not all_digits


def _is_host_and_port(val: str, *, port_required: bool = False) -> bool:
    if len(val) == 0:
        return False

    split_idx = val.rfind(":")

    if val[0] == "[":
        end = val.rfind("]")
        end_plus = end + 1

        if end_plus == len(val):
            return not port_required and _is_ip(val[1:end], 6)
        if end_plus == split_idx:
            return _is_ip(val[1:end], 6) and _is_port(val[split_idx + 1 :])
        # malformed
        return False

    if split_idx < 0:
        return not port_required and (_is_hostname(val) or _is_ip(val, 4))

    host = val[0:split_idx]
    port = val[split_idx + 1 :]

    return (_is_hostname(host) or _is_ip(host, 4)) and _is_port(port)


def _is_port(val: str) -> bool:
    if len(val) == 0:
        return False
    if len(val) > 1 and val[0] == "0":
        return False
    for c in val:
        if c < "0" or c > "9":
            return False
    try:
        return int(val) <= 65535
    except ValueError:
        # Error converting to number
        return False


def _is_ip(string: str, version: int) -> bool:
    """Whether `string` is an IP address of `version` (0 means either).

    protovalidate's own implementation uses hand-written parsers, which exist to
    match CEL semantics exactly and pull in more of the removed module than is
    worth carrying here. `ipaddress` agrees with them on whether a host is an
    address, which is all this is used for.
    """
    try:
        parsed = ipaddress.ip_address(string)
    except ValueError:
        return False
    if version == 0:
        return True
    return parsed.version == version


def _unique(value: list[Any]) -> list[Any]:
    """Mirrors `repeated.unique`; pydantic v2 dropped `unique_items`.

    Same short-circuiting running-set walk as `_extra_func.cel_unique`.
    """
    seen: set[Any] = set()
    for item in value:
        if item in seen:
            msg = "repeated value must contain unique items"
            raise ValueError(msg)
        seen.add(item)
    return value


_UniqueFloats = Annotated[list[float], AfterValidator(_unique)]
_UniqueBytes = Annotated[list[bytes], AfterValidator(_unique)]


def _hostname(value: str) -> str:
    """Mirrors `string.hostname` by calling the same code the CEL rule does.

    Pydantic has no hostname type, and our loop is stricter than any regex we
    could write here: it caps the name at 253 characters, allows a trailing dot,
    and rejects an all-digit right-most label.
    """
    if not _is_hostname(value):
        msg = "must be a valid hostname"
        raise ValueError(msg)
    return value


def _host_and_port(value: str) -> str:
    """Mirrors `string.host_and_port`, which is `isHostAndPort(true)` for us."""
    if not _is_host_and_port(value, port_required=True):
        msg = "must be a valid host (hostname or IP address) and port pair"
        raise ValueError(msg)
    return value


def _const(expected: int) -> Callable[[int], int]:
    """Mirrors the `const` rules when a sibling rule rules out `Literal`.

    `Literal[n]` is the native spelling, but pydantic silently ignores a
    `Field(gt=...)` annotated alongside a `Literal`, so a field carrying both
    `const` and a bound has to express the constant here instead.
    """

    def check(value: int) -> int:
        if value != expected:
            msg = f"must equal {expected}"
            raise ValueError(msg)
        return value

    return check


def _not_in(*disallowed: int) -> Callable[[int], int]:
    """Mirrors `int32.not_in`; pydantic has no negated-membership rule."""
    excluded = frozenset(disallowed)

    def check(value: int) -> int:
        if value in excluded:
            msg = f"must not be in list {sorted(excluded)}"
            raise ValueError(msg)
        return value

    return check


class ProtoModel(BaseModel):
    """Base for the pydantic mirrors, giving the benchmark a uniform entrypoint."""

    # `msg` is Any rather than Message so subclasses can narrow it to the message
    # type they mirror.
    @classmethod
    def to_payload(cls, msg: Any) -> dict[str, Any]:
        """Converts a proto message to pydantic input without validating it."""
        raise NotImplementedError

    @classmethod
    def from_proto(cls, msg: Any) -> ProtoModel:
        return cls.model_validate(cls.to_payload(msg))


class BenchScalar(ProtoModel):
    x: Annotated[int, Field(gt=0)]

    @classmethod
    def to_payload(cls, msg: bench_pb.BenchScalar) -> dict[str, Any]:
        return {"x": msg.x}


class BenchRepeatedScalar(ProtoModel):
    x: Annotated[list[int], Field(max_length=10)]

    @classmethod
    def to_payload(cls, msg: bench_pb.BenchRepeatedScalar) -> dict[str, Any]:
        return {"x": msg.x}


class BenchRepeatedMessage(ProtoModel):
    x: Annotated[list[BenchScalar], Field(max_length=10)]

    @classmethod
    def to_payload(cls, msg: bench_pb.BenchRepeatedMessage) -> dict[str, Any]:
        return {"x": [BenchScalar.to_payload(item) for item in msg.x]}


class BenchRepeatedScalarUnique(ProtoModel):
    x: _UniqueFloats

    @classmethod
    def to_payload(cls, msg: bench_pb.BenchRepeatedScalarUnique) -> dict[str, Any]:
        return {"x": msg.x}


class BenchRepeatedBytesUnique(ProtoModel):
    x: _UniqueBytes

    @classmethod
    def to_payload(cls, msg: bench_pb.BenchRepeatedBytesUnique) -> dict[str, Any]:
        return {"x": msg.x}


class BenchMap(ProtoModel):
    entries: Annotated[dict[str, str], Field(min_length=1)]

    @classmethod
    def to_payload(cls, msg: bench_pb.BenchMap) -> dict[str, Any]:
        return {"entries": msg.entries}


class BenchEnum(enum.IntEnum):
    UNSPECIFIED = 0
    ONE = 1
    TWO = 2


class BenchComplexSchema(ProtoModel):
    s1: Annotated[str, Field(min_length=1)]
    s2: Annotated[str, Field(max_length=100)]
    i32: Annotated[int, Field(gt=0)]
    i64: Annotated[int, Field(lt=1000)]
    u32: Annotated[int, Field(ge=1)]
    u64: Annotated[int, Field(le=1000)]
    si32: Annotated[int, Field(gt=0)]
    si64: Annotated[int, Field(lt=1000)]
    f32: Annotated[int, Field(ge=1)]
    f64: Annotated[int, Field(le=1000)]
    sf32: Annotated[int, Field(gt=0)]
    sf64: Annotated[int, Field(lt=1000)]
    # `float.finite`/`double.finite` are `!isNan() && !isInf()` for us.
    fl: Annotated[float, Field(allow_inf_nan=False)]
    db: Annotated[float, Field(allow_inf_nan=False)]
    bl: bool
    by: Annotated[bytes, Field(min_length=1)]

    nested: BenchScalar | None = None
    self_ref: BenchComplexSchema | None = None

    rep_str: Annotated[list[str], Field(max_length=10)]
    rep_i32: Annotated[list[int], Field(min_length=1)]
    rep_bytes: _UniqueBytes
    rep_msg: Annotated[list[BenchScalar], Field(max_length=5)]

    map_str_str: Annotated[dict[str, str], Field(min_length=1)]
    map_i32_i64: Annotated[dict[int, int], Field(max_length=10)]
    map_u64_bool: dict[int, bool]
    map_str_bytes: dict[Annotated[str, Field(min_length=1)], bytes]
    # `map.values.required` on a message value means "must be set", which a
    # non-optional model value already covers.
    map_str_msg: dict[str, BenchScalar]
    map_i64_msg: dict[int, BenchScalar]

    # `enum.defined_only` is what a Python enum type gives you for free. We have a
    # native (non-CEL) fast path for this rule too.
    enum_field: BenchEnum

    # proto oneofs have no pydantic equivalent; a union of the member types is the
    # idiomatic modelling and preserves the per-member rules.
    choice: (
        Annotated[str, Field(min_length=1)]
        | Annotated[int, Field(gt=0)]
        | BenchScalar
        | None
    ) = None

    @classmethod
    def to_payload(cls, msg: bench_pb.BenchComplexSchema) -> dict[str, Any]:
        oneof = msg.choice
        choice: str | int | dict[str, Any] | None = None
        if oneof is not None:
            choice = (
                BenchScalar.to_payload(oneof.value)
                if oneof.field == "oneof_msg"
                else oneof.value
            )
        return {
            "s1": msg.s1,
            "s2": msg.s2,
            "i32": msg.i32,
            "i64": msg.i64,
            "u32": msg.u32,
            "u64": msg.u64,
            "si32": msg.si32,
            "si64": msg.si64,
            "f32": msg.f32,
            "f64": msg.f64,
            "sf32": msg.sf32,
            "sf64": msg.sf64,
            "fl": msg.fl,
            "db": msg.db,
            "bl": msg.bl,
            "by": msg.by,
            "nested": (
                BenchScalar.to_payload(msg.nested) if msg.nested is not None else None
            ),
            "self_ref": (
                cls.to_payload(msg.self_ref) if msg.self_ref is not None else None
            ),
            "rep_str": msg.rep_str,
            "rep_i32": msg.rep_i32,
            "rep_bytes": msg.rep_bytes,
            "rep_msg": [BenchScalar.to_payload(item) for item in msg.rep_msg],
            "map_str_str": msg.map_str_str,
            "map_i32_i64": msg.map_i32_i64,
            "map_u64_bool": msg.map_u64_bool,
            "map_str_bytes": msg.map_str_bytes,
            "map_str_msg": {
                k: BenchScalar.to_payload(v) for k, v in msg.map_str_msg.items()
            },
            "map_i64_msg": {
                k: BenchScalar.to_payload(v) for k, v in msg.map_i64_msg.items()
            },
            # Left as a plain int so the enum lookup — the `defined_only` check —
            # happens inside the timed region.
            "enum_field": int(msg.enum_field),
            "choice": choice,
        }


# `gt` paired with a lower `lt`/`lte` is an *exclusive* range for us: valid below
# the upper bound or above the lower one. A union of two constrained ints is
# pydantic's way to spell that disjunction. It reports one error per branch where
# we report one per field, but accepts and rejects exactly the same values.
_GtLtExclusive = Annotated[int, Field(gt=0)] | Annotated[int, Field(lt=-20)]
_GtLteExclusive = Annotated[int, Field(gt=0)] | Annotated[int, Field(le=-20)]
_GteLtExclusive = Annotated[int, Field(ge=0)] | Annotated[int, Field(lt=-20)]
_GteLteExclusive = Annotated[int, Field(ge=0)] | Annotated[int, Field(le=-20)]


class BenchGT(ProtoModel):
    gt: Annotated[int, Field(gt=0)]
    gte: Annotated[int, Field(ge=0)]
    lt: Annotated[int, Field(lt=101)]
    lte: Annotated[int, Field(le=101)]
    gtltin: Annotated[int, Field(gt=0, lt=101)]
    gtltein: Annotated[int, Field(gt=0, lt=101)]
    gtltex: _GtLtExclusive
    gtlteex: _GtLteExclusive
    gteltin: Annotated[int, Field(ge=0, lt=101)]
    gteltein: Annotated[int, Field(ge=0, lt=101)]
    gteltex: _GteLtExclusive
    gtelteex: _GteLteExclusive
    const: Literal[10]
    constgt: Annotated[int, Field(ge=0), AfterValidator(_const(10))]
    in_test: Literal[1, 3, 5]
    not_in_test: Annotated[int, AfterValidator(_not_in(1, 3, 5))]

    @classmethod
    def to_payload(cls, msg: native_pb.BenchGT) -> dict[str, Any]:
        return {
            "gt": msg.gt,
            "gte": msg.gte,
            "lt": msg.lt,
            "lte": msg.lte,
            "gtltin": msg.gtltin,
            "gtltein": msg.gtltein,
            "gtltex": msg.gtltex,
            "gtlteex": msg.gtlteex,
            "gteltin": msg.gteltin,
            "gteltein": msg.gteltein,
            "gteltex": msg.gteltex,
            "gtelteex": msg.gtelteex,
            "const": msg.const,
            "constgt": msg.constgt,
            "in_test": msg.in_test,
            "not_in_test": msg.not_in_test,
        }


class TestByteMatching(ProtoModel):
    # Our `bytes.ip*` and `bytes.uuid` rules only check the buffer length — 4 or 16
    # bytes, with a companion `*_empty` rule rejecting zero — whereas pydantic
    # materialises a real address or UUID from the same packed bytes. Strictly more
    # work, and strictly better.
    ip_addr: IPvAnyAddress
    ipv4_addr: IPv4Address
    ipv6_addr: IPv6Address
    uuid: UUID

    @classmethod
    def to_payload(cls, msg: native_pb.TestByteMatching) -> dict[str, Any]:
        return {
            "ip_addr": msg.ip_addr,
            "ipv4_addr": msg.ipv4_addr,
            "ipv6_addr": msg.ipv6_addr,
            "uuid": msg.uuid,
        }


class StringMatching(ProtoModel):
    hostname: Annotated[str, AfterValidator(_hostname)]
    host_and_port: Annotated[str, AfterValidator(_host_and_port)]
    # Reuses our compiled pattern rather than a hand-written one. Ours has no
    # lookaround, so pydantic keeps its native Rust regex engine; that and re2
    # agree on this pattern, and the `^`/`$` anchors make `is_match` equivalent to
    # the `fullmatch` we do.
    email: Annotated[str, Field(pattern=EMAIL_PATTERN)]
    # `string.uuid` is a plain `[0-9a-fA-F]`-and-dashes regex for us. Pydantic's
    # UUID is a real parser, so it also accepts unhyphenated, braced and URN forms.
    uuid: UUID

    @classmethod
    def to_payload(cls, msg: native_pb.StringMatching) -> dict[str, Any]:
        return {
            "hostname": msg.hostname,
            "host_and_port": msg.host_and_port,
            "email": msg.email,
            "uuid": msg.uuid,
        }


class WrapperTesting(ProtoModel):
    # Wrapper types are just nullable scalars; rules apply only when set, which is
    # exactly what an optional pydantic field does.
    i32: Annotated[int, Field(gt=10)] | None = None
    d: Annotated[float, Field(gt=10)] | None = None
    f: Annotated[float, Field(gt=10)] | None = None
    i64: Annotated[int, Field(gt=10)] | None = None
    u64: Annotated[int, Field(gt=10)] | None = None
    u32: Annotated[int, Field(gt=10)] | None = None
    b: Literal[True] | None = None
    s: Literal["hello"] | None = None
    bs: Annotated[bytes, Field(min_length=5, max_length=5)] | None = None

    @classmethod
    def to_payload(cls, msg: native_pb.WrapperTesting) -> dict[str, Any]:
        return {
            "i32": msg.i32.value if msg.i32 is not None else None,
            "d": msg.d.value if msg.d is not None else None,
            "f": msg.f.value if msg.f is not None else None,
            "i64": msg.i64.value if msg.i64 is not None else None,
            "u64": msg.u64.value if msg.u64 is not None else None,
            "u32": msg.u32.value if msg.u32 is not None else None,
            "b": msg.b.value if msg.b is not None else None,
            "s": msg.s.value if msg.s is not None else None,
            "bs": msg.bs.value if msg.bs is not None else None,
        }


class MultiRule(ProtoModel):
    # Pydantic stops a field's validator chain at the first failure, so a value
    # breaking both rules reports one error here and two in protovalidate.
    many: Annotated[int, Field(gt=5), AfterValidator(_const(10))]

    @classmethod
    def to_payload(cls, msg: native_pb.MultiRule) -> dict[str, Any]:
        return {"many": msg.many}


_MODELS: dict[type[Any], type[ProtoModel]] = {
    bench_pb.BenchScalar: BenchScalar,
    bench_pb.BenchRepeatedScalar: BenchRepeatedScalar,
    bench_pb.BenchRepeatedMessage: BenchRepeatedMessage,
    bench_pb.BenchRepeatedScalarUnique: BenchRepeatedScalarUnique,
    bench_pb.BenchRepeatedBytesUnique: BenchRepeatedBytesUnique,
    bench_pb.BenchMap: BenchMap,
    bench_pb.BenchComplexSchema: BenchComplexSchema,
    native_pb.BenchGT: BenchGT,
    native_pb.TestByteMatching: TestByteMatching,
    native_pb.StringMatching: StringMatching,
    native_pb.WrapperTesting: WrapperTesting,
    native_pb.MultiRule: MultiRule,
}


def model_for(message: Any) -> type[ProtoModel] | None:
    """Returns the pydantic mirror of `message`'s type, or None if there isn't one."""
    return _MODELS.get(type(message))
