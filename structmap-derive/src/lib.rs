//! Implements the functionality to enable conversion between a struct type a map container type
//! in Rust through the use of a procedural macros.
#![recursion_limit = "128"]

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;

use quote::quote;
use syn::{Data, DeriveInput, Fields, Ident, Type};

use std::collections::BTreeMap;

/// Implements the functionality for converting entries in a BTreeMap into attributes and values of a
/// struct. It will consume a tokenized version of the initial struct declaration, and use code
/// generation to implement the `FromMap` trait for instantiating the contents of the struct.
//由编译器执行该函数？
/// BTreeMap -> Struct（预定义）
#[proc_macro_derive(FromMap)]
pub fn from_map(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);

    //解析预定义的Struct的所有字段～最终封装在 Vec<T, P>
    //将使用有名方式({})定义的Struct的字段名称(&Ident)，作为 Map的Keys(String)
    let fields = match ast.data {
        //Brace ～{}，对应有字段名称的定义方式——struct { k:v }
        //Parenthesis ～()，对应无字段名称（使用下标偏移）的定义方式——struct(a, b)
        Data::Struct(st) => st.fields,
        _ => panic!("Implementation Must Be A Struct"),
    };
    let idents: Vec<&Ident> = fields
        .iter()
        .filter_map(|field| field.ident.as_ref()) //根据 Field定义，其 ident成员为 Option<Ident>～使用元组方式定义的Struct，该字段为None，将被过滤
        .collect::<Vec<&Ident>>();
    let keys: Vec<String> = idents
        .clone()
        .iter()
        .map(|ident| ident.to_string())
        .collect::<Vec<String>>();

    //解析有名字段的类型～用以将GenericMap中被 Num类型封装的 Entry Value的值进行拆箱
    //[Ident { ident: "i64", span: #6 bytes(72..79) }, Ident { ident: "i64", span: #6 bytes(72..79) }, Ident { ident: "i64", span: #6 bytes(72..79) }]
    let typecalls: Vec<Ident> = fields
        .iter()
        .filter(|field| field.ident.as_ref().is_some())
        .map(|field| match field.ty.clone() {
            Type::Path(typepath) => {
                // "i64"
                let typename: String = quote! {#typepath}.to_string().to_lowercase();
                //Ident 的字符串名称以及关联的Span？
                Ident::new(&typename, Span::mixed_site())
            }
            _ => unimplemented!(),
        })
        .collect::<Vec<Ident>>();

    //获取待转换目标Struct的名称
    let name: &Ident = &ast.ident;//Ident { ident: "MCMonitor", span: #0 bytes(89..98) }
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    // start codegen of a generic or non-generic impl for the given struct using quasi-quoting
    let tokens = quote! {
        use structmap::value::Value;
        use structmap::{StringMap, GenericMap};
        //impl <T> FromMap for Structs<U> Where T: TraitBound
        impl #impl_generics FromMap for #name #ty_generics #where_clause {
            fn from_genericmap(mut hashmap: GenericMap) -> #name {//#name 对应Struct类型
                let mut settings = #name::default();
                #(//对keys中的每一项，分别获取Map 中的(K, V)项
                    match hashmap.entry(String::from(#keys)) {
                        ::std::collections::btree_map::Entry::Occupied(entry) => {//匹配解构出 entry
                            //由structmap 的value.rs可知～GenericMap的中的 Entry.Value，为类似 Num(I64(0))形式，需要经过对应item的类型方法（.i64()之类）进行转换，方可得到 Some(value)形式
                            let value = match entry.get().#typecalls() {
                                Some(val) => val,
                                None => panic!("Cannot Parse Out Map Entry")
                            };
                            //对Struct中的字段逐一赋值
                            settings.#idents = value;
                        },
                        _ => panic!("Cannot Parse Out Map Entry"),
                    }
                )*
                settings
            }
        }
    };
    TokenStream::from(tokens)
}

/// Struct -> BTreeMap
#[proc_macro_derive(ToMap, attributes(rename))]
pub fn to_map(input_struct: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input_struct as DeriveInput);

    let fields = match ast.data {
        Data::Struct(st) => st.fields,
        _ => panic!("Implementation Must Be A Struct"),
    };

    //Struct的字段，通过rename属性，可以设置与字段名称不相同的 Key至 Map
    let rename_map = parse_rename_attrs(&fields);

    let idents: Vec<&Ident> = fields
        .iter()
        .filter_map(|field| field.ident.as_ref())
        .collect::<Vec<&Ident>>();
    let keys: Vec<String> = idents
        .clone()
        .iter()
        .map(|ident| ident.to_string())
        .map(|name| match rename_map.contains_key(&name) {
            true => rename_map.get(&name).unwrap().clone(),
            false => name,
        })
        .collect::<Vec<String>>();

    let name: &Ident = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    // start codegen for to_hashmap functionality that converts a struct into a hashmap
    let tokens = quote! {
        impl #impl_generics ToMap for #name #ty_generics #where_clause {
            fn to_stringmap(mut input_struct: #name) -> structmap::StringMap {
                let mut map = structmap::StringMap::new();
                #(
                    map.insert(#keys.to_string(), input_struct.#idents.to_string());
                )*
                map
            }

            fn to_genericmap(mut input_struct: #name) -> structmap::GenericMap {
                let mut map = structmap::GenericMap::new();
                #(
                    map.insert(#keys.to_string(), structmap::value::Value::new(input_struct.#idents));
                )*
                map
            }
        }
    };
    TokenStream::from(tokens)
}

/// Helper method used to parse out any `rename` attribute definitions in a struct
/// marked with the ToMap trait, returning a mapping between the original field name
/// and the one being changed for later use when doing codegen.
fn parse_rename_attrs(fields: &Fields) -> BTreeMap<String, String> {
    let mut rename: BTreeMap<String, String> = BTreeMap::new();
    match fields {
        Fields::Named(_) => {
            // iterate over fields available and attributes
            for field in fields.iter() {
                for attr in field.attrs.iter() {
                    // parse original struct field name
                    let field_name = field.ident.as_ref().unwrap().to_string();
                    if rename.contains_key(&field_name) {
                        panic!("Cannot redefine field name multiple times");
                    }

                    // parse out name value pairs in attributes
                    // first get `lst` in #[rename(lst)]
                    match attr.parse_meta() {
                        Ok(syn::Meta::List(lst)) => {
                            // then parse key-value name
                            match lst.nested.first() {
                                Some(syn::NestedMeta::Meta(syn::Meta::NameValue(nm))) => {
                                    // check path to be = `name`
                                    let path = nm.path.get_ident().unwrap().to_string();
                                    if path != "name" {
                                        panic!("Must be `#[rename(name = 'VALUE')]`");
                                    }

                                    let lit = match &nm.lit {
                                        syn::Lit::Str(val) => val.value(),
                                        _ => {
                                            panic!("Must be `#[rename(name = 'VALUE')]`");
                                        }
                                    };
                                    rename.insert(field_name, lit);
                                }
                                _ => {
                                    panic!("Must be `#[rename(name = 'VALUE')]`");
                                }
                            }
                        }
                        _ => {
                            panic!("Must be `#[rename(name = 'VALUE')]`");
                        }
                    }
                }
            }
        }
        _ => {
            panic!("Must have named fields");
        }
    }
    rename
}
