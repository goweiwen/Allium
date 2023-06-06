#!/bin/sh

if [ -f /customer/app/axp_test ]; then
    exec "$PWD/retroarch_miyoo354" "$@"
else
    exec "$PWD/retroarch_miyoo283" "$@"
fi
