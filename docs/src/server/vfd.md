# Virtual File Descriptors

Some kinds of file descriptors that exist in Linux, cannot be natively implemented on macOS. To implement them, MacTux
introduced a new concept called *Virtual File Descriptor*, a.k.a. *VFD*. On the OS side, *VFD*s are just open file
descriptors to `/dev/null`. However, all operations on the file descriptor over `rtenv` become IPC invocations to the
MacTux Server.

## InvalidFD

Some file descriptors, like `io_uring`, are handled on the client side, but is designed to occupy a *VFD* slot. To operate
them are required specific calls, and other general FD operations should never succeed. InvalidFDs are designed to implement
such file descriptors.
