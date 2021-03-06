use std::convert::TryFrom;

use quote::quote;
use syn::export::TokenStream2;
use syn::{Fields, ItemEnum};

use crate::attribute_helpers::{contains_initialize_with, contains_skip};

pub fn enum_de(input: &ItemEnum) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let generics = &input.generics;
    let init_method = contains_initialize_with(&input.attrs)?;
    let mut variant_arms = TokenStream2::new();
    let mut deserializable_field_types = TokenStream2::new();
    for (variant_idx, variant) in input.variants.iter().enumerate() {
        let variant_idx = u8::try_from(variant_idx).expect("up to 256 enum variants are supported");
        let variant_ident = &variant.ident;
        let mut variant_header = TokenStream2::new();
        match &variant.fields {
            Fields::Named(fields) => {
                for field in &fields.named {
                    let field_name = field.ident.as_ref().unwrap();
                    if contains_skip(&field.attrs) {
                        variant_header.extend(quote! {
                            #field_name: Default::default(),
                        });
                    } else {
                        let field_type = &field.ty;
                        deserializable_field_types.extend(quote! {
                            #field_type: borsh::BorshDeserialize,
                        });

                        variant_header.extend(quote! {
                            #field_name: borsh::BorshDeserialize::deserialize(buf)?,
                        });
                    }
                }
                variant_header = quote! { { #variant_header }};
            }
            Fields::Unnamed(fields) => {
                for field in fields.unnamed.iter() {
                    if contains_skip(&field.attrs) {
                        variant_header.extend(quote! { Default::default(), });
                    } else {
                        let field_type = &field.ty;
                        deserializable_field_types.extend(quote! {
                            #field_type: borsh::BorshDeserialize,
                        });

                        variant_header
                            .extend(quote! { borsh::BorshDeserialize::deserialize(buf)?, });
                    }
                }
                variant_header = quote! { ( #variant_header )};
            }
            Fields::Unit => {}
        }
        variant_arms.extend(quote! {
            #variant_idx => #name::#variant_ident #variant_header ,
        });
    }
    let variant_idx = quote! {
        let variant_idx: u8 = borsh::BorshDeserialize::deserialize(buf)?;
    };
    if let Some(method_ident) = init_method {
        Ok(quote! {
            impl #generics borsh::de::BorshDeserialize for #name #generics where  #deserializable_field_types {
                fn deserialize(buf: &mut &[u8]) -> std::result::Result<Self, std::io::Error> {
                    #variant_idx
                    let mut return_value = match variant_idx {
                        #variant_arms
                        _ =>
                        return Err(std::io::Error::new(
                                   std::io::ErrorKind::InvalidInput,
                                   format!("Unexpected variant index: {:?}", variant_idx),
                                  )),
                    };
                    return_value.#method_ident();
                    Ok(return_value)
                }
            }
        })
    } else {
        Ok(quote! {
            impl #generics borsh::de::BorshDeserialize for #name #generics where  #deserializable_field_types {
                fn deserialize(buf: &mut &[u8]) -> std::result::Result<Self, std::io::Error> {
                    #variant_idx
                    let return_value = match variant_idx {
                        #variant_arms
                        _ =>
                        return Err(std::io::Error::new(
                                   std::io::ErrorKind::InvalidInput,
                                   format!("Unexpected variant index: {:?}", variant_idx),
                                  )),
                    };
                    Ok(return_value)
                }
            }
        })
    }
}
