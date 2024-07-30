use proc_macro::TokenStream;
use quote::quote;
use quote::ToTokens;
use syn::Generics;

extern crate alloc;

#[proc_macro_attribute]
pub fn byte_string(attr: TokenStream, input: TokenStream) -> TokenStream {
	let ast = syn::parse(input).expect("Cannot parse source");

	impl_byte_string_derive(attr, &ast)
}

enum SupportedType {
	Array(syn::Type),
	Vec,
	BoundedVec(syn::Type),
}

fn impl_byte_string_derive(attr: TokenStream, ast: &syn::DeriveInput) -> TokenStream {
	let name = &ast.ident;
	let generics = &ast.generics;

	let data = &ast.data;

	let syn::Data::Struct(ds) = data else {
		return quote! { compile_error!("byte_string is only defined for structs") }.into();
	};

	let ty = match &ds.fields {
		syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
			let field = fields.unnamed.first().unwrap();
			field.ty.clone()
		},
		_ => return quote! { compile_error!("byte_string has to have one field") }.into(),
	};

	let supported_type = match &ty {
		syn::Type::Array(_arr) => SupportedType::Array(ty),
		syn::Type::Path(path) if !path.path.segments.is_empty() => {
			let type_name = &path.path.segments.last().unwrap().ident;
			if type_name == "Vec" {
				SupportedType::Vec
			} else if type_name == "BoundedVec" {
				SupportedType::BoundedVec(ty)
			} else {
				return quote! { compile_error!("byte_string needs to wrap an array or (bounded) vec") }.into();
			}
		},
		_ => {
			return quote! { compile_error!("byte_string needs to wrap an array or (bounded) vec") }
				.into()
		},
	};

	let mut gen = quote! {
		#ast
	};

	for attr in attr.into_iter().map(|attr| attr.to_string()) {
		let chunk: Box<dyn ToTokens> = match attr.as_str() {
			"debug" => Box::from(gen_debug(name, generics)),
			"hex_serialize" => Box::from(gen_hex_serialize(name, generics)),
			"hex_deserialize" => Box::from(gen_hex_deserialize(name, &supported_type, generics)),
			"from_num" => Box::from(gen_from_num(name, &supported_type, generics)),
			"from_bytes" => Box::from(gen_from_bytes(name, &supported_type, generics)),
			"decode_hex" => Box::from(gen_from_hex(name, &supported_type, generics)),
			"to_hex_string" => Box::from(gen_to_hex(name, generics)),
			"as_ref" => Box::from(gen_as_ref(name, generics)),
			"," => continue,
			_other => return quote! { compile_error!("Incorrect byte_string option") }.into(),
		};

		chunk.to_tokens(&mut gen)
	}

	gen.into()
}

fn gen_debug(name: &syn::Ident, generics: &Generics) -> impl ToTokens {
	let format_str = format!("{}({{hex}})", name);
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
	quote! {
		impl #impl_generics core::fmt::Debug for #name #ty_generics #where_clause {
			fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
				let hex = sp_core::bytes::to_hex(&self.0, true);
				return f.write_str(&alloc::format!(#format_str));
			}
		}
	}
}

fn gen_hex_serialize(name: &syn::Ident, generics: &Generics) -> impl ToTokens {
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

	quote! {
		impl #impl_generics serde::Serialize for #name #ty_generics #where_clause {
			fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
			where
				S: serde::Serializer,
			{
				let s = sp_core::bytes::to_hex(self.0.as_slice(), false);
				serializer.serialize_str(s.as_str())
			}
		}
	}
}

