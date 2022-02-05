use proc_macro2::{Span, TokenStream};
use proc_macro_error::{Diagnostic, Level};
use quote::quote;
use syn::{
    spanned::Spanned, Attribute, DataEnum, DataStruct, Fields, FieldsNamed, FieldsUnnamed, Ident,
    Type,
};

use crate::{
    attributes::{
        parse_enum_attributes, parse_enum_variant_attributes, parse_field_attributes,
        parse_struct_attributes, EnumAttributes, StringWithSpan, StructAttributes,
    },
    case::RenameRule,
    rename_field, rename_variant,
};

pub struct TryIntoEnum {
    name: Ident,
    attributes: EnumAttributes,
}

impl TryIntoEnum {
    pub fn new(name: Ident, attributes: Vec<Attribute>) -> Self {
        Self {
            name,
            attributes: parse_enum_attributes(&attributes),
        }
    }

    pub fn process(self, data: DataEnum) -> TokenStream {
        if self.attributes.tag.is_none() {
            self.process_no_tag(&data)
        } else {
            self.process_tag(&data)
        }
    }

    fn process_tag(&self, data: &DataEnum) -> TokenStream {
        let tag = self.attributes.tag.clone().unwrap().value;
        let (strings, variants) = self.process_variants(data, true);
        let extract_value = match &self.attributes.content {
            Some(content) => {
                let content = &content.value;
                quote! {
                    let mut value = Value::Null;
                    for row in map {
                        if let ::nativeshell_core::Value::String(content_value) = row.0 {
                            if content_value == #content {
                                value = row.1;
                                break;
                            }
                        }
                    }
                }
            }
            None => quote! {
                let value = ::nativeshell_core::Value::Map(map);
            },
        };
        quote! {
            match value {
                ::nativeshell_core::Value::Map(map) => {
                    let mut tag_value = Option::<String>::None;
                    for row in map.iter() {
                        if let (::nativeshell_core::Value::String(tag),
                                ::nativeshell_core::Value::String(value)) = (&row.0, &row.1) {
                            if tag == #tag {
                                tag_value = Some(value.clone());
                            }
                        }
                    }
                    #extract_value;
                    let tag_value = tag_value.ok_or_else(|| Self::Error::OtherError("Couldn't get tag value".into()))?;
                    match tag_value.as_str() {
                        #(
                            #strings => { #variants; },
                        )*
                        (other) => return ::core::result::Result::Err(Self::Error::OtherError(format!("Unexpected enum value {}", other))),
                    }
                }
                other => {
                    return ::core::result::Result::Err(Self::Error::OtherError(format!("Can not deserialize {:?} as enum", other)));
                }
            }
        }
    }

    fn process_no_tag(&self, data: &DataEnum) -> TokenStream {
        let unit_enums = self.process_unit_enums(data);
        let (strings, variants) = self.process_variants(data, false);
        quote! {
            #unit_enums
            match value {
                ::nativeshell_core::Value::Map(map) => {
                    let row = map.into_iter().next().ok_or(Self::Error::OtherError("Unexpected empty map".into()))?;
                    let key : String = row.0.try_into().map_err(|e|Self::Error::OtherError("Enum type must be a String".into()))?;
                    let value = row.1;
                    match key.as_str() {
                        #(
                            #strings => { #variants; },
                        )*
                        (other) => return ::core::result::Result::Err(Self::Error::OtherError(format!("Unexpected enum value {}", other))),
                    }
                }
                other => {
                    return ::core::result::Result::Err(Self::Error::OtherError(format!("Can not deserialize {:?} as enum", other)));
                }
            }
        }
    }

    fn process_variants(
        &self,
        data: &DataEnum,
        allow_unit: bool,
    ) -> (Vec<String>, Vec<TokenStream>) {
        let mut strings = Vec::<String>::new();
        let mut variants = Vec::<TokenStream>::new();
        for variant in &data.variants {
            let attributes = parse_enum_variant_attributes(&variant.attrs);
            if attributes.skip {
                continue;
            }
            let ident = &variant.ident;
            if let syn::Fields::Unit = &variant.fields {
                if !allow_unit {
                    continue;
                } else {
                    variants.push(quote! {
                        return Ok(Self::#ident);
                    });
                }
            } else {
                variants.push(process_struct(
                    &variant.span(),
                    &variant.fields,
                    Some(ident),
                    attributes.rename_all,
                ));
            }
            strings.push(self.variant_ident_to_string(&variant.ident, &attributes.rename));
        }
        (strings, variants)
    }

    fn process_unit_enums(&self, data: &DataEnum) -> TokenStream {
        let mut variants = Vec::<Ident>::new();
        let mut strings = Vec::<String>::new();
        for variant in &data.variants {
            if let syn::Fields::Unit = &variant.fields {
                let attributes = parse_enum_variant_attributes(&variant.attrs);
                if attributes.skip {
                    continue;
                }
                variants.push(variant.ident.clone());
                strings.push(self.variant_ident_to_string(&variant.ident, &attributes.rename));
            }
        }
        let enum_name = &self.name;
        quote! {
            if let ::nativeshell_core::Value::String(string) = value {
                #(
                    if string == #strings {
                        return ::core::result::Result::Ok(#enum_name::#variants);
                    }
                )*
                return ::core::result::Result::Err(Self::Error::OtherError(format!("Unknown enum value {:?}", string)));
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

fn process_struct(
    span: &Span,
    fields: &Fields,
    constructor_suffix: Option<&Ident>,
    rename_rule: RenameRule,
) -> TokenStream {
    match fields {
        Fields::Named(named) => process_struct_named(named, constructor_suffix, rename_rule),
        Fields::Unnamed(unnamed) => process_struct_unnamed(unnamed, constructor_suffix),
        Fields::Unit => Diagnostic::spanned(
            span.clone(),
            Level::Error,
            "unit structs are not supported".into(),
        )
        .abort(),
    }
}

fn process_struct_unnamed(
    unnamed: &FieldsUnnamed,
    constructor_suffix: Option<&Ident>,
) -> TokenStream {
    let constructor = if let Some(suffix) = constructor_suffix {
        quote! { Self:: #suffix}
    } else {
        quote! { Self }
    };

    if unnamed.unnamed.len() == 1 {
        let field = unnamed.unnamed.first().unwrap();
        let ty = &field.ty;
        quote! {
            return Ok(#constructor ( {
                let mut res = std::option::Option::<#ty>::None;
                (&mut &mut ::nativeshell_core::derive_internal::Wrap(&mut res)).assign(value)?;
                res.unwrap()
            } ));
        }
    } else {
        let rows: Vec<TokenStream> = unnamed.unnamed.iter()
            .map(|field| {
                let ty= &field.ty;
                quote! {
                    {
                        let mut res = std::option::Option::<#ty>::None;
                        (&mut &mut ::nativeshell_core::derive_internal::Wrap(&mut res)).assign(
                            iter.next().ok_or_else(||Self::Error::OtherError("Missing value".into()))?
                        )?;
                        res.unwrap()
                    }
                }
            })
            .collect();
        quote! {
            match value {
                ::nativeshell_core::Value::List(entries) => {
                    let mut iter = entries.into_iter();
                    return Ok(#constructor(
                        #(
                            #rows,
                        )*
                    ));
                }
                _=> {
                    return Err(Self::Error::OtherError("Converting into unnamed requires Value::List.".into()))
                }
            }
        }
    }
}

