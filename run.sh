#!/usr/bin/env bash
# Inicia a mtklogo GUI. Cria um venv local (.venv) com PySide6 se o Qt não estiver instalado.
#   ./run.sh                    -> abre a GUI
#   ./run.sh --install-desktop  -> cria atalho no menu de aplicativos
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
Comment=Interface gráfica para o mtklogos.sh
Exec=bash "$HERE/run.sh"
Path=$HERE
Terminal=false
Categories=Utility;Development;
EOF
    echo "Atalho criado em ~/.local/share/applications/mtklogo-gui.desktop"
    exit 0
fi

have_qt() {
    "$1" - <<'PY' 2>/dev/null
import importlib.util, sys
sys.exit(0 if (importlib.util.find_spec("PySide6") or importlib.util.find_spec("PyQt5")) else 1)
PY
}

command -v python3 >/dev/null || { echo "python3 não encontrado."; exit 1; }

if have_qt python3; then
    PY=python3
else
    if [ ! -x .venv/bin/python ]; then
        echo ">> Criando ambiente virtual (.venv)…"
        python3 -m venv .venv || { echo "Instale o pacote python3-venv e tente de novo."; exit 1; }
    fi
    if ! have_qt .venv/bin/python; then
        echo ">> Instalando PySide6 (só na primeira vez)…"
        .venv/bin/pip install --quiet -r requirements.txt
    fi
    PY=.venv/bin/python
fi

exec "$PY" mtklogo_gui.py "$@"
