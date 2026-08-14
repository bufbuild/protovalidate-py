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

import shutil
import subprocess
from pathlib import Path
from urllib.request import urlopen

from fix_protobuf_imports.fix_protobuf_imports import fix_protobuf_imports

from test.versions import CEL_SPEC_VERSION

test_dir = Path(__file__).parent.parent / "test"


def main() -> None:
    """Generates CEL conformance protos and fetches CEL testdata."""
    shutil.rmtree(test_dir / "gen" / "cel", ignore_errors=True)

    subprocess.run(
        [
            "buf",
            "generate",
            f"buf.build/google/cel-spec:{CEL_SPEC_VERSION}",
            "--exclude-path",
            "cel/expr/conformance/proto2",
            "--exclude-path",
            "cel/expr/conformance/proto3",
        ],
        check=True,
        cwd=test_dir,
    )

    with urlopen(
        f"https://raw.githubusercontent.com/google/cel-spec/refs/tags/{CEL_SPEC_VERSION}/tests/simple/testdata/string_ext.textproto"
    ) as res:
        out_path = test_dir / "testdata" / f"string_ext_{CEL_SPEC_VERSION}.textproto"
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_bytes(res.read())

    fix_protobuf_imports.callback(test_dir / "gen", dry=False)
