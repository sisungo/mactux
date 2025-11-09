# The VFS Model

To implement Linux-specific filesystem features, e.g. mount namespaces, chroot-ing \(which is denied by SIP on macOS\), 
procfs, MacTux has its own VFS model, implemented on the server. To get a balance between function and performance, and
implement it stably on macOS, we designed a VFS model different from Linux, but should work well for most programs.
