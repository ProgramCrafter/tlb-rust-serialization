extern crate proc_macro;
use syn::{parse_macro_input, DeriveInput, Data, Expr, Fields, Ident, ItemEnum, Meta, MetaList, Lit, spanned::Spanned};
use quote::{quote_spanned, quote, ToTokens};
use proc_macro2::Span;

use std::collections::HashMap;

type OldTokenStream = proc_macro::TokenStream;
type V2TokenStream = proc_macro2::TokenStream;



/// Creates function that allows to serialize values of given type to TON cells.
/// 
/// # Example
/// 
/// Struct:
/// ```no_run
/// #[tlb_serializable(u 4 3bit, workchain, hash_high, hash_low)]
/// pub struct Address {
///     workchain: u8,      hash_high: u128,      hash_low: u128
/// }
/// ```
/// 
/// Produced code:
/// ```no_run
/// {
///     let mut result = ::std::vec![];
///     result.push("u 4 3bit".to_owned());
///     {
///         let mut s_field = crate::ton::CellSerialize::serialize(&self.workchain);
///         result.append(&mut s_field);
///     }
///     {
///         let mut s_field = crate::ton::CellSerialize::serialize(&self.hash_high);
///         result.append(&mut s_field);
///     }
///     {
///         let mut s_field = crate::ton::CellSerialize::serialize(&self.hash_low);
///         result.append(&mut s_field);
///     }
///     result
/// }
/// ```
fn create_serialization_code(attr: &str, struct_fields: &Fields, self_ref: bool) -> V2TokenStream {
    match attr {
        // -------------------------------------------------------------
        // Processing `#[tlb_serializable(__fundamental_varuint16)]`
        "__fundamental_varuint16" => {
            let Fields::Unnamed(ref fields) = struct_fields else {
                panic!("Fundamental VarUint16 struct must consist of unnamed fields");
            };
            assert!(fields.unnamed.len() == 1, "Fundamental VarUint16 struct must have exactly one field");
            
            assert!(self_ref);
            return quote!{{
                let value = self.0 as u128;
                let bytes_required = 128 / 8 - value.leading_zeros() / 8;
                assert!(bytes_required <= 15, "VarUint16 overflow");
                result = ::std::vec![
                    ::std::format!("u {bytes_required} 4bit"), 
                    ::std::format!("u {value} {}bit", bytes_required * 8)
                ];
            }}
        },
        
        // -------------------------------------------------------------
        // Normal serialization - almost-TL-B for user-defined types.
        _ => {
            // Loading fields list, obtaining string->ident mapping with relevant spans
            let Fields::Named(ref fields) = struct_fields else {
                panic!("For unambiguous parsing, normal structs must consist of named fields");
            };
            let mut field_spans: HashMap<String, (Ident, Span)> = HashMap::new();
            for field in fields.named.iter() {
                let id = field.ident.clone().expect(&format!("unnamed field"));
                field_spans.insert(id.to_string(), (id, field.span()));
            }
            
            // ------------------------------------------------------------------------------------
            // Mapping each part of serialization TL-B to block of code that stores value into cell
            let serializations = attr.split(",").map(|part_whitespaced| {
                let part = part_whitespaced.trim();
                if part.is_empty() {
                    quote!{}
                } else if part.starts_with("u ") {
                    quote! { 
                        result.push(#part.to_owned());
                    }
                } else {
                    let (name, span) = &field_spans[part];
                    
                    if self_ref {
                        quote_spanned! {span.clone()=>{
                            let mut s_field = crate::ton::CellSerialize::serialize(&self.#name);
                            result.append(&mut s_field);
                        }}
                    } else {
                        quote_spanned! {span.clone()=>{
                            let mut s_field = crate::ton::CellSerialize::serialize(#name);
                            result.append(&mut s_field);
                        }}
                    }
                }
            });
            
            // ------------------------------------------------------------------------------------
            // Constructing function of all those code chunks
            quote!{{
                #(#serializations)*
            }}
        }
    }
}

fn create_serialization_code_struct(attr: &str, struct_wrap: &Data) -> V2TokenStream {
    match *struct_wrap {
        Data::Union(_) => unimplemented!("union serialization is not defined yet"),
        Data::Enum(_) => panic!("please use #[tlb_enum_serializable] for enums"),
        Data::Struct(ref data) => {
            create_serialization_code(attr, &data.fields, true)
        }
    }
}


/// Creates impl of crate::ton::CellSerialize for struct the attribute is attached to.
/// Uses [create_serialization_code](fn.create_serialization_code.html) internally.
///
/// # Examples
/// 
/// ```no_run
/// #[tlb_serializable(__fundamental_varuint16)]
/// struct Coins(u128);
/// 
/// #[derive(Default)]
/// #[tlb_serializable(u 4 3bit, workchain, hash_high, hash_low)]
/// pub struct Address {
///     workchain: u8,      hash_high: u128,      hash_low: u128
/// }
/// ```
#[proc_macro_attribute]
pub fn tlb_serializable(attr: OldTokenStream, mut item: OldTokenStream) -> OldTokenStream {
    let struct_item = item.clone();
    let input: DeriveInput = parse_macro_input!(struct_item);
    let name = input.ident;
    
    let serializers = create_serialization_code_struct(&attr.to_string(), &input.data);
    item.extend(OldTokenStream::from(quote! {
        impl crate::ton::CellSerialize for #name {
            fn serialize(&self) -> ::std::vec::Vec<::std::string::String> {
                let mut result : ::std::vec::Vec<::std::string::String> = ::std::vec![];
                #serializers
                result
            }
        }
    }));
    
    item
}


#[derive(Debug)] enum TlbPrefix {Wanted(String), NotWanted}

/// Creates impl of crate::ton::CellSerialize for struct the attribute is attached to.
/// Uses [create_serialization_code](fn.create_serialization_code.html) internally.
///
/// # Examples
/// 
/// ```no_run
/// #[allow(non_camel_case_types)]
/// #[tlb_enum_serializable]
/// #[tlb_assert_unsafe(items_prefixes_nonoverlap)]
/// enum CommonMsgInfo {
///     #[tlb_item_serializable(u 0 1bit, ihr_disabled, bounce, bounced, src, dest,
///                             value, ihr_fee, fwd_fee, created_lt, created_at)]
///     int_msg_info {
///         ihr_disabled: bool,
///         bounce: bool,
///         bounced: bool,
///         src: ton::Address,
///         dest: ton::Address,
///         value: ton::CurrencyCollection,
///         ihr_fee: ton::Coins,
///         fwd_fee: ton::Coins,
///         created_lt: u64,
///         created_at: u32
///     }
/// }
/// 
/// #[tlb_enum_serializable]
/// #[repr(u32)]
/// enum Boc {
///     #[tlb_item_serializable()] Empty{}  = 0,
///     #[tlb_item_serializable()] Normal{} = 0xb5eec792,
/// }
/// ```
#[proc_macro_attribute]
pub fn tlb_enum_serializable(_: OldTokenStream, item: OldTokenStream) -> OldTokenStream {
    let mut input: ItemEnum = parse_macro_input!(item);
    
    // For prefix definition, either #[repr(u?)] or #[tlb_assert_unsafe(items_prefixes_nonoverlap)] is required.
    let mut need_prefix: Option<TlbPrefix> = None;
    input.attrs.retain(|attr| {
        if attr.path().is_ident("tlb_assert_unsafe") {
            let Meta::List(MetaList {tokens: ref tokens_assert, ..}) = attr.meta else {
                panic!("#[tlb_assert_unsafe] attribute must have argument with the specific assertion");
            };
            let assertion = tokens_assert.to_string();
            if assertion == "items_prefixes_nonoverlap" {
                assert!(need_prefix.is_none());
                need_prefix = Some(TlbPrefix::NotWanted);
                false
            } else {
                println!("Unknown assertion {assertion:?}");
                true
            }
        } else if attr.path().is_ident("repr") {
            assert!(need_prefix.is_none(), "Two #[repr] attributes on enum are not supported");
            let Meta::List(MetaList {tokens: ref tokens_type, ..}) = attr.meta else {
                panic!("#[repr] attribute must have argument specifying the type");
            };
            need_prefix = Some(TlbPrefix::Wanted(tokens_type.to_string()));
            true    // we retain #[repr] attribute for use with Rust
        } else {
            true
        }
    });
    let need_prefix: TlbPrefix = need_prefix.expect("Don't know how to differentiate tags of the enum");
    let name = input.ident.clone();
    
    
    // Generating code for each variant of the enum
    let mut variant_index = 0;
    let variant_generators: Vec<V2TokenStream> = input.variants.iter_mut().map(|variant| {
        let mut store = None;
        variant.attrs.retain(|attr| {
            if !attr.path().is_ident("tlb_item_serializable") {return true;}
            let Meta::List(MetaList {tokens: ref tokens_tlb, ..}) = attr.meta else {
                panic!("#[tlb_item_serializable] attribute must have argument with the specific serialization");
            };
            let tlb = tokens_tlb.to_string();
            
            assert!(store.is_none(), "multiple serialization definitions found");
            store = Some(create_serialization_code(&tlb, &variant.fields, false));
            false
        });
        let store = store.expect(&format!("serialization definition for variant {} is required", variant.ident));
        
        // Enum discriminant
        if let Some((_, Expr::Lit(ref idx))) = variant.discriminant {
            if let Lit::Int(ref discriminant) = idx.lit {
                variant_index = discriminant.base10_parse::<u64>().unwrap();
            }
        };
        let vident = &variant.ident;
        
        let fields_unpacker: Vec<_> = variant.fields.iter().map(|field| {
            let id = field.ident.clone().expect("unnamed field in enum");
            quote!{ #id, }
        }).collect();
        
        let store_tag = match need_prefix {
            TlbPrefix::NotWanted => quote! {},     // ^^^ result: Vec<String>
            TlbPrefix::Wanted(ref t) => {
                let s = &t[1..];
                quote! {
                    result.push(::std::format!("u {} {}bit", #variant_index, #s));
                }
            },
        };
        
        variant_index += 1;
        
        quote! {
            #name::#vident {#(#fields_unpacker)*} => {
                #store_tag
                #store
            }
        }
    }).collect();
    
    
    let mut result: OldTokenStream = input.to_token_stream().into();
    result.extend(OldTokenStream::from(quote! {
        impl crate::ton::CellSerialize for #name {
            fn serialize(&self) -> ::std::vec::Vec<::std::string::String> {
                let mut result = ::std::vec![];
                match &self {
                    #(#variant_generators)*
                }
                result
            }
        }
    }));
    
    // println!("{}", result.to_string());
    
    result
}
