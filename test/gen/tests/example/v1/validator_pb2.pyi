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
from ....buf.validate import validate_pb2 as _validate_pb2
from google.protobuf import any_pb2 as _any_pb2
from google.protobuf import api_pb2 as _api_pb2
from google.protobuf import field_mask_pb2 as _field_mask_pb2
from google.protobuf import timestamp_pb2 as _timestamp_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Iterable as _Iterable, Mapping as _Mapping, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class HasMsgExprs(_message.Message):
    __slots__ = ("x", "y")
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    x: int
    y: int
    def __init__(self, x: _Optional[int] = ..., y: _Optional[int] = ...) -> None: ...

class SelfRecursive(_message.Message):
    __slots__ = ("x", "turtle")
    X_FIELD_NUMBER: _ClassVar[int]
    TURTLE_FIELD_NUMBER: _ClassVar[int]
    x: int
    turtle: SelfRecursive
    def __init__(self, x: _Optional[int] = ..., turtle: _Optional[_Union[SelfRecursive, _Mapping]] = ...) -> None: ...

class LoopRecursiveA(_message.Message):
    __slots__ = ("b",)
    B_FIELD_NUMBER: _ClassVar[int]
    b: LoopRecursiveB
    def __init__(self, b: _Optional[_Union[LoopRecursiveB, _Mapping]] = ...) -> None: ...

class LoopRecursiveB(_message.Message):
    __slots__ = ("a",)
    A_FIELD_NUMBER: _ClassVar[int]
    a: LoopRecursiveA
    def __init__(self, a: _Optional[_Union[LoopRecursiveA, _Mapping]] = ...) -> None: ...

class MsgHasOneof(_message.Message):
    __slots__ = ("x", "y", "msg")
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    MSG_FIELD_NUMBER: _ClassVar[int]
    x: str
    y: int
    msg: HasMsgExprs
    def __init__(self, x: _Optional[str] = ..., y: _Optional[int] = ..., msg: _Optional[_Union[HasMsgExprs, _Mapping]] = ...) -> None: ...

class MsgHasRepeated(_message.Message):
    __slots__ = ("x", "y", "z")
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    Z_FIELD_NUMBER: _ClassVar[int]
    x: _containers.RepeatedScalarFieldContainer[float]
    y: _containers.RepeatedScalarFieldContainer[str]
    z: _containers.RepeatedCompositeFieldContainer[HasMsgExprs]
    def __init__(self, x: _Optional[_Iterable[float]] = ..., y: _Optional[_Iterable[str]] = ..., z: _Optional[_Iterable[_Union[HasMsgExprs, _Mapping]]] = ...) -> None: ...

class MsgHasMap(_message.Message):
    __slots__ = ("int32map", "string_map", "message_map")
    class Int32mapEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: int
        value: int
        def __init__(self, key: _Optional[int] = ..., value: _Optional[int] = ...) -> None: ...
    class StringMapEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: str
        def __init__(self, key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...
    class MessageMapEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: int
        value: LoopRecursiveA
        def __init__(self, key: _Optional[int] = ..., value: _Optional[_Union[LoopRecursiveA, _Mapping]] = ...) -> None: ...
    INT32MAP_FIELD_NUMBER: _ClassVar[int]
    STRING_MAP_FIELD_NUMBER: _ClassVar[int]
    MESSAGE_MAP_FIELD_NUMBER: _ClassVar[int]
    int32map: _containers.ScalarMap[int, int]
    string_map: _containers.ScalarMap[str, str]
    message_map: _containers.MessageMap[int, LoopRecursiveA]
    def __init__(self, int32map: _Optional[_Mapping[int, int]] = ..., string_map: _Optional[_Mapping[str, str]] = ..., message_map: _Optional[_Mapping[int, LoopRecursiveA]] = ...) -> None: ...

class TransitiveFieldRule(_message.Message):
    __slots__ = ("mask",)
    MASK_FIELD_NUMBER: _ClassVar[int]
    mask: _field_mask_pb2.FieldMask
    def __init__(self, mask: _Optional[_Union[_field_mask_pb2.FieldMask, _Mapping]] = ...) -> None: ...

class MultipleStepsTransitiveFieldRules(_message.Message):
    __slots__ = ("api",)
    API_FIELD_NUMBER: _ClassVar[int]
    api: _api_pb2.Api
    def __init__(self, api: _Optional[_Union[_api_pb2.Api, _Mapping]] = ...) -> None: ...

class Simple(_message.Message):
    __slots__ = ("s",)
    S_FIELD_NUMBER: _ClassVar[int]
    s: str
    def __init__(self, s: _Optional[str] = ...) -> None: ...

class FieldOfTypeAny(_message.Message):
    __slots__ = ("any",)
    ANY_FIELD_NUMBER: _ClassVar[int]
    any: _any_pb2.Any
    def __init__(self, any: _Optional[_Union[_any_pb2.Any, _Mapping]] = ...) -> None: ...

class CelMapOnARepeated(_message.Message):
    __slots__ = ("values",)
    class Value(_message.Message):
        __slots__ = ("name",)
        NAME_FIELD_NUMBER: _ClassVar[int]
        name: str
        def __init__(self, name: _Optional[str] = ...) -> None: ...
    VALUES_FIELD_NUMBER: _ClassVar[int]
    values: _containers.RepeatedCompositeFieldContainer[CelMapOnARepeated.Value]
    def __init__(self, values: _Optional[_Iterable[_Union[CelMapOnARepeated.Value, _Mapping]]] = ...) -> None: ...

class RepeatedItemCel(_message.Message):
    __slots__ = ("paths",)
    PATHS_FIELD_NUMBER: _ClassVar[int]
    paths: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, paths: _Optional[_Iterable[str]] = ...) -> None: ...

