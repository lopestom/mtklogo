#!/usr/bin/env python3
"""
mtklogo GUI - interface gráfica minimalista (Qt) para o script `mtklogos.sh`.

O script é executado EXATAMENTE como está (`bash ./mtklogos.sh`), sem nenhuma
modificação nos seus processos. A GUI apenas:

  * captura stdout+stderr em tempo real (com cores ANSI);
  * detecta quando o script está parado esperando entrada (via /proc, lendo se
    algum processo da árvore está bloqueado em read(0, ...));
  * mostra o diálogo adequado:
        - menu `select` do bash  -> lista de opções
        - `read` com texto       -> campo de texto
        - [s/N], (y/n), yes/no   -> Sim / Não
        - "Pressione Enter"      -> botão Continuar
  * envia a resposta ao stdin do script.

Única alteração de ambiente: COLUMNS=1, para que o `select` do bash imprima uma
opção por linha (facilita a leitura do menu). Não muda a lógica do script.

Funciona com PySide6 (preferido) ou PyQt5.
"""
from __future__ import annotations

import codecs
import os
import platform
import re
import signal
import subprocess
import sys
import time
from pathlib import Path

try:
    from PySide6 import QtCore, QtGui, QtWidgets
    Signal = QtCore.Signal
    QT_API = "PySide6"
except ImportError:  # pragma: no cover
    try:
        from PyQt5 import QtCore, QtGui, QtWidgets
        Signal = QtCore.pyqtSignal
        QT_API = "PyQt5"
    except ImportError:
        sys.exit("Qt não encontrado. Instale com:  pip install PySide6   (ou PyQt5)")

SCRIPT_NAME = "mtklogos.sh"
PS3 = "#? "              # prompt padrão do `select` do bash
IDLE_SECONDS = 0.15      # silêncio mínimo na saída antes de considerar um prompt

# --------------------------------------------------------------------------- #
#  Inspeção de processos (Linux): quem está bloqueado lendo o stdin?
# --------------------------------------------------------------------------- #
_READ_SYSCALL = {"x86_64": 0, "aarch64": 63, "arm64": 63, "riscv64": 63,
                 "i386": 3, "i686": 3, "armv7l": 3, "armv8l": 3}


def _children_map() -> dict[int, list[int]]:
    tree: dict[int, list[int]] = {}
    for entry in os.listdir("/proc"):
        if not entry.isdigit():
            continue
        try:
            with open(f"/proc/{entry}/stat") as fh:
                stat = fh.read()
            ppid = int(stat.rsplit(")", 1)[1].split()[1])
        except (OSError, ValueError, IndexError):
            continue
        tree.setdefault(ppid, []).append(int(entry))
    return tree


def process_tree(root: int) -> list[int]:
    tree, out, stack = _children_map(), [root], [root]
    while stack:
        for child in tree.get(stack.pop(), []):
            out.append(child)
            stack.append(child)
    return out


def stdin_waiters(root: int):
    """PIDs (do script e descendentes) bloqueados em read(fd 0).

    Retorna None se não for possível inspecionar (outra arquitetura, /proc
    restrito...). Nesse caso a GUI usa uma heurística baseada em tempo.
    """
    nr = _READ_SYSCALL.get(platform.machine())
    if nr is None:
        return None
    readable, waiting = False, []
    for pid in process_tree(root):
        try:
            with open(f"/proc/{pid}/syscall") as fh:
                parts = fh.read().split()
        except OSError:
            continue
        readable = True
        if len(parts) >= 2 and parts[0] == str(nr) and parts[1] == "0x0":
            waiting.append(pid)
    return waiting if readable else None


# --------------------------------------------------------------------------- #
#  Log com suporte a cores ANSI e \r
# --------------------------------------------------------------------------- #
_CSI = re.compile(r"\x1b\[([0-?]*)[ -/]*([@-~])")
_PARTIAL_ESC = re.compile(r"\x1b(\[[0-?]*[ -/]*)?\Z")
_OTHER_ESC = re.compile(r"\x1b[()][0-9A-Za-z]|\x1b[=>78]")
_SPLIT = re.compile(r"(\r\n|\n|\r)")

_ANSI_COLORS = {
    30: "#6b7280", 31: "#ff6b6b", 32: "#7ee787", 33: "#e3b341",
    34: "#79a8ff", 35: "#d2a8ff", 36: "#56d4dd", 37: "#d8dee9",
    90: "#9aa4b2", 91: "#ff8f8f", 92: "#9bf2a3", 93: "#f2cc60",
    94: "#a5c4ff", 95: "#e2c5ff", 96: "#7fe6ee", 97: "#ffffff",
}
NOTE_COLOR = "#f0a35e"


