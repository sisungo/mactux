mod syscall;

/// Generates a system call handler function.
#[proc_macro_attribute]
pub fn syscall(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    syscall::syscall(attr, item)
}
