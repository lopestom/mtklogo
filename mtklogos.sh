#!/bin/bash
# Thanks to [arlept](https://github.com/arlept/mtklogo) from github for original mtklogo binary and code source.
# Thanks to [Sakurajima07](https://4pda.to/forum/index.php?showuser=11778453) from 4pda for original script.

#clear

abort() { echo "Error!"; }

# Start .sh script
echo "Starting .sh script..."
chmod u+x mtklogos.sh
chmod u+x mtklogo
chmod +x mtklogos.sh
chmod +x mtklogo
#chmod 755 ./mtklogos.sh && ./mtklogos.sh
mkdir -p $PWD/logo

echo "Searching for mtklogo tool..."
MTKLOGO=$(find "$PWD" -type f -name "mtklogo" 2>/dev/null | head -n 1)

if [ -z $MTKLOGO ]; then
    echo "mtklogo tool not found in current directory!"
    exit 1
fi

MTKLOGO_DIR=$(dirname "$MTKLOGO")
cd "$MTKLOGO_DIR" || exit
echo "Found mtklogo in: $MTKLOGO_DIR"

select action in "Unpack logo.bin" "Repack logo.bin" "Exit"; do
    case $action in
        "Unpack logo.bin")
            if [ ! -f "$PWD/logo.bin" ]; then
                echo "logo.bin not found in current directory."
                exit 1
            fi
            PS1="  What resolution do you device have?  "
              select resolution in "720x1280" "720x1600" "720x1612" "720x1640" "1080x1920" "1080x2400" "1080x2460" "1280x800"; do
              echo "  The choice that you have selected is : $resolution  "
               ./mtklogo unpack -p $resolution logo.bin -o $PWD/logo
               echo "Unpacking complete."
               break
            done
            ;;
        "Repack logo.bin")
            ./mtklogo repack -o logo_$resolution.bin $PWD/logo/logo_0*
            echo "Repacking complete, logo saved in logo_$resolution.bin"
            break
            ;;
        "Exit")
            echo "Exiting..."
            exit 0
            ;;
        *)
            echo "Invalid option."
            ;;
    esac
done
