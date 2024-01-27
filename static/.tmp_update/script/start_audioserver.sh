#!/bin/sh

if [ -f /customer/app/axp_test ]; then
	model=354
else
	model=283
fi

if [ -f /customer/lib/libpadsp.so ]; then
	LD_PRELOAD=as_preload.so audioserver_"$model" &
fi