use darling::{
    util::SpannedValue,
    FromDeriveInput,
    FromMeta,
};
use proc_macro2::TokenStream;
use quote::{
    quote,
    quote_spanned,
};
use syn::{
    Generics,
    Ident,
};

use crate::util::{
    Deriver,
    Error,
};

#[derive(Clone, Debug, FromDeriveInput)]
#[darling(attributes(quasar), forward_attrs(allow, doc, cfg))]
pub struct DeriveComponent {
    ident: Ident,
    generics: Generics,
    #[darling(default)]
    storage: SpannedValue<StorageType>,
}

#[derive(Clone, Copy, Debug, Default, FromMeta)]
enum StorageType {
    #[default]
    Table,
    SparseSet,
    BitSet,
}

impl Deriver for DeriveComponent {
    fn generate_code(self) -> Result<TokenStream, Error> {
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let ident = &self.ident;
        let storage = match self.storage.as_ref() {
            StorageType::Table => {
                quote_spanned! {
                    self.storage.span() => ::quasar_ecs::StorageType::Table
                }
            }
            StorageType::SparseSet => {
                quote_spanned! {
                    self.storage.span() => ::quasar_ecs::StorageType::SparseSet
                }
            }
            StorageType::BitSet => {
                quote_spanned! {
                    self.storage.span() => ::quasar_ecs::StorageType::BitSet
                }
            }
        };

        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics ::quasar_ecs::Component for #ident #ty_generics #where_clause {
                const STORAGE_TYPE: ::quasar_ecs::StorageType = #storage;
            }
        })
    }
}