class OneTwo(_message.Message):
    __slots__ = ("field1", "field2")
    FIELD1_FIELD_NUMBER: _ClassVar[int]
    FIELD2_FIELD_NUMBER: _ClassVar[int]
    field1: F1
    field2: F2
    def __init__(self, field1: _Optional[_Union[F1, _Mapping]] = ..., field2: _Optional[_Union[F2, _Mapping]] = ...) -> None: ...

class TwoOne(_message.Message):
    __slots__ = ("field2", "field1")
    FIELD2_FIELD_NUMBER: _ClassVar[int]
    FIELD1_FIELD_NUMBER: _ClassVar[int]
    field2: F2
    field1: F1
    def __init__(self, field2: _Optional[_Union[F2, _Mapping]] = ..., field1: _Optional[_Union[F1, _Mapping]] = ...) -> None: ...

class F1(_message.Message):
    __slots__ = ("need_this", "field")
    NEED_THIS_FIELD_NUMBER: _ClassVar[int]
    FIELD_FIELD_NUMBER: _ClassVar[int]
    need_this: str
    field: FieldWithIssue
    def __init__(self, need_this: _Optional[str] = ..., field: _Optional[_Union[FieldWithIssue, _Mapping]] = ...) -> None: ...

class F2(_message.Message):
    __slots__ = ("field",)
    FIELD_FIELD_NUMBER: _ClassVar[int]
    field: FieldWithIssue
    def __init__(self, field: _Optional[_Union[FieldWithIssue, _Mapping]] = ...) -> None: ...

class FieldWithIssue(_message.Message):
    __slots__ = ("f1", "name")
    F1_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    f1: F1
    name: str
    def __init__(self, f1: _Optional[_Union[F1, _Mapping]] = ..., name: _Optional[str] = ...) -> None: ...

class Issue211(_message.Message):
    __slots__ = ("value",)
    VALUE_FIELD_NUMBER: _ClassVar[int]
    value: _timestamp_pb2.Timestamp
    def __init__(self, value: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ...) -> None: ...

class SelfReferentialRepeated(_message.Message):
    __slots__ = ("children", "name")
    CHILDREN_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    children: _containers.RepeatedCompositeFieldContainer[SelfReferentialRepeated]
    name: str
    def __init__(self, children: _Optional[_Iterable[_Union[SelfReferentialRepeated, _Mapping]]] = ..., name: _Optional[str] = ...) -> None: ...

class SelfReferentialMap(_message.Message):
    __slots__ = ("children", "name")
    class ChildrenEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: SelfReferentialMap
        def __init__(self, key: _Optional[str] = ..., value: _Optional[_Union[SelfReferentialMap, _Mapping]] = ...) -> None: ...
    CHILDREN_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    children: _containers.MessageMap[str, SelfReferentialMap]
    name: str
    def __init__(self, children: _Optional[_Mapping[str, SelfReferentialMap]] = ..., name: _Optional[str] = ...) -> None: ...
