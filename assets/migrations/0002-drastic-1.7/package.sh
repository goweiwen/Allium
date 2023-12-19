#!/bin/sh
set -eu
drastic_url="https://github.com/steward-fu/nds/releases/download/v1.7/drastic-v1.7_miyoo.zip"
dist="${PWD}"/dist/.allium/migrations/0002-drastic-1.7/drastic.zip
wget "$drastic_url" -O "$dist"
