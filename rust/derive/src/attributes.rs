use convert_case::Case;
use proc_macro2::{Ident, Span};
use proc_macro_error::{Diagnostic, Level};
use syn::{spanned::Spanned, Attribute, Lit, Meta, NestedMeta, Path};

#[derive(Copy, Clone)]
pub struct Symbol(&'static str);

pub const NATIVESHELL: Symbol = Symbol("nativeshell");
pub const RENAME: Symbol = Symbol("rename");
pub const RENAME_ALL: Symbol = Symbol("rename_all");
pub const SKIP: Symbol = Symbol("skip");
pub const DEFAULT: Symbol = Symbol("default");
pub const TAG: Symbol = Symbol("tag");
pub const CONTENT: Symbol = Symbol("content");

impl PartialEq<Symbol> for Ident {
    fn eq(&self, word: &Symbol) -> bool {
        self == word.0
    }
}

impl<'a> PartialEq<Symbol> for &'a Ident {
    fn eq(&self, word: &Symbol) -> bool {
        *self == word.0
    }
}

impl PartialEq<Symbol> for Path {
    fn eq(&self, word: &Symbol) -> bool {
        self.is_ident(word.0)
    }
}

impl<'a> PartialEq<Symbol> for &'a Path {
    fn eq(&self, word: &Symbol) -> bool {
        self.is_ident(word.0)
    }
}

fn extract_nativeshell_meta(atts: &Vec<Attribute>) -> Vec<Meta> {
    let mut res = Vec::new();
    for attr in atts {
        let meta = attr.parse_meta().unwrap();
        if let Meta::List(list) = meta {
            if list.path == NATIVESHELL {
                for n in list.nested {
                    if let NestedMeta::Meta(meta) = n {
                        res.push(meta.clone())
                    }
                }
            }
        }
    }
    res
}

#[derive(Debug, Clone)]
pub struct StringWithSpan {
    pub value: String,
    pub span: Span,
}

#[derive(Debug, Default)]
pub struct EnumAttributes {
    pub tag: Option<StringWithSpan>,
    pub content: Option<StringWithSpan>,
    pub rename_all: Option<Case>,
}

#[derive(Debug, Default)]
pub struct EnumVariantAttribute {
    pub rename: Option<StringWithSpan>,
}

#[derive(Debug, Default)]
pub struct StructAttributes {
    pub rename_all: Option<Case>,
}

#[derive(Debug, Default)]
pub struct FieldAttributes {
    pub rename: Option<StringWithSpan>,
    pub skip: bool,
    pub default: bool,
}

fn str_from_lit(lit: &Lit, span: Option<Span>) -> StringWithSpan {
    match lit {
        Lit::Str(str) => StringWithSpan {
            value: str.value(),
            span: span.unwrap_or(str.span()),
        },
        lit => {
            Diagnostic::spanned(lit.span(), Level::Error, "Expected string literal".into()).abort();
        }
    }
}

fn case_from_lit(lit: &Lit) -> Case {
    match &lit {
        Lit::Str(str) => {
            let value = str.value();
            match value.as_str() {
                "lowercase" => Case::Lower,
                "UPPERCASE" => Case::Upper,
                "PascalCase" => Case::Pascal,
                "camelCase" => Case::Camel,
                "snake_case" => Case::Snake,
                "SCREAMING_SNAKE_CASE" => Case::ScreamingSnake,
                "kebab-case" => Case::Kebab,
                "SCREAMING-KEBAB-CASE" => Case::UpperKebab,
                _ => {
                    Diagnostic::spanned(
                        lit.span(),
                        Level::Error,
                        "Invalid case. Allowe values are: lowercase, UPPERCASE, \
                        PascalCase, camelCase, snake_case, SCREAMING_SNAKE_CASE, \
                        kebab-case, SCREAMING-KEBAB-CASE"
                            .into(),
                    )
                    .emit();
                    panic!();
                }
            }
        }
        lit => {
            Diagnostic::spanned(lit.span(), Level::Error, "Expected string literal".into()).emit();
            panic!();
        }
    }
}

pub fn parse_enum_attributes(attrs: &Vec<Attribute>) -> EnumAttributes {
    let mut res = EnumAttributes::default();
    let meta = extract_nativeshell_meta(attrs);
    for m in &meta {
        match m {
            Meta::NameValue(nv) => {
                if nv.path == TAG {
                    res.tag = Some(str_from_lit(&nv.lit, Some(nv.span())));
                } else if nv.path == CONTENT {
                    res.content = Some(str_from_lit(&nv.lit, Some(nv.span())));
                } else if nv.path == RENAME_ALL {
                    res.rename_all = Some(case_from_lit(&nv.lit));
                } else {
                    Diagnostic::spanned(nv.span(), Level::Error, "Unknown attribute".into())
                        .abort();
                }
            }
            _ => {
                Diagnostic::spanned(m.span(), Level::Error, "Unknown attribute".into()).abort();
            }
        }
    }
    if let Some(content) = &res.content {
        if res.tag.is_none() {
            Diagnostic::spanned(
                content.span,
                Level::Error,
                "Content attribute must only be used together with 'tag' attribute.".into(),
            )
            .abort();
        }
    }
    res
}

pub fn parse_enum_variant_attributes(attrs: &Vec<Attribute>) -> EnumVariantAttribute {
    let mut res = EnumVariantAttribute::default();
    let meta = extract_nativeshell_meta(attrs);
    for m in &meta {
        match m {
            Meta::NameValue(nv) => {
                if nv.path == RENAME {
                    res.rename = Some(str_from_lit(&nv.lit, Some(nv.span())))
                } else {
                    Diagnostic::spanned(nv.span(), Level::Error, "Unknown attribute".into()).emit();
                }
            }
            _ => {
                Diagnostic::spanned(m.span(), Level::Error, "Unknown attribute".into()).emit();
            }
        }
    }
    res
}

pub fn parse_struct_attributes(attrs: &Vec<Attribute>) -> StructAttributes {
    let mut res = StructAttributes::default();
    let meta = extract_nativeshell_meta(attrs);
    for m in &meta {
        match m {
            Meta::NameValue(nv) => {
                if nv.path == RENAME_ALL {
                    res.rename_all = Some(case_from_lit(&nv.lit));
                } else {
                    Diagnostic::spanned(nv.span(), Level::Error, "Unknown attribute".into())
                        .abort();
                }
            }
            _ => {
                Diagnostic::spanned(m.span(), Level::Error, "Unknown attribute".into()).abort();
            }
        }
    }
    res
}

pub fn parse_field_attributes(attrs: &Vec<Attribute>) -> FieldAttributes {
    let mut res = FieldAttributes::default();
    let meta = extract_nativeshell_meta(attrs);
    for m in &meta {
        match m {
            Meta::NameValue(nv) => {
                if nv.path == RENAME {
                    res.rename = Some(str_from_lit(&nv.lit, Some(nv.span())))
                } else {
                    Diagnostic::spanned(nv.span(), Level::Error, "Unknown attribute".into()).emit();
                }
            }
            Meta::Path(path) => {
                if path == DEFAULT {
                    res.default = true;
                } else if path == SKIP {
                    res.skip = true;
                } else {
                    Diagnostic::spanned(path.span(), Level::Error, "Unknown attribute".into())
                        .emit();
                }
            }
            _ => {
                Diagnostic::spanned(m.span(), Level::Error, "Unknown attribute".into()).emit();
            }
        }
    }
    res
}
