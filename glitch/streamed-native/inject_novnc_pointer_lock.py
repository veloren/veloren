#!/usr/bin/env python3
from pathlib import Path
import json
import os
import shutil
import sys


MARKER = "GLITCH_NOVNC_POINTER_LOCK_MOUSE_V1"
SETTINGS_NAME = "__glitchNoVNCPointerLockSettingsV1"


def env_float(name, default):
    try:
        value = float(os.environ.get(name, default))
    except (TypeError, ValueError):
        value = default
    return value if value == value and value not in (float("inf"), float("-inf")) else default


def main():
    if len(sys.argv) != 4:
        raise SystemExit(
            "usage: inject_novnc_pointer_lock.py <novnc-html> <source-js> <target-js-name>"
        )

    html_path = Path(sys.argv[1])
    source_js = Path(sys.argv[2])
    target_js_name = sys.argv[3]
    target_js = html_path.parent / target_js_name

    text = html_path.read_text(errors="ignore")
    if MARKER in text:
        return

    if not source_js.is_file():
        raise SystemExit(f"missing pointer-lock source script: {source_js}")

    shutil.copyfile(source_js, target_js)

    settings = {
        "xScale": env_float("GLITCH_NOVNC_POINTER_LOCK_X_SCALE", 1.0),
        "yScale": env_float("GLITCH_NOVNC_POINTER_LOCK_Y_SCALE", 1.0),
        "maxDelta": env_float("GLITCH_NOVNC_POINTER_LOCK_MAX_DELTA", 48.0),
    }

    script = f"""
<script id="glitch-novnc-pointer-lock-settings">
/* {MARKER} */
window.{SETTINGS_NAME} = {json.dumps(settings, sort_keys=True)};
</script>
<script type="module" id="glitch-novnc-pointer-lock-mouse" src="./{target_js_name}"></script>
"""

    lower = text.lower()
    idx = lower.rfind("</body>")
    if idx >= 0:
        text = text[:idx] + script + "\n" + text[idx:]
    else:
        text = text + "\n" + script + "\n"

    html_path.write_text(text)


if __name__ == "__main__":
    main()
