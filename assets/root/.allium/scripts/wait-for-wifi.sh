#!/bin/sh

counter=0

while ! ping -c 1 -w 1 1.1.1.1 > /dev/null 2>&1; do
	counter=$((counter+1))
	if [ $counter -gt 60 ]; then
		exit 1
	fi
	sleep 1
done

exit 0