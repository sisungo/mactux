#!/bin/sh

cd $(dirname $0)

build_x86_64() {
    clang \
        -target x86_64-unknown-linux-none \
        -nostdlib \
        -fuse-ld=lld \
        -shared \
        -Wl,--version-script=x86_64.map \
        -o linux-vdso.so.1 \
        x86_64.s
}

build_x86_64
