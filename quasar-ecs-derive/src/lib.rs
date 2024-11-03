mod bundle;
mod component;
mod util;

use crate::{
    bundle::DeriveBundle,
    component::DeriveComponent,
    util::Deriver,
};

#[proc_macro_derive(Component, attributes(quasar))]
pub fn derive_component(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    DeriveComponent::run(input)
}

#[proc_macro_derive(Bundle, attributes(quasar))]
pub fn derive_bundle(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    DeriveBundle::run(input)
}
