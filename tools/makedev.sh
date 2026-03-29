#!/bin/sh

mknod /dev/null c 1 3
mknod /dev/zero c 1 5
mknod /dev/full c 1 7
mknod /dev/random c 1 8
mknod /dev/urandom c 1 9

mknod /dev/tty c 5 0
mknod /dev/console c 5 1

mknod /dev/dsp c 14 3
