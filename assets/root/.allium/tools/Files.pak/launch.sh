#!/bin/sh

export CPU_PATH=/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor

echo ondemand > "$CPU_PATH"
exec ./DinguxCommander
