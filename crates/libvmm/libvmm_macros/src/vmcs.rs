use syn::parse::{Parse, ParseStream};
use syn::{LitInt, LitStr, Token, Error, parse_macro_input, ItemEnum};
use proc_macro::TokenStream;
use quote::{format_ident, quote};

pub struct VmcsFieldArguments {
    pub width: u64,
    pub access: LitStr,
}

impl Parse for VmcsFieldArguments {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let width_lit: LitInt = input.parse()?;
        input.parse::<Token![,]>()?;
        let access: LitStr = input.parse()?;
        let width: u64 = width_lit.base10_parse().expect("Invalid width");

        if width != 16 && width != 32 && width != 64 {
            panic!("width can only be \"16\", \"32\", and \"64\"");
        }

        if access.value().as_str() != "R" && access.value().as_str() != "RW" {
            panic!("access can only be \"R\" or \"RW\"");
        }

        Ok(VmcsFieldArguments { width, access })
    }
}

pub fn vmcs_access(args: TokenStream, input: TokenStream) -> TokenStream {
    let VmcsFieldArguments { width, access } = parse_macro_input!(args as VmcsFieldArguments);

    let enum_stream = parse_macro_input!(input as ItemEnum);
    let name = enum_stream.ident.clone();
    let vm_size = format_ident!("u{}", width);

    let read_fn = quote! {
        pub fn read(&self) -> x86::vmx::Result<#vm_size> {
            unsafe { x86::bits64::vmx::vmread(*self as u32).map(|v| v as #vm_size) }
        }
    };

    let write_fn = quote! {
        pub fn write(&self, value: #vm_size) -> x86::vmx::Result<()> {
            unsafe { x86::bits64::vmx::vmwrite(*self as u32, value as u64) }
        }
    };

    if access.value().as_str() == "R" {
        TokenStream::from(quote! {
            #enum_stream

            impl #name {
                #read_fn
            }
        })
    } else {
        TokenStream::from(quote! {
            #enum_stream

            impl #name {
                #read_fn
                #write_fn
            }
        })
    }
}
