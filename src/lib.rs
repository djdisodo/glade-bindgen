use syn::Ident;
use quote::{quote, format_ident};
use std::path::{PathBuf, Path};
use xml::EventReader;
use xml::reader::XmlEvent;
use std::fs::File;
use proc_macro2::TokenStream as TokenStream2;
use convert_case::{Case, Casing};
use std::io::Write;
use std::process::Command;
use regex::Regex;
use lazy_static::lazy_static;
use std::borrow::Cow;

const README: &[u8] = include_bytes!("README.txt");

const HEAD_ANNOTATION: &[u8] = include_bytes!("head_annotation.rs");
const BUILD_SCRIPT_HEAD_ANNOTATION: &[u8] = include_bytes!("build_script_head_annotation.rs");

pub fn generate_bind_build_script<T: AsRef<Path>>(directory_path: T, static_value: bool) {
	generate_bind_recursive(&directory_path, true, false, static_value);
	let path = PathBuf::from(directory_path.as_ref());
	{
		let mut path = path.clone();
		path.push("README_glade-bindgen.txt");
		std::fs::write(&path, README).unwrap();
	}
	{
		let mut path = path.clone();
		path.push(".gitignore");
		std::fs::write(&path, "*.rs").unwrap();
	}
	println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
}

pub fn generate_bind_recursive<T: AsRef<Path>>(directory_path: T, build_script: bool, format: bool, static_value: bool) -> bool { //true if need to include in tree
	let read_dir = std::fs::read_dir(&directory_path).unwrap();
	let mut modules = Vec::new();
	let mut generated_token_streams = Vec::new();
	for x in read_dir {
		let dir_entry = x.unwrap();
		let file_name = dir_entry.file_name().into_string().unwrap();
		if file_name == "." || file_name == ".."{
			continue;
		}
		if dir_entry.path().is_dir() {
			if generate_bind_recursive(dir_entry.path(), build_script, format, static_value) {
				modules.push(file_name);
			}
		} else if let Some(name) = remove_ui_extension(&file_name) {
			let name = format_ident!("{}", name.to_case(Case::Pascal));
			let file = File::open(dir_entry.path()).unwrap();
			generated_token_streams.push(generate_bind(name, file, file_name, static_value));
		}
	}

	if modules.is_empty() && generated_token_streams.is_empty() {
		return false;
	}
	let mut token_stream = TokenStream2::new();
	for x in modules {
		let module = format_ident!("{}", x);
		token_stream.extend(quote! {
			pub mod #module;
		});
	}
	for x in generated_token_streams {
		token_stream.extend::<TokenStream2>(x);
	}

	let mut mod_path = PathBuf::from(directory_path.as_ref());
	mod_path.push("mod.rs");

	{
		let mut mod_file = File::create(&mod_path).unwrap();
		mod_file.write_all(if build_script {
			BUILD_SCRIPT_HEAD_ANNOTATION
		} else {
			HEAD_ANNOTATION
		}).unwrap();
		mod_file.write_all(token_stream.to_string().as_bytes()).unwrap();
	}

	if format {
		Command::new("rustfmt").args(&[std::fs::canonicalize(mod_path).unwrap()]).output()
			.expect("failed to format");
	}

	true
}

pub fn generate_bind<T: AsRef<Path>>(name: Ident, file: File, file_include_dir: T, static_value: bool) -> TokenStream2 {
	let mut objects = TokenStream2::new();
	let mut objects_new = TokenStream2::new();

	let parser = EventReader::new(file);
	for e in parser {
		match e {
			Ok(XmlEvent::StartElement { name, attributes, .. }) => {
				if &name.local_name == "object" {
					let id = attributes.iter().find(| attr | attr.name.local_name == "id");
					if let Some(id) = id {
						let class = attributes.iter().find(| attr | attr.name.local_name == "class");
						if let Some(class) = class {
							let class = class.value.to_owned();
							let class_ident = format_ident!("{}", class.replace("Gtk", ""));
							let id = id.value.to_owned();
							let id_ident = format_ident!("{}", &id);
							objects.extend::<TokenStream2>(quote!{
								pub #id_ident: gtk::#class_ident,
							});
							objects_new.extend::<TokenStream2>(quote! {
								#id_ident: gtk::prelude::BuilderExtManual::object(&builder, #id).unwrap(),
							})
						}
					}
				}
			}
			Err(e) => {
				println!("Error: {}", e);
				break;
			}
			_ => {}
		}
	}

	let include_str = format_ident!("include_str");
	let thread_local = format_ident!("thread_local");

	let include = file_include_dir.as_ref().to_str().unwrap();

	let static_value_token_stream : TokenStream2 = if static_value {
		quote! {
			#thread_local! {
				static OBJECTS: std::sync::Mutex<Option<std::rc::Rc<#name>>> = std::sync::Mutex::new(None);
			}

			pub fn get() -> std::rc::Rc<Self> {
				Self::OBJECTS.with(| objects | {
					let mut objects = objects.lock().unwrap();
					if objects.is_none() {
						objects.replace(std::rc::Rc::new(Self::new()));
					}
					objects.as_ref().unwrap().clone()
				})
			}
		}
	} else {
		TokenStream2::new()
	};

	let token_stream = quote!{
		#[allow(dead_code)]
		pub struct #name {
			#objects
		}

		impl #name {
			#static_value_token_stream

			pub fn new() -> Self {
				let builder = gtk::Builder::from_string(#include_str!(#include));
				Self {
					#objects_new
				}
			}
		}
	};
	token_stream
}
/*
struct Args(Ident, LitStr);

impl syn::parse::Parse for Args {
	fn parse<'a>(input: &'a ParseBuffer<'a>) -> Result<Self, syn::Error> {
		let type1 = input.parse()?;
		input.parse::<Token![,]>()?;
		let type2 = input.parse()?;
		Ok(Args(type1, type2))
	}
}

#[proc_macro]
pub fn include_glade(args: TokenStream) -> TokenStream {
	let args: Args = syn::parse(args).unwrap();
	let span = args.0.span();
	let name = args.0;
	let file_include_dir = args.1.value();
	let mut file_path = span.unwrap().source_file().path().parent().unwrap().to_owned();
	file_path.push(&file_include_dir);
	let file = File::open(file_path).unwrap();
	generate_bind(name, file, file_include_dir)
}
*/

/* Remove the UI extension of a file, and return its bare name */
fn remove_ui_extension<'a>(file_name: &'a str) -> Option<Cow<'a, str>> {
	lazy_static! {
		static ref UI_FILE_REGEX: Regex = Regex::new(r"^(.*)\.glade|\.ui").unwrap();
	}
	UI_FILE_REGEX.is_match(&file_name).then(|| UI_FILE_REGEX.replace(&file_name, "$1"))
}

#[test]
fn test_remove_ui_extension() {
	assert_eq!("foo", remove_ui_extension("foo.ui").unwrap());
	assert_eq!("bar", remove_ui_extension("bar.glade").unwrap());
	assert_eq!(None, remove_ui_extension("foo.rs"));
}