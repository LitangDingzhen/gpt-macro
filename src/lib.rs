// SPDX-License-Identifier: MIT
// Akira Moroo <retrage01@gmail.com> 2023

use proc_macro::TokenStream;
use std::collections::HashSet;
use syn::{
    parse::{Parse, ParseStream, Result},
    parse_macro_input, Ident, Token,
};

mod chatgpt;

/// Parses a list of test function names separated by commas.
///
/// test_valid, test_div_by_zero
///
/// The function name is used to generate the test function name.
struct Args {
    test_names: HashSet<Ident>,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        let test_names = input.parse_terminated::<Ident, Token![,]>(Ident::parse)?;
        Ok(Args {
            test_names: test_names.into_iter().collect(),
        })
    }
}

/// Attribute macro for automatically generating tests for functions.
///
/// # Example
///
/// ```
/// use r#gpt_auto_test::gpt_auto_test;
///
/// #[gpt_auto_test(test_valid, test_div_by_zero)]
/// fn div_u32(a: u32, b: u32) -> u32 {
///    a / b
/// }
/// ```
#[proc_macro_attribute]
pub fn gpt_auto_test(args: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the list of test function names that should be generated.
    let args = parse_macro_input!(args as Args);

    let output = chatgpt::generate_tests(input, args.test_names).unwrap();

    output
}