fn process_struct_named(
    named: &FieldsNamed,
    constructor_suffix: Option<&Ident>,
    rename_rule: RenameRule,
) -> TokenStream {
    let mut fields = Vec::<Ident>::new();
    let mut strings = Vec::<String>::new();
    let mut types = Vec::<Type>::new();
    let mut err_missing_field = Vec::<String>::new();

    let mut skip_fields = Vec::<Ident>::new();

    let constructor = if let Some(suffix) = constructor_suffix {
        quote! { Self:: #suffix}
    } else {
        quote! { Self }
    };

    for field in &named.named {
        let attributes = parse_field_attributes(&field.attrs);

        if let Some(ident) = &field.ident {
            if attributes.skip {
                skip_fields.push(ident.clone());
                continue;
            }
            let string = rename_field(
                &format!("{}", ident),
                &rename_rule,
                &attributes.rename.map(|a| a.value),
            );
            err_missing_field.push(format!("Required field \"{}\" missing in value.", string));
            strings.push(string);
            fields.push(ident.clone());
            types.push(field.ty.clone());
        }
    }

    quote! {
        #(
            let mut #fields = ::std::option::Option::<#types>::None;
        )*;

        match value {
            ::nativeshell_core::Value::Map(entries) => {
                for __ns_e in entries {
                    let __ns_name = match __ns_e.0 {
                        nativeshell_core::Value::String(name) => name,
                        _ => return Err(Self::Error::OtherError("Key value must be a string.".into()))
                    };
                    #(
                        if __ns_name == #strings {
                            (&mut &mut ::nativeshell_core::derive_internal::Wrap(&mut #fields)).assign(__ns_e.1)?;
                            continue;
                        }
                    )*;
                }
            }
            _=> {
                return Err(Self::Error::OtherError("Converting into struct requires Value::Map.".into()))
            }
        }

        #(
            (&mut &mut ::nativeshell_core::derive_internal::Wrap(&mut #fields)).set_optional_to_none();
        )*;

        let res = #constructor {
            #(
                #fields :  #fields.ok_or(Self::Error::OtherError(#err_missing_field.into()))?,
            )*
            #(
                #skip_fields : ::std::default::Default::default(),
            )*
        };
        return Ok(res);
    }
}

pub struct TryIntoStruct {
    name: Ident,
    attributes: StructAttributes,
}

impl TryIntoStruct {
    pub fn new(name: Ident, attributes: Vec<Attribute>) -> Self {
        Self {
            name,
            attributes: parse_struct_attributes(&attributes),
        }
    }

    pub fn process(self, data: DataStruct) -> TokenStream {
        process_struct(
            &self.name.span(),
            &data.fields,
            None,
            self.attributes.rename_all,
        )
    }
}
