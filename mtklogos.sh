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

select action in "Unpack logo.bin" "Repack logo.bin" "Unpack other logo.bin" "Repack other logo.bin" "Exit"; do
    case $action in
        "Unpack logo.bin")
            if [ ! -f "$PWD/logo.bin" ]; then
                echo "logo.bin not found in current directory."
                echo -e "\033[1;36m ************************************************* \033[0m "
                echo -e "\033[5;34m   logo.bin file not found in current directory. \033[0m "
                echo -e "\033[1;36m ************************************************* \033[0m "
                exit 1
            fi
            PS1="  What resolution do you device have?  "
              select resolution in "720x1280" "720x1600" "720x1612" "720x1640" "1080x1920" "1080x2400" "1080x2460" "1280x800" "1220x2712" "1200x1920" "1920x3446" "1080x2340"; do
              echo "  The choice that you have selected is : $resolution  "
               ./mtklogo unpack -p $resolution logo.bin -o $PWD/logo
               echo -e "\033[1;36m *************************** \033[0m "
               echo -e "\033[5;34m     Unpacking logo_$resolution.bin completed. \033[0m "
               echo -e "\033[1;36m *************************** \033[0m "
               ./mtklogos.sh
        done
        ;;
        "Unpack other logo.bin")
               ./mtklogo unpack logo.bin -o $PWD/logo --mode rgbabe --slots 0
               echo -e "\033[1;36m *************************** \033[0m "
               echo -e "\033[5;34m     Unpacking complete. \033[0m "
               echo -e "\033[1;36m *************************** \033[0m "
               ./mtklogos.sh
               ;;
        "Repack logo.bin")
         ./mtklogo repack logo/* -o logo_$resolution.bin
         echo -e "\033[1;36m ********************************************* \033[0m "
         echo -e "\033[5;34m  Repacking complete, logo saved in logo_repacked.bin \033[0m "
         echo -e "\033[1;36m ********************************************* \033[0m "
         ./mtklogos.sh
        ;;
        "Repack other logo.bin")
            cp mtklogo $PWD/logo/
            sleep 1
            cd $PWD/logo/
            ./mtklogo repack --output otherlogo.bin logo_*
            echo -e "\033[1;36m ************************************************* \033[0m "
            echo -e "\033[5;34m  Repacking complete, logo saved in otherlogo.bin \033[0m "
            echo -e "\033[1;36m ************************************************* \033[0m "
            cd -
            ;;
        "Exit")
            echo "Exiting..."
            exit 1
            ;;
        *)
            echo "Invalid option."
            ;;
    esac
done

