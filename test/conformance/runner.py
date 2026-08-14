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

import os
import sys
from typing import TYPE_CHECKING

import protobuf
from protobuf import Oneof, Registry, wkt as pb_wkt

import protovalidate

from ..gen.buf.validate import validate_pb  # noqa: TID252
from ..gen.buf.validate.conformance.harness.harness_pb import (  # noqa: TID252
    TestConformanceRequest,
    TestConformanceResponse,
    TestResult,
)

if TYPE_CHECKING:
    from google.protobuf import (
        descriptor_pool as google_descriptor_pool,
        message as google_message,
    )

# Set to exercise google.protobuf messages instead of protobuf-py. The native
# validator accepts both, so both are worth conformance coverage.
_LEGACY = os.environ.get("PROTOVALIDATE_CONFORMANCE_LEGACY") == "1"


def build_google_pool(
    fdset: pb_wkt.FileDescriptorSet,
) -> google_descriptor_pool.DescriptorPool:
    from google.protobuf import (  # noqa: PLC0415
        descriptor_pb2 as google_descriptor_pb2,
        descriptor_pool as google_descriptor_pool,
    )

    pool = google_descriptor_pool.DescriptorPool()
    by_name = {file.name: file for file in fdset.file}
    added: set[str] = set()

    def add(name: str) -> None:
        proto = by_name.get(name)
        if proto is None or name in added:
            return
        added.add(name)
        for dep in proto.dependency:
            add(dep)
        pool.Add(
            google_descriptor_pb2.FileDescriptorProto.FromString(proto.to_binary())
        )

    for file in fdset.file:
        add(file.name)
    return pool


def run_test_case(
    validator: protovalidate.Validator, tc: protobuf.Message | google_message.Message
) -> TestResult:
    # Run the validator
    try:
        violations = validator.collect_violations(tc)
        if len(violations) > 0:
            # Convert from protovalidate bundled proto to test harness's.
            pv_violations = protovalidate.ViolationsPb(
                violations=[violation.proto for violation in violations]
            )
            return TestResult(
                result=Oneof(
                    field="validation_error",
                    value=validate_pb.Violations.from_binary(pv_violations.to_binary()),
                )
            )
        return TestResult(result=Oneof(field="success", value=True))
    except protovalidate.CompilationError as e:
        return TestResult(result=Oneof(field="compilation_error", value=str(e)))
    except RuntimeError as e:
        # EvaluationError subclasses RuntimeError, which is what puts rule
        # evaluation failures in the right conformance bucket.
        return TestResult(result=Oneof(field="runtime_error", value=str(e)))
    except Exception as e:  # noqa: BLE001
        return TestResult(result=Oneof(field="unexpected_error", value=str(e)))


def run_any_test_case(
    validator: protovalidate.Validator,
    registry: Registry | google_descriptor_pool.DescriptorPool,
    tc: pb_wkt.Any,
) -> TestResult:
    type_name = tc.type_url.split("/")[-1]
    msg: protobuf.Message | google_message.Message
    if isinstance(registry, Registry):
        desc = registry.message(type_name)
        if desc is None:
            return TestResult(
                result=Oneof(
                    field="unexpected_error", value=f"unknown type: {type_name}"
                )
            )
        unpacked = tc.unpack(desc)
        if unpacked is None:
            return TestResult(
                result=Oneof(
                    field="unexpected_error", value=f"cannot unpack {tc.type_url}"
                )
            )
        msg = unpacked
    else:
        from google.protobuf import message_factory as google_message_factory  # noqa: PLC0415, I001

        try:
            google_desc = registry.FindMessageTypeByName(type_name)
        except KeyError:
            return TestResult(
                result=Oneof(
                    field="unexpected_error", value=f"unknown type: {type_name}"
                )
            )
        msg = google_message_factory.GetMessageClass(google_desc)()
        msg.ParseFromString(tc.value)
    return run_test_case(validator, msg)


def run_conformance_test(request: TestConformanceRequest) -> TestConformanceResponse:
    registry = request.fdset.to_registry()
    # The registry resolves the conformance suite's custom predefined-rule extensions.
    validator = protovalidate.Validator(registry=registry)
    test_registry = registry if not _LEGACY else build_google_pool(request.fdset)
    response = TestConformanceResponse()
    for name, tc in request.cases.items():
        response.results[name] = run_any_test_case(validator, test_registry, tc)
    return response


if __name__ == "__main__":
    # Read a serialized TestConformanceRequest from stdin
    request = TestConformanceRequest.from_binary(sys.stdin.buffer.read())
    # Run the test
    result = run_conformance_test(request)
    # Write a serialized TestConformanceResponse to stdout
    sys.stdout.buffer.write(result.to_binary())
    sys.stdout.flush()
    sys.exit(0)
