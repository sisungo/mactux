# MacTux
MacTux is a compatibility layer that enables you to run Linux binaries on macOS.

🚧 INCOMPLETE PROJECT 🚧

## Architecture Support
Only x86_64 is currently supported. Note that running MacTux on ARM Macs with Rosetta 2 is supported, too.

## Compatibility
Currently, we have tested:

 - musl-libc dynamic linker
 - glibc dynamic linker
 - gnu coreutils
 - busybox
 - toybox
 - opusdec
 - sqlite3
 - bash
 - python3
 - tinycc

We suggest you to run dynamically linked or static-pie binaries, due to macOS's ASLR policy have not yet been solved
completely.

## Filesystem Hierarchy
We have our own VFS stack, so filesystem mounts are independent to the macOS filesystem. See `tools/mkrootfs/fstab` for
details.

`~/.mactux/rootfs` is the emulated root directory. Currently a minimal Alpine Linux rootfs can be installed and work.

## Multimedia Support
We plan to support multimedia APIs, like D-Bus, OSS, ALSA, X11, Wayland, etc.

Currently, we have a working \(but naive\) implementation of OSS audio output.

## Roadmap
We currently prioritizes to implement:

 - Multi-thread support
 - Networking support
 - Symlink support
 - Epoll support
 - Full procfs support
 - sysfs support
