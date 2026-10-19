#!/usr/bin/env bash
# Starts the mtklogo GUI. Creates a local venv (.venv) with PySide6 if Qt is not installed.
#   ./run.sh                    -> Opens the GUI
#   ./run.sh --install-desktop  -> Creates a shortcut in the applications menu
set -e
cd "$(dirname "$(readlink -f "$0")")"
HERE="$PWD"

chmod u+x mtklogo mtklogos.sh run.sh mtklogo_gui.py 2>/dev/null || true

if [ "$1" = "--install-desktop" ]; then
    mkdir -p "$HOME/.local/share/applications"
    cat > "$HOME/.local/share/applications/mtklogo-gui.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=mtklogo GUI
Comment=Graphical interface for mtklogos.sh
Exec=bash "$HERE/run.sh"
Path=$HERE
Terminal=false
Categories=Utility;Development;
EOF
    echo "Shortcut created in ~/.local/share/applications/mtklogo-gui.desktop"
    exit 0
fi

have_qt() {
    "$1" - <<'PY' 2>/dev/null
import importlib.util, sys
sys.exit(0 if (importlib.util.find_spec("PySide6") or importlib.util.find_spec("PyQt5")) else 1)
PY
}

command -v python3 >/dev/null || { echo "python3 not found."; exit 1; }

if have_qt python3; then
    PY=python3
else
    if [ ! -x .venv/bin/python ]; then
        echo ">> Creating a virtual environment (.venv)…"
        python3 -m venv .venv || { echo "Install the python3-venv package and try again."; exit 1; }
    fi
    if ! have_qt .venv/bin/python; then
        echo ">> Installing PySide6 (first time only)..."
        .venv/bin/pip install --quiet -r requirements.txt
    fi
    PY=.venv/bin/python
fi

exec "$PY" mtklogo_gui.py "$@"
