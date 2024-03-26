#!/bin/bash
echo "DEVICE:"
cat /sys/devices/virtual/dmi/id/{sys_vendor,product_{family,version,name},bios_version}
echo
echo "SOUND_CARD:"
aplay -l | grep card
