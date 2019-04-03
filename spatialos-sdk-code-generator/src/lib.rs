use crate::schema_bundle::*;
use heck::SnekCase;
use proc_macro2::{Span, TokenStream};
use proc_quote::*;
use std::collections::{BTreeMap, HashMap};
use syn::Ident;

pub mod schema_bundle;

static NESTED_ITEMS_MODULE_NAME: &str = "nested_items";

/// Context for the current code generation process.
///
/// Contains the schema bundle and configuration options to be used in code
/// generation. This struct is passed down through the code generation process so
/// that types can easily reference each other.
#[derive(Debug, Clone, Copy)]
#[allow(non_snake_case)]
pub struct Context<'a> {
    pub bundle: &'a SchemaBundleV1,
    pub dependencies: &'a HashMap<&'static str, &'static str>,

    spatialos_sdk: &'a TokenStream,
    Component: &'a TokenStream,
    ComponentId: &'a TokenStream,
    SchemaComponentData: &'a TokenStream,
    SchemaComponentUpdate: &'a TokenStream,
    SchemaCommandRequest: &'a TokenStream,
    SchemaCommandResponse: &'a TokenStream,
    SchemaObject: &'a TokenStream,
    TypeConversion: &'a TokenStream,
}

/// Generates the Rust types from a bundle's schema information.
///
/// # Parameters
///
/// * `bundle` - The contents of the schema bundle file generated by the schema compiler.
/// * `package` - The name of the current package for which to generate code. `bundle`
///   will contain schema information for the current package *and all dependencies*,
///   so you must specify the current package name in order to select which package to
///   generate.
/// * `dependencies` - The mapping from the name of a schema package to the Rust path.
///   Used to construct paths in the generated code to correctly reference types in
///   external crates. Note that you MUST include the current package in the mapping.
// TODO: Create a proper error type to return.
pub fn generate(
    bundle: &SchemaBundle,
    package: &str,
    dependencies: &HashMap<&'static str, &'static str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let null_span = proc_macro2::Span::call_site();

    let spatialos_sdk = if package == "improbable" {
        quote! { crate }
    } else {
        // TODO: Handle the case that the crate renamed the `spatialos_sdk` dependency.
        // This likely means allowing the calling code to specify the name of the
        // `spatialos_sdk` dependency.
        quote! { spatialos_sdk }
    };

    let default_module = Module {
        contents: quote! {
            use #spatialos_sdk::worker::component::inventory;
        },

        modules: Default::default(),
    };

    // Define variables for commonly-referenced types so that we don't have to type
    // out the fully-qualified paths every time.
    #[allow(non_snake_case)]
    let Component = &quote! { #spatialos_sdk::worker::component::Component };
    #[allow(non_snake_case)]
    let ComponentId = &quote! { #spatialos_sdk::worker::component::ComponentId };
    #[allow(non_snake_case)]
    let SchemaComponentData =
        &quote! { #spatialos_sdk::worker::internal::schema::SchemaComponentData };
    #[allow(non_snake_case)]
    let SchemaComponentUpdate =
        &quote! { #spatialos_sdk::worker::internal::schema::SchemaComponentUpdate };
    #[allow(non_snake_case)]
    let SchemaCommandRequest =
        &quote! { #spatialos_sdk::worker::internal::schema::SchemaCommandRequest };
    #[allow(non_snake_case)]
    let SchemaCommandResponse =
        &quote! { #spatialos_sdk::worker::internal::schema::SchemaCommandResponse };
    #[allow(non_snake_case)]
    let SchemaObject = &quote! { #spatialos_sdk::worker::internal::schema::SchemaObject };
    #[allow(non_snake_case)]
    let TypeConversion = &quote! { #spatialos_sdk::worker::component::TypeConversion };

    let context = Context {
        bundle: bundle.v1.as_ref().ok_or("Only v1 bundle is supported")?,
        dependencies,

        spatialos_sdk: &spatialos_sdk,
        Component,
        ComponentId,
        SchemaComponentData,
        SchemaComponentUpdate,
        SchemaCommandRequest,
        SchemaCommandResponse,
        SchemaObject,
        TypeConversion,
    };

    // Track all generated modules in a `BTreeMap` in order to get deterministic
    // iteration ordering, which is useful when doing diffs when testing.
    let mut modules = BTreeMap::new();

    context.bundle
        .component_definitions
        .iter()
        .filter(|def| def.identifier.package_name() == package)
        .for_each(|component_def| {
            let ident = Ident::new(&component_def.identifier.name, null_span);

            let submodule_name = component_def.identifier.name.to_snek_case();
            let submodule_ident = Ident::new(&submodule_name, null_span);

            let struct_definition = match &component_def.data_definition {
                ComponentDataDefinition::Inline(fields) => quote_struct(
                    &component_def.identifier,
                    &fields,
                    context,
                ),

                ComponentDataDefinition::TypeReference(type_reference) => {
                    let ty_ref = type_reference.quotable(context);
                    quote! {
                        #[derive(Debug, Clone)]
                        pub struct #ident(#ty_ref);

                        impl #TypeConversion for #ident {
                            fn from_type(input: &#SchemaObject) -> Result<Self, String> {
                                Ok(Self(
                                    <#ty_ref as #TypeConversion>::from_type(input)?
                                ))
                            }

                            fn to_type(input: &Self, output: &mut #SchemaObject) -> Result<(), String> {
                                <#ty_ref as #TypeConversion>::to_type(&input.0, output)
                            }
                        }
                    }
                }
            };

            let component_id = component_def.component_id;
            let impls = quote! {
                impl #Component for #ident {
                    type Update = associated_data::#submodule_ident::Update;
                    type CommandRequest = associated_data::#submodule_ident::CommandRequest;
                    type CommandResponse = associated_data::#submodule_ident::CommandResponse;

                    const ID: #ComponentId = #component_id;

                    fn from_data(data: &#SchemaComponentData) -> Result<Self, String> {
                        <Self as #TypeConversion>::from_type(&data.fields())
                    }

                    fn from_update(_update: &#SchemaComponentUpdate) -> Result<Self::Update, String> {
                        unimplemented!("Component::from_update")
                    }

                    fn from_request(_request: &#SchemaCommandRequest) -> Result<Self::CommandRequest, String> {
                        unimplemented!("Component::from_request")
                    }

                    fn from_response(_response: &#SchemaCommandResponse) -> Result<Self::CommandResponse, String> {
                        unimplemented!("Component::from_response")
                    }

                    fn to_data(data: &Self) -> Result<#SchemaComponentData, String> {
                        let mut serialized_data = #SchemaComponentData::new(Self::ID);
                        <Self as #TypeConversion>::to_type(data, &mut serialized_data.fields_mut())?;
                        Ok(serialized_data)
                    }

                    fn to_update(_update: &Self::Update) -> Result<#SchemaComponentUpdate, String> {
                        unimplemented!()
                    }

                    fn to_request(_request: &Self::CommandRequest) -> Result<#SchemaCommandRequest, String> {
                        unimplemented!()
                    }

                    fn to_response(_response: &Self::CommandResponse) -> Result<#SchemaCommandResponse, String> {
                        unimplemented!()
                    }

                    fn get_request_command_index(_request: &Self::CommandRequest) -> u32 {
                        unimplemented!()
                    }

                    fn get_response_command_index(_response: &Self::CommandResponse) -> u32 {
                        unimplemented!()
                    }
                }

                impl #spatialos_sdk::worker::component::ComponentData<#ident> for #ident {
                    fn merge(&mut self, update: <#ident as #Component>::Update) {
                        unimplemented!();
                    }
                }

                inventory::submit!(#spatialos_sdk::worker::component::VTable::new::<#ident>());
            };

            let module_path = component_def.identifier.module_path();
            let module = get_submodule(&mut modules, module_path, &default_module);
            module.contents.append_all(struct_definition);
            module.contents.append_all(impls);

            // Generate definitions for associated types.
            let update_type = {
                let fields = match &component_def.data_definition {
                    ComponentDataDefinition::Inline(fields) => fields,

                    ComponentDataDefinition::TypeReference(type_reference) =>
                        &context.bundle.get_referenced_type(type_reference).field_definitions
                };

                let fields = fields.iter().map(|field_def| {
                    let ident = Ident::new(&field_def.identifier.name, null_span);
                    let ty = field_def.ty.quotable(context);
                    quote! {
                        #ident: Option<#ty>
                    }
                });

                quote! {
                    #[derive(Debug, Clone, Default)]
                    pub struct Update {
                        #( pub #fields ),*
                    }
                }
            };

            let command_types = quote! {
                #[derive(Debug, Clone)]
                pub enum CommandRequest {}

                #[derive(Debug, Clone)]
                pub enum CommandResponse {}
            };

            // Put the associated data types for the component in a submodule named
            // after the component.
            let submodule = get_submodule(
                &mut module.modules,
                std::iter::once("associated_data".into()).chain(std::iter::once(submodule_name)),
                &default_module,
            );
            submodule.contents.append_all(update_type);
            submodule.contents.append_all(command_types);
        });

    context
        .bundle
        .type_definitions
        .iter()
        .filter(|def| def.identifier.package_name() == package)
        .for_each(|type_def| {
            let generated =
                quote_struct(&type_def.identifier, &type_def.field_definitions, context);

            let module_path = type_def.identifier.module_path();
            let module = get_submodule(&mut modules, module_path, &default_module);
            module.contents.append_all(generated);
        });

    context
        .bundle
        .enum_definitions
        .iter()
        .filter(|def| def.identifier.package_name() == package)
        .for_each(|enum_def| {
            let ident = syn::Ident::new(&enum_def.identifier.name, null_span);
            let values = &enum_def.value_definitions;

            let generated = quote! {
                #[derive(Debug, Clone)]
                pub enum #ident {
                    #( #values ),*
                }

                impl #TypeConversion for #ident {
                    fn from_type(input: &#SchemaObject) -> Result<Self, String> {
                        unimplemented!()
                    }

                    fn to_type(input: &Self, output: &mut #SchemaObject) -> Result<(), String> {
                        unimplemented!()
                    }
                }
            };

            let module_path = enum_def.identifier.module_path();
            let module = get_submodule(&mut modules, module_path, &default_module);
            module.contents.append_all(generated);
        });

    // Generate the code for each of the modules.
    let module_names = modules.keys().map(|name| Ident::new(name, null_span));
    let modules = modules.values();
    let raw_generated = quote! {
        #(
            #[allow(unused_imports)]
            pub mod #module_names {
                #modules
            }
        )*
    }
    .to_string();

    // Attempt to use rustfmt to format the code in order to help with debugging.
    // If this fails for any reason, we simply default to using the unformatted
    // code. This ensures that code generation can still work even if the user
    // doesn't have rustfmt installed.
    let generated = rustfmt(raw_generated.clone()).unwrap_or(raw_generated);

    // If the `RUST_SPATIALOS_CODEGEN_DEBUG` environment variable is set to "1", spit
    // out a Rust source file containing the generated code for easier debugging.
    let generate_debug_file = std::env::var("RUST_SPATIALOS_CODEGEN_DEBUG")
        .map(|val| val == "1")
        .unwrap_or(false);
    if generate_debug_file {
        let _ = std::fs::write(format!("{}.rs", package), &generated);
    }

    Ok(generated)
}

