# Register Redirection

On x86_64, thread-local variables are commonly accessed via *Segment Registers*, which in this case are usually `gs` and
`fs`. The use of segment registers diverses between operating systems:

 - On macOS, `gs` is used for thread-local storage
 - On Linux, `fs` is used for thread-local storage, but `gs` can be also used

On Intel Macs, `fs` base address can be set via Mach APIs. However, on Apple Silicon Macs running Rosetta 2, the Mach APIs and
debugging APIs to set segment registers are no longer working properly. Though Linux allows use of the `gs` segment register,
but we may notice that the register is really, really rarely used. To do so, we would:

 - Redirect all access to `fs` to `gs`
 - Switch `gs` register when entering/exiting to/from Linux context
 - Deny set of `gs` register in Linux context
