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
import subprocess
import sys
from pathlib import Path
from textwrap import dedent

import pytest

from test.versions import PROTOVALIDATE_VERSION


def maybe_patch_args_with_debug(args: list[str]) -> list[str]:
    # Do a best effort to invoke the child with debugging.
    # This invokes internal methods from bundles provided by the IDE
    # and may not always work.
    try:
        from pydevd import (  # ty: ignore[unresolved-import] - provided by IDE  # pyright: ignore[reportMissingImports] # noqa: PLC0415
            _pydev_bundle,
        )

        return _pydev_bundle.pydev_monkey.patch_args(args)
    except Exception:  # noqa: BLE001
        return args


# Both message flavours the validator accepts get conformance coverage: `py` is
# protobuf-py, `legacy` is google.protobuf.
@pytest.mark.parametrize("legacy", [False, True], ids=["py", "legacy"])
def test_conformance(*, legacy: bool) -> None:
    # Workaround pydevd monkeypatching of -m invocation not being compatible
    # with Python 3.14 yet by executing a script that uses runpy itself.
    # pydevd does monkeypatch -c form correctly.
    script = dedent(
        """
        import runpy
        runpy.run_module(
            'test.conformance.runner',
            run_name='__main__',
            alter_sys=True
        )
        """
    )
    command = [sys.executable, "--", "-c", script]
    command = maybe_patch_args_with_debug(command)

    env = os.environ.copy()
    if legacy:
        pytest.importorskip(
            "google.protobuf", reason="optional dependency not installed"
        )

        env["PROTOVALIDATE_CONFORMANCE_LEGACY"] = "1"

    subprocess.run(  # noqa: S603
        [  # noqa: S607
            "go",
            "run",
            f"github.com/bufbuild/protovalidate/tools/protovalidate-conformance@{PROTOVALIDATE_VERSION}",
            "--strict_message",
            f"--expected_failures={Path(__file__).parent / 'nonconforming.yaml'}",
            "--timeout",
            "10s",
            *command,
        ],
        env=env,
        check=True,
    )
