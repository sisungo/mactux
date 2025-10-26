use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{FnArg, Ident, ItemFn, Pat, parse_macro_input};

pub fn syscall(_: TokenStream, item: TokenStream) -> TokenStream {
    let item_fn: ItemFn = parse_macro_input!(item);
    let vis = item_fn.vis;
    let ident = item_fn.sig.ident;
    let unsafety = item_fn.sig.unsafety;
    let body = item_fn.block;
    let inputs = item_fn.sig.inputs;
    let output = item_fn.sig.output;

    let mut input_conversion = Vec::with_capacity(inputs.len());
    let mut call_impl_inputs = Vec::with_capacity(inputs.len());

    for (n, input) in inputs.iter().enumerate() {
        let FnArg::Typed(pat_type) = input else {
            return quote! {
                ::std::compile_error!("receivers are not allowed in system call handlers");
            }
            .into();
        };
        let Pat::Ident(ident) = &*pat_type.pat else {
            return quote! {
                ::std::compile_error!("only ident patterns are allowed in system call handlers");
            }
            .into();
        };
        let arg_method = Ident::new(&format!("arg{n}"), Span::mixed_site());
        let ty = &pat_type.ty;
        input_conversion.push(quote! {
            let #ident: #ty = crate::FromSyscall::from_syscall(uctx.#arg_method());
        });
        call_impl_inputs.push(ident);
    }

    quote! {
        #vis #unsafety fn #ident(uctx: &mut ::libc::ucontext_t) {
            fn __impl(#inputs) #output #body
            unsafe { ::rtenv::emuctx::leave_emulated(); }
            #(#input_conversion)*
            crate::UcontextExt::ret(uctx, crate::ToSysret::to_sysret(__impl(#(#call_impl_inputs,)*)));
            //eprintln!("{}{:?} = {}", stringify!(#ident), (#(#call_impl_inputs,)*), uctx.sysno());
            unsafe { ::rtenv::emuctx::enter_emulated(); }
        }
    }.into()
}
