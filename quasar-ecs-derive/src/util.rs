use std::fmt::Display;

use darling::FromDeriveInput;
use proc_macro2::{
    Span,
    TokenStream,
};
use syn::{
    parse_macro_input,
    spanned::Spanned,
    DeriveInput,
    Ident,
    Index,
    Member,
    Type,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error while parsing macro input")]
    Syn(#[from] syn::Error),
    #[error("Error while parsing macro attributes")]
    Darling(#[from] darling::Error),
}

impl Error {
    pub fn new<T: Display>(span: Span, message: T) -> Self {
        Self::Syn(syn::Error::new(span, message))
    }

    pub fn into_compile_error(self) -> TokenStream {
        match self {
            Self::Syn(error) => error.into_compile_error(),
            Self::Darling(error) => error.write_errors(),
        }
    }
}

pub trait Deriver {
    fn generate_code(self) -> Result<TokenStream, Error>;

    fn run(input: proc_macro::TokenStream) -> proc_macro::TokenStream
    where
        Self: FromDeriveInput,
    {
        fn run_inner<D: Deriver + FromDeriveInput>(
            input: DeriveInput,
        ) -> Result<TokenStream, Error> {
            D::from_derive_input(&input)?.generate_code()
        }

        let input = parse_macro_input!(input as DeriveInput);
        match run_inner::<Self>(input) {
            Ok(output) => output,
            Err(error) => error.into_compile_error(),
        }
        .into()
    }
}

#[derive(Clone)]
pub struct FieldName {
    pub span: Span,
    pub member: Member,
    pub var: Ident,
}

impl FieldName {
    pub fn new(index: usize, ident: Option<&Ident>, ty: &Type) -> Self {
        if let Some(ident) = ident {
            Self {
                span: ident.span(),
                member: Member::Named(ident.clone()),
                var: ident.clone(),
            }
        }
        else {
            let span = ty.span();
            Self {
                span,
                member: Member::Unnamed(Index {
                    index: index as u32,
                    span,
                }),
                var: Ident::new(&format!("_{index}"), span),
            }
        }
    }
}
