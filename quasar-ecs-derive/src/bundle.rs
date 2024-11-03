use darling::{
    ast::Data,
    FromDeriveInput,
    FromField,
};
use proc_macro2::TokenStream;
use quote::{
    quote,
    quote_spanned,
};
use syn::{
    Generics,
    Ident,
    Type,
    Visibility,
};

use crate::util::{
    Deriver,
    Error,
    FieldName,
};

#[derive(Clone, Debug, FromDeriveInput)]
#[darling(attributes(quasar), forward_attrs(allow, doc, cfg))]
pub struct DeriveBundle {
    ident: Ident,
    vis: Visibility,
    generics: Generics,
    data: Data<(), BundleField>,
}

#[derive(Clone, Debug, FromField)]
struct BundleField {
    ident: Option<Ident>,
    ty: Type,
    bundle: bool,
}

impl Deriver for DeriveBundle {
    fn generate_code(self) -> Result<TokenStream, Error> {
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let ident = &self.ident;

        let fields = self
            .data
            .take_struct()
            .ok_or_else(|| Error::new(ident.span(), "Bundle can only be derived for structs."))?;

        let (_style, fields) = fields.split();

        let mut num_components_from_components = 0;
        let mut num_components_from_bundles = Vec::with_capacity(fields.len());
        let mut for_each_component = Vec::with_capacity(fields.len());
        let mut into_each_component = Vec::with_capacity(fields.len());

        for (index, field) in fields.iter().enumerate() {
            let field_name = FieldName::new(index, field.ident.as_ref(), &field.ty);
            let member = &field_name.member;

            if field.bundle {
                num_components_from_bundles.push(quote_spanned! {
                    field_name.span => ::quasar_ecs::Bundle::num_components(&self.#member)
                });

                for_each_component.push(quote_spanned!{
                    field_name.span => ::quasar_ecs::Bundle::for_each_component(&self.#member, &mut callback);
                });

                into_each_component.push(quote_spanned!{
                    field_name.span => ::quasar_ecs::Bundle::into_each_component(self.#member, &mut callback);
                });
            }
            else {
                num_components_from_components += 1;

                for_each_component.push(quote_spanned!{
                    field_name.span => ::quasar_ecs::__private::ForEachComponent::call(&mut callback, &self.#member);
                });

                into_each_component.push(quote_spanned!{
                    field_name.span => ::quasar_ecs::__private::IntoEachComponent::call(&mut callback, self.#member);
                });
            }
        }

        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics ::quasar_ecs::Bundle for #ident #ty_generics #where_clause {
                fn num_components(&self) -> usize {
                    #num_components_from_components + #(#num_components_from_bundles)+*
                }

                fn for_each_component<C: ::quasar_ecs::__private::ForEachComponent>(&self, mut callback: C) {
                    #(#for_each_component)*
                }

                fn into_each_component<C: ::quasar_ecs::__private::IntoEachComponent>(self, mut callback: C) {
                    #(#into_each_component)*
                }
            }
        })
    }
}