class AnsiLog(QtWidgets.QPlainTextEdit):
    """Área de log somente-leitura. Mantém `tail` (linha atual sem ANSI)."""

    def __init__(self, on_line=None):
        super().__init__()
        self.on_line = on_line
        self.setReadOnly(True)
        self.setMaximumBlockCount(50000)
        self.setLineWrapMode(QtWidgets.QPlainTextEdit.NoWrap)
        self.setFont(QtGui.QFontDatabase.systemFont(QtGui.QFontDatabase.FixedFont))
        self.setStyleSheet("QPlainTextEdit { background:#10141a; color:#d8dee9; }")
        self.reset_state()

    def reset_state(self):
        self.tail = ""
        self._cr = False
        self._carry = ""
        self._fg = None
        self._bold = False

    # -- formatação ---------------------------------------------------------
    def _fmt(self, color=None, bold=None):
        fmt = QtGui.QTextCharFormat()
        color = color or self._fg
        if color:
            fmt.setForeground(QtGui.QColor(color))
        if self._bold if bold is None else bold:
            fmt.setFontWeight(QtGui.QFont.Bold)
        return fmt

    def _sgr(self, params: str):
        codes = [int(p) if p.isdigit() else 0 for p in params.split(";")] or [0]
        i = 0
        while i < len(codes):
            c = codes[i]
            if c == 0:
                self._fg, self._bold = None, False
            elif c == 1:
                self._bold = True
            elif c == 22:
                self._bold = False
            elif c in _ANSI_COLORS:
                self._fg = _ANSI_COLORS[c]
            elif c == 39:
                self._fg = None
            elif c in (38, 48):          # cor estendida: ignora os argumentos
                i += 2 if i + 1 < len(codes) and codes[i + 1] == 5 else 4
            i += 1

    # -- escrita ------------------------------------------------------------
    def _insert(self, text: str, fmt=None):
        cur = self.textCursor()
        cur.movePosition(QtGui.QTextCursor.End)
        cur.insertText(text, fmt or self._fmt())

    def _newline(self):
        line = self.tail
        self._insert("\n", QtGui.QTextCharFormat())
        self.tail, self._cr = "", False
        if self.on_line:
            self.on_line(line)

    def _text(self, text: str):
        if self._cr:                      # '\r' isolado: sobrescreve a linha
            cur = self.textCursor()
            cur.movePosition(QtGui.QTextCursor.End)
            cur.movePosition(QtGui.QTextCursor.StartOfBlock, QtGui.QTextCursor.KeepAnchor)
            cur.removeSelectedText()
            self.tail, self._cr = "", False
        self._insert(text)
        self.tail += text

    def _emit(self, segment: str):
        for part in _SPLIT.split(segment):
            if part in ("\n", "\r\n"):
                self._newline()
            elif part == "\r":
                self._cr = True
            elif part:
                self._text(part)

    def feed(self, text: str):
        sb = self.verticalScrollBar()
        at_bottom = sb.value() >= sb.maximum() - 4
        text, self._carry = self._carry + text, ""
        m = _PARTIAL_ESC.search(text)
        if m:                              # sequência ANSI cortada no meio
            self._carry, text = text[m.start():], text[:m.start()]
        text = _OTHER_ESC.sub("", text)
        pos = 0
        for m in _CSI.finditer(text):
            self._emit(text[pos:m.start()])
            if m.group(2) == "m":
                self._sgr(m.group(1))
            pos = m.end()
        self._emit(text[pos:])
        if at_bottom:
            sb.setValue(sb.maximum())

    def note(self, text: str):
        """Escreve uma anotação (ex.: resposta enviada) e quebra a linha."""
        self._insert(text + "\n", self._fmt(NOTE_COLOR, True))
        self.tail, self._cr = "", False
        sb = self.verticalScrollBar()
        sb.setValue(sb.maximum())


# --------------------------------------------------------------------------- #
#  Leitura do processo em thread
# --------------------------------------------------------------------------- #
class Reader(QtCore.QThread):
    chunk = Signal(bytes)
    done = Signal(int)

    def __init__(self, proc: subprocess.Popen, parent=None):
        super().__init__(parent)
        self.proc = proc

    def run(self):
        fd = self.proc.stdout.fileno()
        while True:
            try:
                data = os.read(fd, 65536)
            except OSError:
                break
            if not data:
                break
            self.chunk.emit(data)
        self.done.emit(self.proc.wait())


