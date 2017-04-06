#!/bin/bash

set -e

echo "Please run 'st-util' in another terminal window (you might need sudo)"
echo ""

binary=$(basename $(pwd))
case "$@" in

    "--release")

        mode=release
        ;;
    *)
        mode=debug
        ;;
esac

arm-none-eabi-gdb -iex 'add-auto-load-safe-path .' -ex "tar ext :4242" -ex "load-reset" target/stm32f7/"$mode"/"$binary"
