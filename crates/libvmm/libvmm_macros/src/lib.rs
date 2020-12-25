#[cfg(all(target_arch = "x86_64", feature = "vmx"))]
mod vmcs;

extern crate proc_macro;

use proc_macro::TokenStream;

///
/// Macro usage:
///
/// #[vmcs_field({16, 32, 64}, {"R", "RW"})
///
/// This can only be used for "enums"
#[cfg(all(target_arch = "x86_64", feature = "vmx"))]
#[proc_macro_attribute]
pub fn vmcs_access(args: TokenStream, input: TokenStream) -> TokenStream {
    vmcs::vmcs_access(args, input)
}
