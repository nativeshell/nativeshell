use proc_macro2::TokenStream;
use proc_macro_error::{proc_macro_error, Diagnostic, Level};
use syn::{
    Attribute, DataEnum, DeriveInput, Field, FieldsUnnamed, Ident, Lit, Path, Type, Variant,
};

use quote::{format_ident, quote};

mod attributes;
mod case;
mod from;

use attributes::*;
use from::*;

#[proc_macro_derive(TryFromValue, attributes(nativeshell))]
#[proc_macro_error]
pub fn try_from_value(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let mut fields = Vec::<Ident>::new();
    let mut strings = Vec::<String>::new();
    let mut types = Vec::<Type>::new();
    let mut err_missing_field = Vec::<String>::new();

    let mut skip_fields = Vec::<Ident>::new();

    match ast.data {
        syn::Data::Struct(st) => {
            for field in st.fields {
                let attributes = parse_field_attributes(&field.attrs);
                // panic!("ATTR: {:?}", attributes);
                if let Some(ident) = field.ident {
                    if attributes.skip {
                        skip_fields.push(ident);
                        continue;
                    }
                    let string = format!("{}", ident);
                    err_missing_field
                        .push(format!("Required field \"{}\" missing in value.", string));
                    strings.push(string);
                    fields.push(ident);
                    types.push(field.ty);
                }
            }
        }
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => todo!(),
    }

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let tokens = quote! {
        impl #impl_generics core::convert::TryFrom<nativeshell_core::Value> for #name #ty_generics #where_clause {
            type Error = nativeshell_core::TryFromError;
            fn try_from(value: Value) -> Result<Self, Self::Error> {
                use ::core::convert::TryInto;
                use ::nativeshell_core::derive_internal::Assign;
                #(
                    let mut #fields = ::std::option::Option::<#types>::None;
                )*;
                match value {
                    Value::Map(entries) => {
                        for e in entries {
                            let __ns_name = match e.0 {
                                nativeshell_core::Value::String(name) => name,
                                _ => return Err(Self::Error::OtherError("Key value must be a string."))
                            };
                            #(
                                if __ns_name == #strings {
                                    (&mut &mut ::nativeshell_core::derive_internal::Wrap(&mut #fields)).assign(e.1)?;
                                    continue;
                                }
                            )*;
                        }
                    }
                    _=> {
                        return Err(Self::Error::OtherError("Converting into struct requires Value::Map."))
                    }
                }

                #(
                    (&mut &mut ::nativeshell_core::derive_internal::Wrap(&mut #fields)).set_optional_to_none();
                )*;

                let res = Self {
                    #(
                        #fields :  #fields.ok_or(Self::Error::OtherError(#err_missing_field))?,
                    )*
                    #(
                        #skip_fields : ::std::default::Default::default(),
                    )*
                };
                Ok(res)
            }
        }
    };
    proc_macro::TokenStream::from(tokens)
}

#[proc_macro_derive(IntoValue, attributes(nativeshell))]
#[proc_macro_error]
pub fn into_value(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);
    let name = ast.ident;
    let token_stream = match ast.data {
        syn::Data::Struct(s) => FromStruct::new(name.clone(), ast.attrs).process(s),
        syn::Data::Enum(e) => FromEnum::new(name.clone(), ast.attrs).process(e),
        syn::Data::Union(_) => {
            Diagnostic::spanned(
                name.span(),
                Level::Error,
                "derive(IntoValue) is not supported for unions".into(),
            )
            .abort();
        }
    };

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let tokens = quote! {
        #[automatically_derived]
        impl #impl_generics From<#name #ty_generics> for ::nativeshell_core::Value #where_clause {
            fn from(value: #name #ty_generics) -> Self {
                #token_stream
            }
        }
    };
    proc_macro::TokenStream::from(tokens)
}
