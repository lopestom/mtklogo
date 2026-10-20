#!/usr/bin/env bash
# mtklogos.sh (v2) - interactive front-end for mtklogo (MediaTek logo.bin)
#
# Credits / base:
#   cyberknight777/mtklogo (binary and source code) · arlept/mtklogo (original)
#   Sakurajima07/4pda (original script) · lopestom (Linux fork)
#
# What changes in relation to v1 (see ANALYSIS_AND_SUGGESTIONS.md):
#   * no recursion (v1 called ./mtklogos.sh inside select -> each turn
#     stacked another process); now it's a simple loop;
#   * profiles come from mtklogo.yaml (`mtklogo profiles`): new profile in yaml
#     appears alone in the menu, without editing the script;
#   * "Detect profile" tests all profiles in --no-out mode and shows which
#     recognizes more slots;
#   * separate folders: logo/ (unpack), explore/ (explore), out/ (results),
#     backup/ (copy of the original logo.bin, made once and never overwritten);
#   * no longer copies the binary into logo/ (this would break the repack);
#   * the name of the generated file is the same as the message informs;
#   * validates the entered width and asks for confirmation before deleting/overwriting;
#   * with mtklogo patched, use --reference (checks dimensions and size).
#
# Usage: ./mtklogos.sh (LOGO_BIN=logo.bin ./mtklogos.sh for another file)

PS3='#? '                       # prompt from select (the GUI recognizes "#?")
cd "$(dirname "$(readlink -f "$0")")" || exit 1

LOGO_BIN="${LOGO_BIN:-logo.bin}"
UNPACK_DIR="logo"
EXPLORE_DIR="explore"
OUT_DIR="out"
BACKUP_DIR="backup"
STATE_FILE=".mtklogos_state"

C1=$'\033[1;36m'; C2=$'\033[1;34m'; CW=$'\033[1;33m'; CE=$'\033[1;31m'; C0=$'\033[0m'
banner() { echo -e "${C1} ************************************************* ${C0}"
           for l in "$@"; do echo -e "${C2}   $l ${C0}"; done
           echo -e "${C1} ************************************************* ${C0}"; }
warn()   { echo -e "${CW}Aviso:${C0} $*"; }
err()    { echo -e "${CE}Erro:${C0} $*"; }

# ---- locate mtklogo ---------------------------------------------------
chmod u+x ./mtklogo 2>/dev/null
if [ -x ./mtklogo ]; then MTKLOGO=./mtklogo
elif command -v mtklogo >/dev/null 2>&1; then MTKLOGO=$(command -v mtklogo)
else err "The binary mtklogo was not found in $PWD nor in the PATH."; exit 1; fi
echo "Using: $MTKLOGO  (folder $PWD)"

HAS_PROFILES=0; HAS_REFERENCE=0
"$MTKLOGO" profiles >/dev/null 2>&1 && HAS_PROFILES=1
"$MTKLOGO" repack --help 2>/dev/null | grep -q -- '--reference' && HAS_REFERENCE=1

# ---- utilities ------------------------------------------------------------
strip_ansi() { sed -E 's/\x1b\[[0-9;]*[A-Za-z]//g'; }

confirm() {   # Confirm "question" -> 0 if yes; default = no
    local a; printf '%s [s/N] ' "$1"; read -r a || return 1
    [[ "$a" =~ ^([sSyY]|sim|SIM|yes|YES)$ ]]
}

ask() {       # ask "question" -> reply in $REPLY
    printf '%s ' "$1"; read -r REPLY || exit 0
}

list_profiles() {   # lines: name<TAB>model<TAB>number formats<TAB>aliases
    if [ "$HAS_PROFILES" = 1 ]; then "$MTKLOGO" profiles
    else   # fallback without the subcommand: reads the names directly from the YAML
        local y; for y in "$HOME/.config/mtklogo.yaml" /etc/mtklogo.yaml ./mtklogo.yaml; do [ -f "$y" ] && break; done
        sed -nE 's/^[[:space:]]*-[[:space:]]+name:[[:space:]]*([^[:space:]#]+).*/\1\t?\t?\t/p' "$y"
    fi
}

