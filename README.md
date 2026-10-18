# mtklogo_gui based on the mtklogo_2025-2 branch [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE) [![License: APACHE](https://img.shields.io/badge/license-APACHE-blue.svg)](LICENSE)
This branch aims to improve the project through a graphical user interface (GUI).

The real goal is to have a visually graphical process with mtklogo_Linux, and possible future updates that may have better options are welcome.

## About 

mtklogo is a Command Line Interface with a thin reusable library which lets you hack an MTK "LOGO" image. The main (and probably only) use case is replacing the OEM logo with a custom one when your MTK-based phone boots!

This application is able to unpack, and subsequently repack all logo images (including the logo/initials big pictures, the phone charging animation, etc. provided you can tell - or guess the resolution - size/dimension of these images).

> [!CAUTION]
> replacing the vendor's logo image may break your device.
> 
> Do backups before proceeding.

**I can't be responsible for any damage caused to your device, use this material at your own risk.**

# mtklogo GUI

Minimalist graphical interface (Python + Qt) for `mtklogos.sh`.
The script runs **exactly as is** (`bash ./mtklogos.sh`): the GUI only captures the output in real time and answers its questions through dialogs.

## Contents

| File | Description |
|---|---|
| `mtklogo_gui.py` | The graphical interface (PySide6, with fallback to PyQt5) |
| `run.sh` | Launcher: uses the system's Qt or creates a `.venv` with PySide6 |
| `requirements.txt` | Python dependency (`PySide6-Essentials`) |
| `mtklogos.sh` | Original script (**unchanged**) |
| `mtklogo` | Linux x86-64 binary v1.0.1 (identical to the [latest upstream release](cyberknight777/mtklogo@41d7abe)) |
| `mtklogo.yaml` | Fork profiles **+ 8 merged upstream profiles** (see below) |
| `logo.bin` | File `logo.bin` from the repository (replace with your own)  |

## Requirements

* Linux, `bash`, Python ≥ 3.8
* Qt: `run.sh` automatically installs PySide6 in a local `.venv` file (requires internet the first time).
  If you prefer the system package: `sudo apt install python3-pyside6.qtwidgets` or `sudo apt install python3-pyqt5`.
* If you get *“Could not load the Qt platform plugin xcb”*: `sudo apt install libxcb-cursor0`
  (Fedora: `sudo dnf install xcb-util-cursor`).

## How to use

```bash
unzip mtklogo-gui.zip && cd mtklogo-gui
./run.sh
```

1. Place **your `logo.bin`** in the working folder (by default, this same folder;
   you can change it in “Folder…”). Back up the original before writing anything to the device.
2. Click on **▶ Run run.sh** or **▶ mtklogo_gui.py** - open directly with Terminal.
3. When the script asks for something, a dialog box appears:
   * **menu `select`** → list of options (double click or Enter sends);
   * **text question** (e.g., device width) → text field;
   * **[y/N], (y/n), yes/no** → Yes/No; **“Press Enter…”** → Continue button.
4. **Cancel** in a dialog box asks if you want to end the script. The **■ Stop** button does the same at any time.
5. The “Manual input” field allows you to send text to the script if something is not detected.

Shortcut in the application menu (optional): `./run.sh --install-desktop`

The extracted files are located in `logo/` folder within the working directory; the **Open** button opens the folder in the file manager.

Check the images for colors, size/resolution, and keep in mind to have a copy for further modifications if you wish.

> [!WARNING]
> Replacing the file in the `/logo` partition with a logo_Mod.bin file may cause a bootloop or crash the device if something goes wrong/is chosen/done incorrectly, and use at your own risk. Back up the `/logo` partition and name it logo_Stock.bin to ensure the file is for your device and that you know exactly its resolution.

## How the GUI detects queries (without modifying the script)

* stdout and stderr are joined in a pipe and displayed live, with the ANSI colors of the script.
* The GUI checks in `/proc/<pid>/syscall` if any process in the tree is blocked in `read(0, …)`;
  only then does it consider that the script is waiting for a response (it never interrupts the `mtklogo` working).
* The `#?` prompt of `select` indicates a menu; the preceding `N) option` lines form the list.
  The bash `select` prints the menu **only once**, so the GUI remembers the menu of each process
  (necessary because the script calls `./mtklogos.sh` recursively).
* Only environment change: `COLUMNS=1`, for `select` to list one option per line.
* Bash limitation: `read -p "text"` does not print the prompt when stdin is not a terminal.
  The current script uses `echo -n`, which works; in other scripts, the generic dialog shows the last printed line.

-------------
## mtklogo Linux
Original files [mtklogo_2025](https://github.com/lopestom/mtklogo/blob/mtklogo_2025/README.md).

Original files [mtklogo_2025_26](https://github.com/lopestom/mtklogo/tree/mtklogo_2025_26).

The original source code [mtklogo](https://github.com/arlept/mtklogo) is from @arlept.

The source code mtklogo [latest upstream release](https://github.com/cyberknight777/mtklogo).
