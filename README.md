## About

mtklogo is a Command Line Interface with a thin reusable library which lets you hack
an MTK "LOGO" image. The main (and probably only) use case is replacing
your vendor logo with a custom one when your MTK-based phone boots!

This program is able to unpack, and subsequently repack all logo images (including
the big logo, the phone charging animation, etc. provided you can tell - or guess - the dimension
of these images).

**WARNING**: replacing the vendor's logo image may break your device.

Do backups before proceeding.

I can't be responsible for any damage caused to your device, use this material at your own risk.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![License: APACHE](https://img.shields.io/badge/license-APACHE-blue.svg)](LICENSE)

The original file [Readme](https://github.com/arlept/mtklogo/blob/master/README.md) of mtklogo source code is from @arlept.

-------------

## mtklogo Linux

### Brief description
I've been using Linux for a while and don't want to constantly install Windows 10/11 to use LogoBuilder 11/03/2015 (version 1.6).

[Sidorovich74](https://4pda.to/forum/index.php?showuser=9596588) has post on the 4pda forum was the first informations from [_Mature_](https://4pda.to/forum/index.php?showuser=4368144)'s instructions in the [Replacing the boot logo on Android. (Post by _Mature_ 130797752)](https://4pda.to/forum/index.php?showtopic=724792&view=findpost&p=130797752)

I tested and commented something in post [Replacing the boot logo on Android. (Post by lopestom 132889514)](https://4pda.to/forum/index.php?showtopic=724792&view=findpost&p=132889514).

The purpose of this post is to update and provide instructions for Linux users on how to change the logo (logo.bin_img) of Mediatek devices. I was reminded of this situation and was disappointed to find that the link provided in "Xiaomi Redmi 9C NFC - Modifications and Beautifications" did not work correctly with the correct Linux permissions. By combining the two projects, you can get something relatively safe to use and therefore functional.


Therefore, I made some interesting changes and additions aimed not only at beginners but also at advanced users who need something simple.
As I don't want to change/tamper with the source code (I don't see myself capable of doing so) either due to time or incapacity, I leave the original mtklogo file and the modified files as they are.

### What was changed/included
 1. Added the 720x1612 resolution parameter to the mtklogo.yaml file to allow specifying new devices.
    
    – Added resolution parameters to the name_ file to correctly unpack the logo.bin file into the mtklogos.sh file, allowing the user to select them directly in the terminal;

    – Changed the name_ parameters and added them to the alias;

 2. Changed the name of start.sh to mtklogos.sh;
    
    – Added the names of the users who suggested original ideas;
    
    – Added the necessary chmod permissions;

    – Added the ability to select the resolution based on the user's device. This provides more flexibility without needing to check the template (name_) or alias (alias) in the mtklogo.yaml file, eliminating the need to write commands with this template;

    – Changed the display of the compressed bin file name in the output.

Available resolutions: "720x1280" "720x1600" "720x1612" "720x1640" "1080x1920" "1080x2400" "1080x2460" "1280x800"


### What does the mtklogos.sh script do?

It gives you the option in the Linux terminal to choose between three options:

***1. Unpack logo.bin***

   When you select option 1, you'll be prompted to select your device's resolution. This ensures the unpacking process is successful.
   
***2. Repack logo.bin***

   After unpack logo.bin file file, you can make changes to the logo_000_bgrabe.png and logo_002_bgrabe.png files (initial images) and even the logo_038_bgrabe.png file (for the image that starts before the battery charge begins).

Also, if you want to try modifying the files that create the battery charge animation, feel free. But obviously, this requires more time and testing, as well as full responsibility for your own decisions regarding your device.

***3. Exit***

If you only want to unzip and make modifications to the png files and only after a while repack the new bin file then choose this option.

### An example of the process in operation

![ezgif-5ee6b302e5841A](https://github.com/user-attachments/assets/90b6d3c8-2f54-4bea-aff4-746b1beea185)


### What if my device resolution isn't specified in the project?
You can edit the mtklogo.yaml file and add a name (name_) for the new resolution. After that, simply change the alias (if you have one) and formats. But keep in mind that you'll need to know if the resolution of other images, such as battery indicators and percentages, is appropriate to ensure everything is verified and confirmed.

``` yaml
- name: 1080x2340
color_model: bgrabe
# A profile may fit many phones, so you can give it an alias.
Alias:
- If you want to write a name or codiname of the device
formats:
- { w: 1080, h: 2340, t: "FHD+ (If you want to write a name for the device - full screen)" }
- { w: 45, h: 64, t: "symbol" }
- { w: 57, h: 64, t: "bigger symbol" }
- { w: 163, h: 29, t: "battery charge animation" }
- { w: 163, h: 1, t: "battery charge line" }
- { w: 84, h: 121, t: "small letter" }
- { w: 304, h: 52, t: "charge animation" }
- { w: 108, h: 121, t: "bigger letter (percent)" }
- { w: 304, h: 1, t: "charge line" }
```
After making all possible and necessary changes, edit the mtklogos.sh file and add the new device resolution parameter to line 38.
```ruby
select resolution in "720x1280" "720x1600" "720x1612" "1080x1920" "1080x2400" "1080x2340"; do
```


## License

- The original mtklogo project is licensed under either of

   * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
   * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

- The modified mtklogo Linux project is licensed with [![GPLv3+](https://img.shields.io/badge/license-GPLv3+-red.svg)](https://www.gnu.org/licenses/gpl-3.0.html).

   * Feel free to modify the mtklogos.sh and mtklogo.yaml files according to your device's needs or future ideas/improvements.
   * Always credit the people who made this happen as a way to acknowledge/thank them for their time and testing.