# --------------------------------------------------------------------------- #
#  Detecção de menus / perguntas
# --------------------------------------------------------------------------- #
_MENU_START = re.compile(r"^\s*\d+\) ")
_MENU_ITEM = re.compile(r"(\d+)\) (.*?)(?=\s{2,}\d+\) |\s*$)")
_YES_NO = re.compile(
    r"[\[(]\s*(y(?:es)?|s(?:im)?)\s*/\s*(n(?:o|ão|ao)?)\s*[\])]", re.IGNORECASE)
_ENTER = re.compile(r"(press|hit|pressione|tecle|aperte)\W+(the\s+)?(enter|return)|any key|qualquer tecla",
                    re.IGNORECASE)


def parse_menu_line(line: str):
    if not _MENU_START.match(line):
        return []
    return [(int(n), label.strip()) for n, label in _MENU_ITEM.findall(line)]


# --------------------------------------------------------------------------- #
#  Janela principal
# --------------------------------------------------------------------------- #
class MainWindow(QtWidgets.QMainWindow):
    def __init__(self, workdir: Path):
        super().__init__()
        self.setWindowTitle("mtklogo GUI")
        self.resize(900, 620)

        self.workdir = workdir
        self.proc = None
        self.reader = None
        self._running = False
        self._dialog_open = False
        self._reset_run_state()

        # --- widgets
        self.dir_edit = QtWidgets.QLineEdit(str(workdir))
        self.dir_edit.setReadOnly(True)
        self.btn_dir = QtWidgets.QPushButton("Pasta…")
        self.btn_open = QtWidgets.QPushButton("Abrir")
        self.log = AnsiLog(on_line=self._on_line)
        self.btn_run = QtWidgets.QPushButton("▶  Executar mtklogos.sh")
        self.btn_stop = QtWidgets.QPushButton("■  Parar")
        self.btn_clear = QtWidgets.QPushButton("Limpar")
        self.input = QtWidgets.QLineEdit()
        self.input.setPlaceholderText("Entrada manual para o script (Enter envia)")
        self.btn_send = QtWidgets.QPushButton("Enviar")

        top = QtWidgets.QHBoxLayout()
        top.addWidget(QtWidgets.QLabel("Pasta de trabalho:"))
        top.addWidget(self.dir_edit, 1)
        top.addWidget(self.btn_dir)
        top.addWidget(self.btn_open)

        bottom = QtWidgets.QHBoxLayout()
        bottom.addWidget(self.btn_run)
        bottom.addWidget(self.btn_stop)
        bottom.addWidget(self.btn_clear)
        bottom.addSpacing(16)
        bottom.addWidget(self.input, 1)
        bottom.addWidget(self.btn_send)

        root = QtWidgets.QVBoxLayout()
        root.addLayout(top)
        root.addWidget(self.log, 1)
        root.addLayout(bottom)
        central = QtWidgets.QWidget()
        central.setLayout(root)
        self.setCentralWidget(central)
        self.statusBar().showMessage("Pronto.")

        # --- sinais
        self.btn_run.clicked.connect(self.start)
        self.btn_stop.clicked.connect(self.stop)
        self.btn_clear.clicked.connect(self.log.clear)
        self.btn_dir.clicked.connect(self.choose_dir)
        self.btn_open.clicked.connect(self.open_dir)
        self.btn_send.clicked.connect(self._manual_send)
        self.input.returnPressed.connect(self._manual_send)

        self.timer = QtCore.QTimer(self)
        self.timer.setInterval(100)
        self.timer.timeout.connect(self._poll)

        self._update_buttons()

    # ------------------------------------------------------------------ estado
    def _reset_run_state(self):
        self._menu_buf: list = []
        self._pending_menu = None
        self._menus: dict = {}
        self._menu_stack: list = []
        self._last_line = ""
        self._last_data = time.monotonic()
        self._need_new_data = False
        self._sent_at = 0.0
        self._decoder = codecs.getincrementaldecoder("utf-8")(errors="replace")

    def _update_buttons(self):
        run = self._running
        self.btn_run.setEnabled(not run)
        self.btn_stop.setEnabled(run)
        self.btn_dir.setEnabled(not run)
        self.btn_send.setEnabled(run)
        self.input.setEnabled(run)

    def _status(self, msg: str):
        self.statusBar().showMessage(msg)

    # ------------------------------------------------------------ pasta / ações
    def choose_dir(self):
        d = QtWidgets.QFileDialog.getExistingDirectory(
            self, "Pasta com mtklogos.sh, mtklogo e logo.bin", str(self.workdir))
        if d:
            self.workdir = Path(d)
            self.dir_edit.setText(d)

    def open_dir(self):
        QtGui.QDesktopServices.openUrl(QtCore.QUrl.fromLocalFile(str(self.workdir)))

    # -------------------------------------------------------------- executar
    def start(self):
        if self._running:
            return
        script = self.workdir / SCRIPT_NAME
        if not script.is_file():
            QtWidgets.QMessageBox.critical(
                self, "Script não encontrado",
                f"{SCRIPT_NAME} não existe em:\n{self.workdir}\n\n"
                "Use “Pasta…” para escolher a pasta correta.")
            return
        self.log.clear()
        self.log.reset_state()
        self._reset_run_state()
        env = os.environ.copy()
        env["COLUMNS"] = "1"          # `select` imprime uma opção por linha
        try:
            self.proc = subprocess.Popen(
                ["bash", f"./{SCRIPT_NAME}"], cwd=str(self.workdir), env=env,
                stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT, bufsize=0, start_new_session=True)
        except OSError as exc:
            QtWidgets.QMessageBox.critical(self, "Erro ao iniciar", str(exc))
            return
        self._running = True
        self.reader = Reader(self.proc, self)
        self.reader.chunk.connect(self._on_chunk)
        self.reader.done.connect(self._on_done)
        self.reader.start()
        self.timer.start()
        self._update_buttons()
        self._status(f"Executando {SCRIPT_NAME} (PID {self.proc.pid})…")

    def stop(self):
        if not self._running or self.proc is None:
            return
        self._signal_group(signal.SIGTERM)
        QtCore.QTimer.singleShot(2000, lambda: self._signal_group(signal.SIGKILL))
        self._status("Encerrando o script…")

    def _signal_group(self, sig):
        if self.proc is not None and self.proc.poll() is None:
            try:
                os.killpg(self.proc.pid, sig)
            except (ProcessLookupError, PermissionError):
                pass

    # ------------------------------------------------------------ saída em tempo real
    def _on_chunk(self, data: bytes):
        self._last_data = time.monotonic()
        self._need_new_data = False
        self.log.feed(self._decoder.decode(data))

    def _on_done(self, code: int):
        tail = self._decoder.decode(b"", final=True)
        if tail:
            self.log.feed(tail)
        self.timer.stop()
        self._running = False
        self._menu_buf = []
        if self.log.tail:
            self.log.feed("\n")
        self.log.note(f"— Script finalizado (código de saída {code}) —")
        self._status(f"Finalizado (código {code}).")
        self._update_buttons()

    def _on_line(self, line: str):
        items = parse_menu_line(line)
        if items:
            self._menu_buf.extend(items)
            return
        self._finalize_menu()
        if line.strip():
            self._last_line = line.strip()

    def _finalize_menu(self):
        if self._menu_buf:
            self._pending_menu = sorted(dict(self._menu_buf).items())
            self._menu_buf = []

    # --------------------------------------------------------- detectar prompts
    def _poll(self):
        if not self._running or self._dialog_open or self.proc is None:
            return
        now = time.monotonic()
        if now - self._last_data < IDLE_SECONDS:
            return
        waiters = stdin_waiters(self.proc.pid)
        if self._need_new_data:
            # resposta enviada; só pergunta de novo se o script voltou a esperar
            if waiters and now - self._sent_at > 0.6:
                self._need_new_data = False
            else:
                return
        tail = self.log.tail
        if waiters is None:                       # sem /proc: heurística por tempo
            if not tail.strip() or now - self._last_data < 1.0:
                return
        elif not waiters:                         # algo ainda está trabalhando
            return
        self._handle_prompt(tail, waiters[-1] if waiters else None)

    def _handle_prompt(self, tail: str, pid):
        self._finalize_menu()
        is_select = tail.endswith(PS3)
        menu = None
        if is_select:
            key = pid if pid is not None else "?"
            if self._pending_menu:
                self._menus[key] = self._pending_menu
                self._menu_stack.append(self._pending_menu)
                self._pending_menu = None
            menu = self._menus.get(key)
            if menu is None and pid is None and self._menu_stack:
                menu = self._menu_stack[-1]
        self._dialog_open = True
        self._status("Aguardando sua resposta…")
        try:
            answer = self._ask_until_answered(tail, is_select, menu)
        finally:
            self._dialog_open = False
        if answer is None:
            self.stop()
            return
        if self._running:
            self._send(*answer)
            self._status("Executando…")

    def _ask_until_answered(self, tail, is_select, menu):
        """Retorna (texto_enviado, texto_exibido) ou None se o usuário cancelar."""
        while True:
            plain = tail.strip()
            if is_select and menu:
                res = self.ask_choice(menu)
                answer = (str(res[0]), f"{res[0]}  ({res[1]})") if res else None
            elif is_select:
                text = self.ask_text("O script pede o número de uma opção:")
                answer = (text, text) if text is not None else None
            elif (m := _YES_NO.search(plain)):
                yes, no = m.group(1).lower(), m.group(2).lower()
                r = self.ask_yes_no(plain)
                answer = ((yes, yes) if r else (no, no)) if r is not None else None
            elif _ENTER.search(plain):
                answer = ("", "⏎") if self.ask_enter(plain) else None
            else:
                label = plain or self._last_line or "O script está aguardando uma entrada:"
                text = self.ask_text(label)
                answer = (text, text) if text is not None else None
            if answer is not None:
                return answer
            r = QtWidgets.QMessageBox.question(
                self, "Cancelar?",
                "Cancelar encerra o script.\nDeseja realmente parar a execução?")
            if r == QtWidgets.QMessageBox.Yes:
                return None

    # ---------------------------------------------------------------- diálogos
    def ask_choice(self, menu):
        dlg = QtWidgets.QDialog(self)
        dlg.setWindowTitle("O script pede uma escolha")
        lst = QtWidgets.QListWidget()
        for num, label in menu:
            item = QtWidgets.QListWidgetItem(f"{num})  {label}")
            item.setData(QtCore.Qt.UserRole, (num, label))
            lst.addItem(item)
        lst.setCurrentRow(0)
        box = QtWidgets.QDialogButtonBox(
            QtWidgets.QDialogButtonBox.Ok | QtWidgets.QDialogButtonBox.Cancel)
        box.accepted.connect(dlg.accept)
        box.rejected.connect(dlg.reject)
        lst.itemActivated.connect(lambda _i: dlg.accept())
        lay = QtWidgets.QVBoxLayout(dlg)
        lay.addWidget(QtWidgets.QLabel("Selecione uma opção:"))
        lay.addWidget(lst)
        lay.addWidget(box)
        dlg.resize(360, min(150 + 26 * len(menu), 560))
        if dlg.exec() and lst.currentItem():
            return lst.currentItem().data(QtCore.Qt.UserRole)
        return None

    def ask_text(self, label: str):
        text, ok = QtWidgets.QInputDialog.getText(
            self, "O script pede um valor", label.strip() or "Entrada:")
        return text.strip() if ok else None

    def ask_yes_no(self, question: str):
        r = QtWidgets.QMessageBox.question(
            self, "Confirmação do script", question,
            QtWidgets.QMessageBox.Yes | QtWidgets.QMessageBox.No,
            QtWidgets.QMessageBox.No)
        return r == QtWidgets.QMessageBox.Yes

    def ask_enter(self, text: str):
        box = QtWidgets.QMessageBox(
            QtWidgets.QMessageBox.Information, "O script aguarda", text,
            QtWidgets.QMessageBox.Ok | QtWidgets.QMessageBox.Cancel, self)
        box.button(QtWidgets.QMessageBox.Ok).setText("Continuar")
        return box.exec() == QtWidgets.QMessageBox.Ok

    # ------------------------------------------------------------------- envio
    def _send(self, text: str, shown: str | None = None):
        if not self._running or self.proc is None or self.proc.stdin is None:
            return
        try:
            self.proc.stdin.write((text + "\n").encode())
            self.proc.stdin.flush()
        except (BrokenPipeError, OSError, ValueError):
            self._status("Falha ao enviar: o script não está mais lendo a entrada.")
            return
        self.log.note(shown if shown is not None else text)
        self._need_new_data = True
        self._sent_at = time.monotonic()

    def _manual_send(self):
        if self._running:
            self._send(self.input.text())
            self.input.clear()

    # ------------------------------------------------------------------ fechar
    def closeEvent(self, event):
        if self._running:
            r = QtWidgets.QMessageBox.question(
                self, "Script em execução",
                "O script ainda está rodando. Encerrar e sair?")
            if r != QtWidgets.QMessageBox.Yes:
                event.ignore()
                return
            self._signal_group(signal.SIGKILL)
            if self.reader:
                self.reader.wait(3000)
        event.accept()


def main():
    if sys.platform != "linux":
        print("Aviso: este programa foi feito para Linux.", file=sys.stderr)
    here = Path(__file__).resolve().parent
    workdir = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else here
    app = QtWidgets.QApplication(sys.argv)
    app.setApplicationName("mtklogo GUI")
    win = MainWindow(workdir)
    win.show()
    sys.exit(app.exec())


if __name__ == "__main__":
    main()
