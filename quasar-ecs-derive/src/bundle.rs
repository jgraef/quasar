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
        let mut component_types = Vec::with_capacity(fields.len());
        let mut from_components = Vec::with_capacity(fields.len());
        let mut from_components_constructor = Vec::with_capacity(fields.len());
        let mut into_components = Vec::with_capacity(fields.len());

        for (index, field) in fields.iter().enumerate() {
            let field_name = FieldName::new(index, field.ident.as_ref(), &field.ty);
            let member = &field_name.member;
            let var_name = &field_name.var;
            let field_ty = &field_name.ty;

            if field.bundle {
                num_components_from_bundles.push(quote_spanned! {
                    field_name.span => <#field_ty as ::quasar_ecs::Bundle>::NUM_COMPONENTS
                });

                component_types.push(quote_spanned!{
                    field_name.span => <#field_ty as ::quasar_ecs::Bundle>::component_types(&mut callback);
                });

                from_components.push(quote_spanned!{
                    field_name.span => let #var_name = <#field_ty as ::quasar_ecs::Bundle>::from_components(&mut callback);
                });

                into_components.push(quote_spanned!{
                    field_name.span => ::quasar_ecs::Bundle::into_each_component(self.#member, &mut callback);
                });
            }
            else {
                num_components_from_components += 1;

                component_types.push(quote_spanned!{
                    field_name.span => ::quasar_ecs::bundle_impl::ComponentTypesCallback::call(&mut callback, &self.#member);
                });

                from_components.push(quote_spanned!{
                    field_name.span => let #var_name = ::quasar_ecs::bundle_impl::FromComponentsCallback::call(&mut callback);
                });

                into_components.push(quote_spanned!{
                    field_name.span => ::quasar_ecs::bundle_impl::IntoComponentsCallback::call(&mut callback, self.#member);
                });
            }

            from_components_constructor.push(quote_spanned! {
                field_name.span => #member: #var_name,
            });
        }

        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics ::quasar_ecs::Bundle for #ident #ty_generics #where_clause {
                const NUM_COMPONENTS: usize = #num_components_from_components #(+ #num_components_from_bundles)*;

                fn component_types<C: ::quasar_ecs::bundle_impl::ComponentTypesCallback>(&self, mut callback: C) {
                    #(#component_types)*
                }

                fn from_components<C: ::quasar_ecs::bundle_impl::FromComponentsCallback>(self, mut callback: C) {
                    #(#from_components)*
                    Self {
                        #(#from_components_constructor)*
                    }
                }

                fn into_components<C: ::quasar_ecs::bundle_impl::IntoComponentsCallback>(self, mut callback: C) {
                    #(#into_components)*
                }
            }
        })
    }
}