fn gen_hex_deserialize(
	name: &syn::Ident,
	ty: &SupportedType,
	generics: &Generics,
) -> impl ToTokens {
	let type_params = generics.params.clone().into_iter();
	let (_, ty_generics, where_clause) = generics.split_for_impl();

	let created = match ty {
		SupportedType::Array(ty) => {
			quote! {
				#name(<#ty>::try_from(inner)
					  .map_err(|err| serde::de::Error::custom("Can't deserialize"))?)
			}
		},
		SupportedType::BoundedVec(ty) => {
			quote! {
				#name(<#ty>::try_from(inner)
					  .map_err(|_| serde::de::Error::custom("Invalid length"))?)
			}
		},
		_ => quote! { #name(inner) },
	};
	quote! {
		impl<'de, #(#type_params),* > serde::Deserialize < 'de > for #name #ty_generics #where_clause {
			fn deserialize < D > (deserializer: D) -> Result < Self, D::Error >
			where
				D: serde::Deserializer < 'de >,
			{
				use alloc::string::ToString;
				let str = <alloc::string::String>::deserialize(deserializer)?;
				let inner = sp_core::bytes::from_hex(&str).map_err( | err | serde::de::Error::custom(err.to_string()))?;
				Ok(#created)
			}
		}
	}
}

fn gen_from_num(name: &syn::Ident, ty: &SupportedType, generics: &Generics) -> impl ToTokens {
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

	match ty {
		SupportedType::Array(ty) => quote! {
			impl #impl_generics From<u64> for #name #ty_generics #where_clause {
				fn from(n: u64) -> Self {
					let mut ret = <#ty>::default();
					let ret_len = ret.len();
					let bytes = n.to_be_bytes();
					ret[(ret_len-bytes.len())..].copy_from_slice(&bytes);
					#name(ret)
				}
			}
		},
		_ => quote! {
			impl #impl_generics From<u64> for #name #ty_generics #where_clause {
				fn from(n: u64) -> Self {
					#name(n.to_be_bytes().to_vec())
				}
			}
		},
	}
}

fn gen_from_bytes(name: &syn::Ident, ty: &SupportedType, generics: &Generics) -> impl ToTokens {
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

	match ty {
		SupportedType::Array(ty) => quote! {
			impl<'a> TryFrom<&'a [u8]> for #name {
				type Error = <#ty as TryFrom<&'a [u8]>>::Error;
				fn try_from(bytes: &'a [u8]) -> Result<Self, Self::Error> {
					Ok(#name(bytes.try_into()?))
				}
			}
		},
		_ => quote! {
			impl #impl_generics From<&[u8]> for #name #ty_generics #where_clause {
				fn from(bytes: &[u8]) -> Self {
					#name(bytes.clone().to_vec())
				}
			}
		},
	}
}

fn gen_from_hex(name: &syn::Ident, ty: &SupportedType, generics: &Generics) -> impl ToTokens {
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

	let decode_hex = match ty {
		SupportedType::Array(ty) => quote! {
			pub fn decode_hex(s: &str) -> Result<Self, &'static str> {
				let value = <#ty>::try_from(sp_core::bytes::from_hex(s).map_err(|_| "Cannot decode bytes from hex string")?)
					.map_err(|_| "Invalid length")?;
				Ok(#name(value))
			}
		},
		_ => quote! {
			pub fn decode_hex(s: &str) -> Result<Self, &'static str> {
				Ok(#name(sp_core::bytes::from_hex(s).map_err(|_| "Cannot decode bytes from hex string")?))
			}
		},
	};

	quote! {
		impl #impl_generics #name #ty_generics #where_clause {
			#decode_hex

			pub fn from_hex_unsafe(s: &str) -> Self {
				Self::decode_hex(s).unwrap()
			}

		}

		impl #impl_generics alloc::str::FromStr for #name #ty_generics #where_clause {
			type Err = &'static str;
			fn from_str(s: &str) -> Result<Self, Self::Err> {
				Self::decode_hex(s)
			}
		}
	}
}

fn gen_to_hex(name: &syn::Ident, generics: &Generics) -> impl ToTokens {
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

	quote! {
		impl #impl_generics #name #ty_generics #where_clause {
			pub fn to_hex_string(&self) -> String {
				sp_core::bytes::to_hex(&self.0, false)
			}
		}
	}
}

fn gen_as_ref(name: &syn::Ident, generics: &Generics) -> impl ToTokens {
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

	quote! {
		impl #impl_generics AsRef<[u8]> for #name #ty_generics #where_clause {
			fn as_ref(&self) -> &[u8] {
				&self.0
			}
		}
	}
}
