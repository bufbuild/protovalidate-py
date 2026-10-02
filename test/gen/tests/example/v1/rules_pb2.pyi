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
from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Iterable as _Iterable, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class TestEnum(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TEST_ENUM_UNSPECIFIED: _ClassVar[TestEnum]
    TEST_ENUM_VAL1: _ClassVar[TestEnum]
    TEST_ENUM_VAL2: _ClassVar[TestEnum]
    TEST_ENUM_VAL3: _ClassVar[TestEnum]
TEST_ENUM_UNSPECIFIED: TestEnum
TEST_ENUM_VAL1: TestEnum
TEST_ENUM_VAL2: TestEnum
TEST_ENUM_VAL3: TestEnum

class BenchTestBytes(_message.Message):
    __slots__ = ("b1", "b")
    B1_FIELD_NUMBER: _ClassVar[int]
    B_FIELD_NUMBER: _ClassVar[int]
    b1: bytes
    b: bytes
    def __init__(self, b1: _Optional[bytes] = ..., b: _Optional[bytes] = ...) -> None: ...

class TestUnique(_message.Message):
    __slots__ = ("enums", "bytes", "strings")
    ENUMS_FIELD_NUMBER: _ClassVar[int]
    BYTES_FIELD_NUMBER: _ClassVar[int]
    STRINGS_FIELD_NUMBER: _ClassVar[int]
    enums: _containers.RepeatedScalarFieldContainer[TestEnum]
    bytes: _containers.RepeatedScalarFieldContainer[bytes]
    strings: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, enums: _Optional[_Iterable[_Union[TestEnum, str]]] = ..., bytes: _Optional[_Iterable[bytes]] = ..., strings: _Optional[_Iterable[str]] = ...) -> None: ...

class TestByteBroken(_message.Message):
    __slots__ = ("broken",)
    BROKEN_FIELD_NUMBER: _ClassVar[int]
    broken: bytes
    def __init__(self, broken: _Optional[bytes] = ...) -> None: ...

class MultiRule(_message.Message):
    __slots__ = ("many",)
    MANY_FIELD_NUMBER: _ClassVar[int]
    many: int
    def __init__(self, many: _Optional[int] = ...) -> None: ...
