#!/bin/sh

for pid in $(pgrep -f 'tcpsvd -E 0.0.0.0 21 ftpd'); do
    kill "$pid"
done