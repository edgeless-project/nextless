// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

// Inspiration + Sources to Understand Macros:
// https://doc.rust-lang.org/book/ch19-06-macros.html
// https://users.rust-lang.org/t/proc-macro-cannot-use-item-as-ident-inside-quote-when-loop-through-vector-of-idents/85061/2
// https://www.rareskills.io/post/rust-attribute-derive-macro
// https://stackoverflow.com/questions/68415296/is-it-possible-to-write-a-file-containing-macros-gathered-data-at-compile-time
// https://github.com/bytecodealliance/wit-bindgen/tree/main/crates/guest-rust/macro
// https://users.rust-lang.org/t/acceptable-for-procedural-macros-to-write-outside-of-source-file/69295

use edgeless_function_core::PortMethod;
use quote::quote;

#[proc_macro]
pub fn generate(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parsed_ident: syn::Ident = syn::parse(input).unwrap();

    let dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let function_spec = std::fs::read_to_string(dir.join("function.json")).unwrap();
    let parsed_spec = edgeless_function_core::WorkflowSpecFunctionClass::parse(function_spec);

    let trait_base: String = parsed_spec
        .id
        .split('_')
        .map(|part| {
            // https://stackoverflow.com/a/69996191
            let mut s = part.to_string().to_lowercase();
            format!("{}{s}", s.remove(0).to_uppercase())
        })
        .collect::<Vec<String>>()
        .join("");

    let trait_name = quote::format_ident!("{}API", trait_base);

    let mut types = std::collections::HashMap::new();

    let mut cast_inputs = Vec::new();
    let mut call_inputs = Vec::new();

    let handlers : Vec<_> = parsed_spec.inputs.iter().map(|(key, val)| {
        let type_name = val.data_type.replace('.', "_").to_uppercase();
        let type_ident = quote::format_ident!("{}", type_name);
        types.entry(type_ident.clone()).and_modify(|(input, _output)| *input = true).or_insert((true, false));

        let feature = format!("input_{key}");

        match val.method {
            PortMethod::CAST => {
                let method_name = quote::format_ident!("handle_cast_{}", key.to_lowercase());
                let cloned_ident = type_ident.clone();
                cast_inputs.push(quote! {
                    #[cfg(feature = #feature)]
                    if port == #key {
                        let param = <<#parsed_ident as #trait_name>::#cloned_ident as edgeless_function_core::Deserialize>::deserialize(encoded_message);
                        <#parsed_ident as #trait_name>::#method_name(src.clone(), param);
                        return Ok(());
                    }

                });

                quote! {
                    fn #method_name(src : InstanceId, data: Self::#type_ident);
                }
            },
            PortMethod::CALL => {
                // assert!(val.return_data_type.is_some());

                let (return_type_ident, return_statement) = if let Some(rdt) = val.return_data_type.as_ref() {
                    let return_type_name = rdt.replace('.', "_").to_uppercase();
                    let return_type_ident = quote::format_ident!("{}", return_type_name);
                    types.entry(return_type_ident.clone()).and_modify(|(_input, output)| *output = true).or_insert((false, true));

                    let return_statement = quote! {
                        let serialized = <<#parsed_ident as #trait_name>::#return_type_ident as edgeless_function_core::Serialize>::serialize(&res);
                        return Ok(edgeless_function::CallRet::Reply(edgeless_function::owned_data::OwnedByteBuff::new_from_slice(serialized.as_ref())));
                    };

                    (Some(return_type_ident), return_statement)
                } else {
                    (None, quote! {
                        return edgeless_function::CallRet::NoReply;
                    })
                };



                let method_name = quote::format_ident!("handle_call_{}", key.to_lowercase());
                let cloned_ident = type_ident.clone();

                call_inputs.push(quote! {
                    #[cfg(feature = #feature)]
                    if port == #key {
                        let param = <<#parsed_ident as #trait_name>::#cloned_ident as edgeless_function_core::Deserialize>::deserialize(encoded_message);
                        let res = <#parsed_ident as #trait_name>::#method_name(src, param);
                        #return_statement
                    }
                });

                if let Some(return_type_ident) = return_type_ident {
                    quote! {
                        fn #method_name(_src : InstanceId, data: Self::#type_ident) -> Self::#return_type_ident;
                    }
                } else {
                    quote! {
                        fn #method_name(_src : InstanceId, data: Self::#type_ident) -> ();
                    }
                }

            }
        }

    }).collect();

    let output_handlers: Vec<_> = parsed_spec
        .outputs
        .iter()
        .map(|(output_id, output_spec)| {
            let type_name = output_spec.data_type.replace('.', "_").to_uppercase();
            let type_ident = quote::format_ident!("{}", type_name);
            types
                .entry(type_ident.clone())
                .and_modify(|(_input, output)| *output = true)
                .or_insert((false, true));

            let feature = format!("output_{output_id}");

            match output_spec.method {
                PortMethod::CAST => {
                    let handler_ident = quote::format_ident!("cast_{}", output_id);

                    quote! {
                        fn #handler_ident(payload: &<#parsed_ident as #trait_name>::#type_ident) {
                            #[cfg(feature = #feature)]
                            {
                                let serialized = <<#parsed_ident as #trait_name>::#type_ident as edgeless_function_core::Serialize>::serialize(payload);
                                cast(#output_id, serialized.as_ref());
                            }
                        }
                    }
                }
                PortMethod::CALL => {
                    let (return_type_ident, return_statement) = if let Some(rdt) = output_spec.return_data_type.as_ref() {
                        let return_type_name = rdt.replace('.', "_").to_uppercase();
                        let return_type_ident = quote::format_ident!("{}", return_type_name);
                        types
                            .entry(return_type_ident.clone())
                            .and_modify(|(input, _output)| *input = true)
                            .or_insert((true, false));

                        let return_statement = quote! {
                            if let edgeless_function::CallRet::Reply(val) = res {
                                return Ok(<<#parsed_ident as #trait_name>::#return_type_ident as edgeless_function_core::Deserialize>::deserialize(&val))
                            } else {
                                return Err(())
                            }
                        };

                        (Some(return_type_ident), return_statement)
                    } else {
                        (
                            None,
                            quote! {
                                return Ok(());
                            },
                        )
                    };

                    let handler_ident = quote::format_ident!("call_{}", output_id);
                    let rt = if let Some(return_type_ident) = return_type_ident  {
                        quote!{
                            Result<<#parsed_ident as #trait_name<'a>>::#return_type_ident, ()>
                        }
                    } else {
                        quote!{
                            Result<(), ()>
                        }
                    };
                    quote! {
                        fn #handler_ident<'a>(payload: &'a <#parsed_ident as #trait_name>::#type_ident) -> #rt {
                            #[cfg(feature = #feature)]
                            {
                                let serialized = <<#parsed_ident as #trait_name>::#type_ident as edgeless_function_core::Serialize>::serialize(payload);
                                let res = call(#output_id, serialized.as_ref());
                                #return_statement
                            }
                            #[cfg(not(feature = #feature))]
                            {
                                return Err(())
                            }
                        }
                    }
                }
            }
        })
        .collect();

    let quoted_types = types.iter().map(|(t, (input, output))| {
        let mut traits = Vec::new();

        if *input {
            traits.push(quote!(edgeless_function_core::Deserialize<'a>));
        }

        if *output {
            traits.push(quote!(edgeless_function_core::Serialize<'a>));
        }

        quote! {
            type #t : #(#traits)+* + 'a;
        }
    });

    quote! {


        #[cfg(not(target_arch = "wasm32"))]
        #[no_mangle]
        fn init<'a>(host_api: &'static mut dyn edgeless_actor_abi::HostApi<'static>) -> edgeless_actor_abi::GuestApi<'a> {
            unsafe { edgeless_function::interface_dynlib::HOST_API = Some(host_api) };

            let addr = edgeless_actor_abi::GuestApi {
                handle_cast: handle_cast,
                handle_call: handle_call,
                handle_init: handle_init,
                handle_stop: handle_stop,
            };
            core::hint::black_box(&addr.handle_call);
            core::hint::black_box(&addr.handle_cast);
            core::hint::black_box(&addr.handle_init);
            core::hint::black_box(&addr.handle_stop);
            addr
        }

        trait #trait_name<'a> {
            #(#quoted_types)*
            #(#handlers)*
            fn handle_internal(encoded_message: &[u8]);
            fn handle_init(payload: Option<&[u8]>, serialized_state: Option<&[u8]>);
            fn handle_stop();
        }

        #(#output_handlers)*

        //edgeless_actor_abi::HandleCast
        #[no_mangle]
        pub fn handle_cast(src: InstanceId, port: &str, encoded_message: &[u8]) -> edgeless_actor_abi::ActorResult<()> {
            #(#cast_inputs)*

            if port == "INTERNAL" {
                <#parsed_ident as #trait_name>::handle_internal(encoded_message);
                return Ok(());
            }

            Err(edgeless_actor_abi::ActorError::UndefinedPort)
        }

        //edgeless_actor_abi::HandleCall
        #[no_mangle]
        pub fn handle_call<'a>(src: InstanceId, port: &str, encoded_message: &[u8]) -> edgeless_actor_abi::ActorResult<edgeless_actor_abi::CallRet<'a>> {
            #(#call_inputs)*
            return Err(edgeless_actor_abi::ActorError::UndefinedPort);
        }

        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        pub unsafe extern "C" fn handle_cast_asm(
            node_id_ptr: *mut u8,
            component_id_ptr: *mut u8,
            port_ptr: *const u8,
            port_len: usize,
            payload_ptr: *mut u8,
            payload_len: usize,
        ) {
            let payload: &[u8] = core::slice::from_raw_parts(payload_ptr, payload_len);
            let instance_id = InstanceId {
                node_id: core::slice::from_raw_parts(node_id_ptr, 16).try_into().unwrap(),
                component_id: core::slice::from_raw_parts(component_id_ptr, 16).try_into().unwrap(),
            };

            let port: &str = core::str::from_utf8(core::slice::from_raw_parts(port_ptr, port_len)).unwrap();

            handle_cast(instance_id, port, payload).unwrap();
        }

        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        pub unsafe extern "C" fn handle_call_asm(
            node_id_ptr: *mut u8,
            component_id_ptr: *mut u8,
            port_ptr: *const u8,
            port_len: usize,
            payload_ptr: *mut u8,
            payload_len: usize,
            out_ptr_ptr: *mut *const u8,
            out_len_ptr: *mut usize,
        ) -> i32 {
            let payload: &[u8] = core::slice::from_raw_parts(payload_ptr, payload_len);

            let instance_id = InstanceId {
                node_id: core::slice::from_raw_parts(node_id_ptr, 16).try_into().unwrap(),
                component_id: core::slice::from_raw_parts(node_id_ptr, 16).try_into().unwrap(),
            };

            let port: &str = core::str::from_utf8(core::slice::from_raw_parts(port_ptr, port_len)).unwrap();

            let ret = handle_call(instance_id, port, payload).unwrap();

            let (ret, output_params) = match ret {
                edgeless_actor_abi::CallRet::NoReply => (0, None),
                edgeless_actor_abi::CallRet::Reply(reply) => (1, Some(edgeless_function::owned_data::OwnedByteBuff::new_from_slice(&reply).consume())),
                edgeless_actor_abi::CallRet::Err => (2, None),
            };
            if let (Some((output_ptr, output_len))) = output_params {
                *out_ptr_ptr = output_ptr;
                *out_len_ptr = output_len
            }
            ret
        }

        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        pub unsafe extern "C" fn handle_init_asm(
            payload_ptr: *mut u8,
            payload_len: usize,
            serialized_state_ptr: *mut u8,
            serialized_state_len: usize,
        ) {
            let payload: Option<&[u8]> = if payload_len > 0 {
                Some(core::slice::from_raw_parts(payload_ptr, payload_len))
            } else {
                None
            };

            let serialized_state = if serialized_state_len > 0 {
                Some(core::slice::from_raw_parts(serialized_state_ptr, serialized_state_len))
            } else {
                None
            };

            <#parsed_ident as #trait_name>::handle_init(payload, serialized_state);
        }

        //edgeless_actor_abi::HandleInit
        #[cfg(not(target_arch = "wasm32"))]
        #[no_mangle]
        pub fn handle_init(payload: Option<&[u8]>, serialized_state: Option<&[u8]>) -> edgeless_actor_abi::ActorResult<()> {
            <#parsed_ident as #trait_name>::handle_init(payload, serialized_state);
            Ok(())
        }

        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        pub extern "C" fn handle_stop_asm() {
            <#parsed_ident as #trait_name>::handle_stop();
        }

        //edgeless_actor_abi::HandleStop
        #[cfg(not(target_arch = "wasm32"))]
        #[no_mangle]
        fn handle_stop() -> edgeless_actor_abi::ActorResult<()> {
            <#parsed_ident as #trait_name>::handle_stop();
            Ok(())
        }

    }
    .into()
}
