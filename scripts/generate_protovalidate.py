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

import re
import shutil
import subprocess
from pathlib import Path

from test.versions import PROTOVALIDATE_VERSION


def main() -> None:
    """Generate protovalidate and protovalidate-testing proto stubs."""
    if re.match(r"^v\d+\.\d+\.\d+(\-.+)?$", PROTOVALIDATE_VERSION):
        # Version tag, fetch from BSR
        protovalidate_path = f"buf.build/bufbuild/protovalidate:{PROTOVALIDATE_VERSION}"
        protovalidate_testing_path = (
            f"buf.build/bufbuild/protovalidate-testing:{PROTOVALIDATE_VERSION}"
        )
    else:
        # Not a tag, generally an unreleased commit, fetch directly from git
        protovalidate_path = f"https://github.com/bufbuild/protovalidate.git#subdir=proto/protovalidate,ref={PROTOVALIDATE_VERSION}"
        protovalidate_testing_path = f"https://github.com/bufbuild/protovalidate.git#subdir=proto/protovalidate-testing,ref={PROTOVALIDATE_VERSION}"

    repo = Path(__file__).parent.parent

    protos_dir = repo / "proto"
    shutil.rmtree(protos_dir, ignore_errors=True)
    protos_dir.mkdir(parents=True, exist_ok=True)
    subprocess.run(["buf", "export", protovalidate_path, "-o", protos_dir], check=True)
    subprocess.run(["buf", "generate"], cwd=repo, check=True)

    subprocess.run(
        ["buf", "export", protovalidate_testing_path, "-o", repo / "test" / "proto"],
        check=True,
    )