pub fn rustfmt<S>(module: S) -> Result<String, Box<dyn std::error::Error>>
where
    S: Into<String>,
{
    use std::{
        io::Write,
        process::{Command, Stdio},
        str,
    };

    let mut child = Command::new("rustfmt")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let stdin = child.stdin.as_mut().unwrap();
    stdin.write_all(module.into().as_bytes())?;

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err("Failed to format the code I guess".into());
    }

    let formatted = str::from_utf8(&output.stdout[..])?.into();
    Ok(formatted)
}

#[derive(Debug, Clone)]
struct Module {
    /// The items included in the module.
    contents: TokenStream,

    // NOTE: We track the modules in a `BTreeMap` because it provides deterministic
    // iterator ordering. Deterministic ordering in the generated code is useful
    // when diffing changes and debugging the generated code.
    modules: BTreeMap<String, Module>,
}

impl ToTokens for Module {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for (ident, module) in &self.modules {
            let ident = syn::Ident::new(ident, proc_macro2::Span::call_site());
            tokens.append_all(quote! {
                pub mod #ident {
                    #module
                }
            });
        }

        tokens.append_all(self.contents.clone());
    }
}

fn get_submodule<'a, I, S>(
    modules: &'a mut BTreeMap<String, Module>,
    path: I,
    default_module: &Module,
) -> &'a mut Module
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    // Extract the first module based on the first segment of the path.
    let mut iterator = path.into_iter();
    let first = iterator.next().expect("`path` was empty");
    let mut module = modules
        .entry(first.into())
        .or_insert_with(|| default_module.clone());

    // Iterate over the remaining segments of the path, getting or creating all
    // intermediate modules.
    for ident in iterator {
        module = module
            .modules
            .entry(ident.into())
            .or_insert_with(|| default_module.clone());
    }

    module
}

