use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn, Pat, parse_macro_input};

pub fn gui_helper_method(_: TokenStream, item: TokenStream) -> TokenStream {
    let item_fn: ItemFn = parse_macro_input!(item);
    let vis = item_fn.vis;
    let ident = item_fn.sig.ident;
    let body = item_fn.block;
    let inputs = item_fn.sig.inputs;
    let output = item_fn.sig.output;

    let mut input_ty = Vec::with_capacity(inputs.len());
    let mut call_impl_inputs = Vec::with_capacity(inputs.len());

    for input in inputs.iter() {
        let FnArg::Typed(pat_type) = input else {
            return quote! {
                ::std::compile_error!("receivers are not allowed in GUI helper methods");
            }
            .into();
        };
        let Pat::Ident(ident) = &*pat_type.pat else {
            return quote! {
                ::std::compile_error!("only ident patterns are allowed in system call handlers");
            }
            .into();
        };
        let ty = &pat_type.ty;
        input_ty.push(ty);
        call_impl_inputs.push(ident);
    }

    quote! {
        #vis fn #ident(id: u64, mut stream: &mut dyn crate::ipc::Stream) {
            fn __impl(#inputs) #output #body
            let Ok((#(#call_impl_inputs),*)) : Result<(#(#input_ty),*), _> = ::bincode::decode_from_std_read(
                &mut stream,
                ::bincode::config::standard(),
            ) else {
                crate::ipc::_handle_proto_error(stream, id);
                return;
            };
            let resp = __impl(#(#call_impl_inputs),*);
            let status = crate::ipc::Respond::status(&resp);
            let header = ::structures::internal::mactux_gui_abi::ResponseHeader {
                id,
                status,
            };
            _ = ::bincode::encode_into_std_write(&header, &mut stream, ::bincode::config::standard());
            _ = crate::ipc::Respond::write_body(resp, stream);
        }
    }.into()
}
