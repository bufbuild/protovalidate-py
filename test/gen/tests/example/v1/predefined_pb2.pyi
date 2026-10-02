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
from google.protobuf import field_mask_pb2 as _field_mask_pb2
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Mapping as _Mapping, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor
ABS_NOT_IN_FIELD_NUMBER: _ClassVar[int]
abs_not_in: _descriptor.FieldDescriptor
THIS_EQUALS_RULE_FIELD_NUMBER: _ClassVar[int]
this_equals_rule: _descriptor.FieldDescriptor
FM_RULES_EXTENSION_FIELD_NUMBER: _ClassVar[int]
fm_rules_extension: _descriptor.FieldDescriptor

class Issue148(_message.Message):
    __slots__ = ("test",)
    TEST_FIELD_NUMBER: _ClassVar[int]
    test: int
    def __init__(self, test: _Optional[int] = ...) -> None: ...

class Issue187(_message.Message):
    __slots__ = ("false_field", "true_field")
    FALSE_FIELD_FIELD_NUMBER: _ClassVar[int]
    TRUE_FIELD_FIELD_NUMBER: _ClassVar[int]
    false_field: bool
    true_field: bool
    def __init__(self, false_field: bool = ..., true_field: bool = ...) -> None: ...

class Issue296(_message.Message):
    __slots__ = ("fm",)
    FM_FIELD_NUMBER: _ClassVar[int]
    fm: _field_mask_pb2.FieldMask
    def __init__(self, fm: _Optional[_Union[_field_mask_pb2.FieldMask, _Mapping]] = ...) -> None: ...