/// Quotes the definition for a struct type and creates the `TypeConversion` impl for it.
///
/// This logic is shared between inline component definitions and struct type definitions.
fn quote_struct<'a>(
    ident: &'a Identifier,
    fields: &'a [FieldDefinition],
    context: Context<'a>,
) -> TokenStream {
    let input = Ident::new("input", Span::call_site());
    let ident = ident.ident();
    let field_defs = fields.iter().map(|field| field.quotable(context));
    // let serialize_fields = fields.iter().map(|field| field.quote_serialize_impl());
    let deserialize_fields = fields
        .iter()
        .map(|field| field.quote_deserialize_impl(&input, context));
    let spatialos_sdk = context.spatialos_sdk;
    #[allow(non_snake_case)]
    let TypeConversion = context.TypeConversion;
    #[allow(non_snake_case)]
    let SchemaObject = context.SchemaObject;

    quote! {
        #[derive(Debug, Clone)]
        pub struct #ident {
            #( pub #field_defs ),*
        }

        impl #TypeConversion for #ident {
            fn from_type(#input: &#SchemaObject) -> Result<Self, String> {
                use #spatialos_sdk::worker::internal::schema::{SchemaPrimitiveField, SchemaBytesField, SchemaStringField, SchemaObjectField};

                Ok(Self {
                    #( #deserialize_fields, )*
                })
            }

            fn to_type(input: &Self, output: &mut #SchemaObject) -> Result<(), String> {
                unimplemented!()
            }
        }
    }
}
