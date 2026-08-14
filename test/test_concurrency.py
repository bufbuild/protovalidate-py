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

import threading

from test.conftest import make_validator
from test.gen.bench.v1.bench_pb import BenchMap, BenchScalar
from test.gen.bench.v1.native_pb import MultiRule

THREADS = 8
ROUNDS = 50


def test_concurrent_first_use() -> None:
    """Shared-validator use from many threads, starting before registration.

    Regression test for a deadlock on GIL builds: registration used to walk
    Python descriptors while holding the validator's locks, so the interpreter
    could hand the GIL to another thread that then blocked on those locks with
    the GIL held, while the lock holder needed the GIL back to finish. The
    threads race first-time registration, validation, and lazy violation
    resolution; a deadlock shows up as threads that never finish.
    """
    validator = make_validator()
    messages = [BenchScalar(x=42), BenchMap(entries={"k": "v"}), MultiRule(many=1)]
    barrier = threading.Barrier(THREADS)
    errors: list[Exception] = []

    def run() -> None:
        try:
            barrier.wait(timeout=30)
            for _ in range(ROUNDS):
                for message in messages:
                    for violation in validator.collect_violations(message):
                        _ = violation.field_value
                        _ = violation.rule_value
        except Exception as e:  # noqa: BLE001 - re-raised via `errors` below
            errors.append(e)

    threads = [threading.Thread(target=run, daemon=True) for _ in range(THREADS)]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join(timeout=60)
    assert not errors
    assert all(not thread.is_alive() for thread in threads)
