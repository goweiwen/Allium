#!/bin/sh
mydir=`dirname "$0"`
export HOME=$mydir
export PATH=$mydir/bin:$PATH
export LD_LIBRARY_PATH=$mydir/libs:$LD_LIBRARY_PATH
cd $mydir
echo performance > /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor
touch /tmp/stay_awake
ffplay -autoexit -vf "hflip,vflip" -i "$1"
rm -f /tmp/stay_awake
