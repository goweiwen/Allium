#!/bin/sh
DIR=$(dirname $0)

if [ -f /customer/app/axp_test ]; then
    exec "$DIR/retroarch_miyoo354" "$@"
else
    exec "$DIR/retroarch_miyoo283" "$@"
fi
