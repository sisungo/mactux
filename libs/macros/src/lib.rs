mod gui_helper_method;
mod syscall;

/// Generates a system call handler function.
#[proc_macro_attribute]
pub fn syscall(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    syscall::syscall(attr, item)
}

/// Generates a GUI helper method function.
#[proc_macro_attribute]
pub fn gui_helper_method(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    gui_helper_method::gui_helper_method(attr, item)
}
