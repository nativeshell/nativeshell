use proc_macro2::{Ident, TokenStream};
use proc_macro_error::{Diagnostic, Level};
use quote::{format_ident, quote};
use syn::{Attribute, DataEnum, DataStruct, FieldsNamed, Variant};

use crate::{
    attributes::{
        parse_enum_attributes, parse_enum_variant_attributes, parse_field_attributes,
        parse_struct_attributes, EnumAttributes, StringWithSpan, StructAttributes,
    },
    case::RenameRule,
    rename_field, rename_variant,
};

fn insert_fields(
    target: &Ident,
    prefix: Option<Ident>,
    fields_named: &FieldsNamed,
    rename_rule: &RenameRule,
) -> TokenStream {
    struct Field {
        string: String,
        field: TokenStream,
    }
    let mut fields = Vec::<Field>::new();

    for field in &fields_named.named {
        let ident = field.ident.clone().unwrap();
        let attributes = parse_field_attributes(&field.attrs);
        if attributes.skip {
            continue;
        }
        let string = rename_field(
            &format!("{}", ident),
            rename_rule,
            &attributes.rename.map(|a| a.value.clone()),
        );
        let field_access = if let Some(prefix) = &prefix {
            quote! { #prefix.#ident }
        } else {
            quote! { #ident }
        };
        let token_stream = if attributes.skip_if_empty {
            quote! {
                if (&&::nativeshell_core::derive_internal::Wrap(& #field_access)).is_none() == false {
                    #target.push( ( #string.into(), #field_access.into() ) );
                }
            }
        } else {
            quote! {
                #target.push( ( #string.into(), #field_access.into() ) );
            }
        };
        fields.push(Field {
            string,
            field: token_stream,
        });
    }

    // Sort fields now (compile time) so that we don't need to do it in ValueTupleList
    fields.sort_by(|a, b| a.string.cmp(&b.string));

    let fields: Vec<TokenStream> = fields.iter().map(|f| f.field.clone()).collect();

    quote! {
        #(
            #fields
        )*
    }
}

pub struct FromEnum {
    name: Ident,
    attributes: EnumAttributes,
}

impl FromEnum {
    pub fn new(name: Ident, attributes: Vec<Attribute>) -> Self {
        Self {
            name,
            attributes: parse_enum_attributes(&attributes),
        }
    }

    pub fn process(self, data: DataEnum) -> TokenStream {
        let variants: Vec<TokenStream> = data
            .variants
            .into_iter()
            .filter_map(|v| self.enum_variant(v))
            .collect();
        let name = self.name;
        quote! {
            match value {
                #(
                    #name::#variants,
                )*
                _ => {
                    // For skipped variants. Not ideal but we can't report errors here
                    Value::Null
                }
            }
        }
    }

    fn enum_variant(&self, v: Variant) -> Option<TokenStream> {
        let attributes = parse_enum_variant_attributes(&v.attrs);
        if attributes.skip {
            return None;
        }
        let ident = v.ident;
        let ident_as_string = self.variant_ident_to_string(&ident, &attributes.rename);
        match v.fields {
            syn::Fields::Named(fields) => {
                let mut names = Vec::<Ident>::new();
                for field in &fields.named {
                    names.push(field.ident.clone().unwrap());
                }
                let target = format_ident!("__ns_vec");
                let create_vec = quote! {
                    let mut #target = Vec::<(::nativeshell_core::Value, ::nativeshell_core::Value)>::new();
                };
                let insert = insert_fields(&target, None, &fields, &attributes.rename_all);
                let epilogue = match (&self.attributes.tag, &self.attributes.content) {
                    (None, None) => quote! {
                        let value = ::nativeshell_core::Value::Map(#target.into());
                        #create_vec;
                        #target.push((#ident_as_string.into(), value));
                        ::nativeshell_core::Value::Map(#target.into())
                    },
                    (None, Some(_)) => panic!("Can't have content without tag"),
                    (Some(tag), None) => {
                        let tag = &tag.value;
                        quote! {
                            #target.push((#tag.into(), #ident_as_string.into()));
                            ::nativeshell_core::Value::Map(#target.into())
                        }
                    }
                    (Some(tag), Some(content)) => {
                        let tag = &tag.value;
                        let content = &content.value;
                        quote! {
                            let value = ::nativeshell_core::Value::Map(#target.into());
                            #create_vec;
                            #target.push((#tag.into(), #ident_as_string.into()));
                            #target.push((#content.into(), value));
                            ::nativeshell_core::Value::Map(#target.into())
                        }
                    }
                };
                Some(quote! {
                    #ident { #( #names, )* } => {
                        #create_vec;
                        #insert;
                        #epilogue
                    }
                })
            }
            syn::Fields::Unnamed(fields) => {
                if let Some(tag) = &self.attributes.tag {
                    if self.attributes.content.is_none() {
                        Diagnostic::spanned(
                            tag.span,
                            Level::Error,
                            format!(
                                "tag for unnamed enum variants (i.e. {}) is only supported \
                                if 'content' attribute is set as well",
                                ident
                            ),
                        )
                        .abort();
                    }
                }
                let idents: Vec<Ident> = (0..fields.unnamed.len())
                    .map(|i| format_ident!("v{}", i))
                    .collect();
                let value = if idents.len() == 1 {
                    let name = idents.first().unwrap();
                    quote! {
                        #name.into()
                    }
                } else {
                    quote! {
                        {
                            let mut vec = Vec::<::nativeshell_core::Value>::new();
                            #(
                                vec.push(#idents.into());
                            )*
                            ::nativeshell_core::Value::List(vec)
                        }
                    }
                };
                let insert = if let (Some(tag), Some(content)) =
                    (&self.attributes.tag, &self.attributes.content)
                {
                    let tag = &tag.value;
                    let content = &content.value;
                    quote! {
                        v.push((#tag.into(), #ident_as_string.into()));
                        v.push((#content.into(), value));
                    }
                } else {
                    quote! {
                        v.push((#ident_as_string.into(), value));
                    }
                };
                Some(quote! {
                    #ident ( #( #idents, )* ) => {
                        let value = #value;
                        let mut v = Vec::<(::nativeshell_core::Value, ::nativeshell_core::Value)>::new();
                        #insert
                        ::nativeshell_core::Value::Map(v.into())
                    }
                })
            }
            syn::Fields::Unit => {
                let result = if let Some(tag) = &self.attributes.tag {
                    // { 'tag': 'enumName' }
                    let tag = &tag.value;
                    quote! {
                        let mut v = Vec::<(::nativeshell_core::Value, ::nativeshell_core::Value)>::new();
                        v.push((#tag.into(), value));
                        ::nativeshell_core::Value::Map(v.into())
                    }
                } else {
                    // just 'enumName'
                    quote! {
                        value
                    }
                };
                Some(quote! {
                    #ident => {
                        let value = ::nativeshell_core::Value::String(#ident_as_string.into());
                        #result
                    }
                })
            }
        }
    }

    fn variant_ident_to_string(&self, ident: &Ident, r: &Option<StringWithSpan>) -> String {
        rename_variant(
            &format!("{}", ident),
            &self.attributes.rename_all,
            &r.as_ref().map(|s| s.value.clone()),
        )
    }
}

pub struct FromStruct {
    name: Ident,
    attributes: StructAttributes,
}

impl FromStruct {
    pub fn new(name: Ident, attributes: Vec<Attribute>) -> Self {
        Self {
            name,
            attributes: parse_struct_attributes(&attributes),
        }
    }

    pub fn process(self, data: DataStruct) -> TokenStream {
        match data.fields {
            syn::Fields::Named(fields) => {
                let target = format_ident!("__ns_vec");
                let insert = insert_fields(
                    &target,
                    Some(format_ident!("value")),
                    &fields,
                    &self.attributes.rename_all,
                );
                quote! {
                    let mut #target = Vec::<(::nativeshell_core::Value, ::nativeshell_core::Value)>::new();
                    #insert;
                    ::nativeshell_core::Value::Map(#target.into())
                }
            }
            syn::Fields::Unnamed(fields) => {
                let idents: Vec<syn::Index> = (0..fields.unnamed.len())
                    .map(|i| syn::Index::from(i))
                    .collect();

                if idents.len() == 1 {
                    let name = idents.first().unwrap();
                    quote! {
                        value.#name.into()
                    }
                } else {
                    quote! {
                        let mut vec = Vec::<::nativeshell_core::Value>::new();
                        #(
                            vec.push(value.#idents.into());
                        )*
                        ::nativeshell_core::Value::List(vec)
                    }
                }
            }
            syn::Fields::Unit => {
                Diagnostic::spanned(
                    self.name.span(),
                    Level::Error,
                    "unit structs are not supported".into(),
                )
                .abort();
            }
        }
    }
}