choose_profile() {  # Choose a profile -> $PROFILE (empty if cancelled)
    local -a names labels; local n cm cnt al
    while IFS=$'\t' read -r n cm cnt al; do
        names+=("$n"); labels+=("$n  [$cm, $cnt formats${al:+, alias: $al}]")
    done < <(list_profiles)
    [ ${#names[@]} -eq 0 ] && { err "No profiles found in mtklogo.yaml"; PROFILE=""; return 1; }
    echo; echo "  What is the resolution profile of your device?"
    local i
    select _ in "${labels[@]}" "Cancel"; do
        [ -z "${REPLY:-}" ] && continue
        if [ "$REPLY" -eq $(( ${#names[@]} + 1 )) ] 2>/dev/null; then PROFILE=""; return 1; fi
        i=$(( REPLY - 1 ))
        if [ "$i" -ge 0 ] && [ "$i" -lt ${#names[@]} ]; then PROFILE="${names[$i]}"; return 0; fi
        echo "  Invalid option."
    done
    PROFILE=""; return 1
}

need_logo_bin() {
    [ -f "$LOGO_BIN" ] && return 0
    banner "$LOGO_BIN not found in $PWD" "Put your logo.bin file in this folder."
    return 1
}

make_backup() {   # cópia única do logo.bin original
    mkdir -p "$BACKUP_DIR"
    if [ ! -e "$BACKUP_DIR/$(basename "$LOGO_BIN").orig" ]; then
        cp -p -- "$LOGO_BIN" "$BACKUP_DIR/$(basename "$LOGO_BIN").orig" &&
            echo "Backup of the original file: $BACKUP_DIR/$(basename "$LOGO_BIN").orig"
    fi
}

clean_dir() {     # Empty one of the workbooks (fixed names, never arbitrary paths).
    local d="$1"
    case "$d" in "$UNPACK_DIR"|"$EXPLORE_DIR") ;; *) err "folder not allowed: $d"; return 1;; esac
    if [ -d "$d" ] && [ -n "$(ls -A -- "$d" 2>/dev/null)" ]; then
        confirm "The folder $d/ already contains files. Delete the contents and continue?" || return 1
        find "$d" -mindepth 1 -delete
    fi
    mkdir -p "$d"
}

# ---- actions -------------------------------------------------------------------
do_unpack() {
    need_logo_bin || return
    choose_profile || { echo "Cancelled."; return; }
    clean_dir "$UNPACK_DIR" || { echo "Cancelled."; return; }
    make_backup
    echo "Selected profile: $PROFILE"
    "$MTKLOGO" unpack -p "$PROFILE" "$LOGO_BIN" -o "$UNPACK_DIR" 2>&1 | strip_ansi | grep -v '░' | grep -v '^$'
    local rc=${PIPESTATUS[0]}
    echo "$PROFILE" > "$STATE_FILE"
    local raw png
    shopt -s nullglob
    local -a _z=("$UNPACK_DIR"/*raw.z) _p=("$UNPACK_DIR"/*.png)
    shopt -u nullglob
    raw=${#_z[@]}; png=${#_p[@]}
    if [ "$rc" -ne 0 ]; then err "the unpacking ended with code. $rc"; return; fi
    banner "Unpack completed (profile $PROFILE)" "$png PNG images in $UNPACK_DIR/" "$raw slots maintained as .z (unrecognized resolution-dimension)"
    [ "$raw" -gt 0 ] && warn "slots .z files are preserved in the repack; to view them as an image, add the dimension to the profile in mtklogo.yaml."
}

do_detect() {
    need_logo_bin || return
    echo "Testing all profiles (read-only, nothing is extracted)..."
    local n cm cnt al out ok total best="" bestok=-1 tmp bin lbin
    local -a rows
    # The -n mode saves a .mtklogo-meta file in the current folder; it runs in a temporary folder
    tmp=$(mktemp -d); bin=$(readlink -f "$MTKLOGO"); lbin=$(readlink -f "$LOGO_BIN")
    while IFS=$'\t' read -r n cm cnt al; do
        out=$(cd "$tmp" && "$bin" unpack -n -p "$n" "$lbin" 2>&1 | strip_ansi)
        ok=$(grep -c 'will be exported as' <<<"$out")
        total=$(sed -nE 's/.*logo image has ([0-9]+).*/\1/p' <<<"$out" | head -1)
        rows+=("$ok|$total|$n")
        if [ "$ok" -gt "$bestok" ]; then bestok=$ok; best=$n; fi
    done < <(list_profiles)
    rm -rf -- "$tmp"
    echo; echo "  Recognized slots / total   profile"
    printf '%s\n' "${rows[@]}" | sort -t'|' -k1,1nr | head -8 | while IFS='|' read -r ok total n; do
        printf '        %3s / %-3s              %s\n' "$ok" "${total:-?}" "$n"; done
    echo
    if [ "$bestok" -le 0 ]; then warn "no profile recognized slots; use 'Unpack other' (explore) to discover the width."; return; fi
    echo "Best candidate: $best ($bestok slots)."
    warn "profiles with the same screen but a different color model may tie; check the PNGs after unpacking."
    if confirm "Unpack now with the profile $best?"; then
        PROFILE="$best"; clean_dir "$UNPACK_DIR" || return; make_backup
        "$MTKLOGO" unpack -p "$PROFILE" "$LOGO_BIN" -o "$UNPACK_DIR" 2>&1 | strip_ansi | grep -v '░' | grep -v '^$' | tail -3
        echo "$PROFILE" > "$STATE_FILE"
    fi
}

do_explore() {
    need_logo_bin || return
    local w s
    while :; do
        ask "  Device screen width in pixels (e.g., 1080), empty cancels:"; w="$REPLY"
        [ -z "$w" ] && { echo "Cancelled."; return; }
        [[ "$w" =~ ^[0-9]+$ ]] && [ "$w" -ge 16 ] && [ "$w" -le 10000 ] && break
        echo "  Invalid value: enter only numbers (16 to 10000)."
    done
    ask "  Slots to explore (e.g.: 0  ou  0,1  ou  0-3) [0]:"; s="${REPLY:-0}"
    [[ "$s" =~ ^[0-9]+([,-][0-9]+)*$ ]] || { err "invalid slots: $s"; return; }
    clean_dir "$EXPLORE_DIR" || { echo "Cancelled."; return; }
    make_backup
    "$MTKLOGO" explore "$LOGO_BIN" --slots "$s" --width "$w" -o "$EXPLORE_DIR" 2>&1 | strip_ansi | grep -v '░' | grep -v '^$'
    banner "Exploration complete: see the PNGs at $EXPLORE_DIR/" \
           "Choose the PNG with the correct colors and image;" \
           "The file suffix is ​​the color_model of your device."
    echo "  Then use 'Unpack' with -m <model> on a $w wide profile or create a profile in yaml."
}

do_repack() {
    local dir="${1:-$UNPACK_DIR}" prof="" out files a alpha=() ref=()
    [ -f "$STATE_FILE" ] && prof=$(<"$STATE_FILE")
    [ -d "$dir" ] || { err "folder $dir/ doesn't exist: unpack it first."; return; }
    shopt -s nullglob; files=("$dir"/logo_*); shopt -u nullglob
    [ ${#files[@]} -eq 0 ] && { err "no logo_* file in $dir/"; return; }
    echo "  ${#files[@]} files in $dir/"
    confirm "Remove the alpha channel (-a, assumes opaque images)?" && alpha=(--alpha)
    mkdir -p "$OUT_DIR"
    out="$OUT_DIR/logo_${prof:-custom}_$(date +%Y%m%d-%H%M%S).bin"
    if [ "$HAS_REFERENCE" = 1 ] && [ -f "$LOGO_BIN" ]; then ref=(--reference "$LOGO_BIN"); fi
    "$MTKLOGO" repack "${alpha[@]}" "${ref[@]}" "${files[@]}" -o "$out" 2>&1 | strip_ansi | grep -v '░' | grep -v '^$' | grep -v ' is slot [0-9]* in '
    if [ "${PIPESTATUS[0]}" -ne 0 ]; then
        if [ "$HAS_REFERENCE" = 1 ] && confirm "Validation revealed discrepancies. Repackage anyway (--force)?"; then
            "$MTKLOGO" repack "${alpha[@]}" "${ref[@]}" --force "${files[@]}" -o "$out" 2>&1 | strip_ansi | grep -v '░' | grep -v '^$' | grep -v ' is slot [0-9]* in '
            [ "${PIPESTATUS[0]}" -ne 0 ] && { err "repack failed"; rm -f -- "$out"; return; }
        else
            err "repack not performed."; rm -f -- "$out"; return
        fi
    fi
    banner "Repack completed: $out" "$(du -b -- "$out" | cut -f1) bytes · sha256 $(sha256sum -- "$out" | cut -c1-16)…"
    warn "Backup the /logo partition and name it logo_Stock.bin to ensure the file is from & for your device and that you know exactly its resolution. I can't be responsible for any damage caused to your device, use all at your own risk."
}

do_repack_other() {
    ask "  Folder with the logo_NNN_* files already prepared. [$UNPACK_DIR]:"
    local d="${REPLY:-$UNPACK_DIR}"
    [ -d "$d" ] || { err "folder not found: $d"; return; }
    do_repack "$d"
}

do_profiles() {
    echo; printf '  %-22s %-9s %-9s %s\n' PROFILE MODEL FORMATS ALIAS
    list_profiles | while IFS=$'\t' read -r n cm cnt al; do printf '  %-22s %-9s %-9s %s\n' "$n" "$cm" "$cnt" "$al"; done
}

# ---- main loop (without recursion) --------------------------------------------
echo "Caution: experimental profiles (exp_*) have not been verified on real devices."
while :; do
    echo
    select action in "Unpack logo.bin" "Detect profile (tests all)" "Repack logo.bin" \
                      "Unpack other logo.bin (explore)" "Repack other logo.bin (other folder)" \
                      "List profiles" "Exit"; do
        case "$REPLY" in
            1) do_unpack ;;
            2) do_detect ;;
            3) do_repack ;;
            4) do_explore ;;
            5) do_repack_other ;;
            6) do_profiles ;;
            7) echo "Exiting..."; exit 0 ;;
            *) echo "Invalid option."; continue ;;
        esac
        break        # exit the select menu to reprint it.
    done || exit 0
    [ -z "${action:-}" ] && [ -z "${REPLY:-}" ] && exit 0   # EOF
done
