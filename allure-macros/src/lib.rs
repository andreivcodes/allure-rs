//! Procedural macros for Allure test reporting.
//!
//! This crate provides the following macros:
//!
//! - `#[allure_test]` - Wraps a test function with Allure tracking
//! - `#[step]` - Marks a function as an Allure step
//! - `#[allure_suite]` - Groups tests in a module under a suite
//! - Metadata annotations: `#[epic]`, `#[feature]`, `#[story]`, `#[severity]`, etc.

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Attribute, FnArg, ItemFn, ItemMod, Lit, Meta, Pat, ReturnType, Token, Type,
};

/// Native Rust test attributes detected on the function
#[derive(Default)]
struct NativeTestAttrs {
    /// Whether #[should_panic] is present
    has_should_panic: bool,
    /// The expected panic message from #[should_panic(expected = "...")]
    should_panic_expected: Option<String>,
    /// Whether #[ignore] is present
    has_ignore: bool,
}

/// Extracts native Rust test attributes from the function's attributes
fn extract_native_test_attrs(attrs: &[Attribute]) -> NativeTestAttrs {
    let mut result = NativeTestAttrs::default();

    for attr in attrs {
        // Check for #[should_panic] or #[should_panic(expected = "...")]
        if attr.path().is_ident("should_panic") {
            result.has_should_panic = true;

            // Try to parse the expected message from #[should_panic(expected = "...")]
            if let Meta::List(meta_list) = &attr.meta {
                // Parse the tokens inside should_panic(...)
                let _ = meta_list.parse_nested_meta(|meta| {
                    if meta.path.is_ident("expected") {
                        let value: syn::LitStr = meta.value()?.parse()?;
                        result.should_panic_expected = Some(value.value());
                    }
                    Ok(())
                });
            }
        }

        // Check for #[ignore] or #[ignore = "reason"]
        if attr.path().is_ident("ignore") {
            result.has_ignore = true;
        }
    }

    result
}

/// Checks if the return type is a Result type
fn is_result_return(output: &ReturnType) -> bool {
    match output {
        ReturnType::Default => false,
        ReturnType::Type(_, ty) => {
            if let Type::Path(type_path) = ty.as_ref() {
                type_path
                    .path
                    .segments
                    .last()
                    .map(|seg| seg.ident == "Result")
                    .unwrap_or(false)
            } else {
                false
            }
        }
    }
}

/// Attribute macro that wraps a test function with Allure tracking.
///
/// # Examples
///
/// ```no_run
/// use allure_macros::allure_test;
///
/// #[allure_test]
/// fn test_basic() {
///     assert!(true);
/// }
///
/// #[allure_test("Custom test name")]
/// fn test_with_name() {
///     assert!(true);
/// }
/// ```
///
/// The macro also works with async tests when using `#[tokio::test]`.
#[proc_macro_attribute]
pub fn allure_test(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let custom_name = if attr.is_empty() {
        None
    } else {
        let name = parse_macro_input!(attr as Lit);
        match name {
            Lit::Str(s) => Some(s.value()),
            _ => None,
        }
    };

    expand_allure_test(input, custom_name)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn expand_allure_test(
    input: ItemFn,
    custom_name: Option<String>,
) -> syn::Result<proc_macro2::TokenStream> {
    let fn_name = &input.sig.ident;
    let fn_name_str = fn_name.to_string();
    let test_name = custom_name.unwrap_or_else(|| fn_name_str.clone());
    let visibility = &input.vis;
    let attrs = &input.attrs;
    let block = &input.block;
    let sig = &input.sig;
    let is_async = sig.asyncness.is_some();
    let output = &sig.output;
    let generics = &sig.generics;

    // Extract native test attributes
    let native_attrs = extract_native_test_attrs(attrs);

    // Extract metadata from attributes
    let metadata = extract_metadata_from_attrs(attrs);

    // Check if there's already a #[test] attribute
    let has_test_attr = attrs.iter().any(|attr| {
        attr.path().is_ident("test")
            || attr
                .path()
                .segments
                .last()
                .is_some_and(|seg| seg.ident == "test")
    });

    let test_attr = if has_test_attr {
        quote! {}
    } else {
        quote! { #[test] }
    };

    let setup_metadata = generate_metadata_setup(&metadata);

    // Check if this test returns a Result type
    let returns_result = is_result_return(output);

    if is_async {
        // For async tests - wrap in catch_unwind to handle panics
        Ok(quote! {
            #(#attrs)*
            #test_attr
            #visibility #sig {
                use ::allure_core::runtime::{set_context, take_context, TestContext};
                use ::allure_core::enums::Status;
                use ::allure_core::futures::FutureExt;

                // Build full name at runtime using module_path!()
                let full_name = concat!(module_path!(), "::", #fn_name_str);
                let ctx = TestContext::new(#test_name, full_name);
                set_context(ctx);

                #setup_metadata

                // Run the async body with panic catching
                let test_body = async #block;
                let panic_result = std::panic::AssertUnwindSafe(test_body).catch_unwind().await;

                match panic_result {
                    Ok(result) => {
                        // Test completed successfully
                        if let Some(mut ctx) = take_context() {
                            ctx.finish(Status::Passed, None, None);
                        }
                        result
                    }
                    Err(panic) => {
                        // Test panicked
                        let panic_msg = if let Some(s) = panic.downcast_ref::<&str>() {
                            Some(s.to_string())
                        } else if let Some(s) = panic.downcast_ref::<String>() {
                            Some(s.clone())
                        } else {
                            Some("Test panicked".to_string())
                        };
                        if let Some(mut ctx) = take_context() {
                            ctx.finish(Status::Failed, panic_msg, None);
                        }
                        std::panic::resume_unwind(panic);
                    }
                }
            }
        })
    } else if native_attrs.has_should_panic {
        // For #[should_panic] tests - invert the pass/fail logic
        let expected_check = if let Some(ref expected) = native_attrs.should_panic_expected {
            quote! {
                // Validate the panic message contains the expected string
                if panic_msg.as_ref().map(|m| m.contains(#expected)).unwrap_or(false) {
                    (Status::Passed, None)
                } else {
                    (Status::Failed, Some(format!(
                        "Panic message mismatch. Expected to contain '{}', got: {:?}",
                        #expected, panic_msg
                    )))
                }
            }
        } else {
            quote! {
                // Any panic is acceptable
                (Status::Passed, None)
            }
        };

        Ok(quote! {
            #(#attrs)*
            #test_attr
            #visibility fn #fn_name #generics () #output {
                use ::allure_core::runtime::{set_context, take_context, TestContext};
                use ::allure_core::enums::Status;

                // Build full name at runtime using module_path!()
                let full_name = concat!(module_path!(), "::", #fn_name_str);
                let ctx = TestContext::new(#test_name, full_name);
                set_context(ctx);

                #setup_metadata

                // Run the test body and catch panics
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| #block));

                // Extract panic message
                let panic_msg = match &result {
                    Ok(_) => None,
                    Err(e) => {
                        if let Some(s) = e.downcast_ref::<&str>() {
                            Some(s.to_string())
                        } else if let Some(s) = e.downcast_ref::<String>() {
                            Some(s.clone())
                        } else {
                            Some("Test panicked".to_string())
                        }
                    }
                };

                // For should_panic tests: panic = pass, no panic = fail
                let (status, message) = match &result {
                    Err(_) => {
                        #expected_check
                    }
                    Ok(_) => {
                        // No panic occurred - this is a failure for should_panic tests
                        (Status::Failed, Some("Test did not panic as expected".to_string()))
                    }
                };

                // Finish the test context with appropriate status
                if let Some(mut ctx) = take_context() {
                    ctx.finish(status, message, None);
                }

                // Re-panic or panic to match test framework expectations
                match result {
                    Err(e) => std::panic::resume_unwind(e),
                    Ok(_) => panic!("Test did not panic as expected"),
                }
            }
        })
    } else if returns_result {
        // For tests returning Result<T, E> - handle both Ok and Err
        Ok(quote! {
            #(#attrs)*
            #test_attr
            #visibility fn #fn_name #generics () #output {
                use ::allure_core::runtime::{set_context, take_context, TestContext};
                use ::allure_core::enums::Status;

                // Build full name at runtime using module_path!()
                let full_name = concat!(module_path!(), "::", #fn_name_str);
                let ctx = TestContext::new(#test_name, full_name);
                set_context(ctx);

                #setup_metadata

                // Run the test body, catching panics
                let panic_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| #block));

                match panic_result {
                    Ok(Ok(value)) => {
                        // Result::Ok - test passed
                        if let Some(mut ctx) = take_context() {
                            ctx.finish(Status::Passed, None, None);
                        }
                        Ok(value)
                    }
                    Ok(Err(e)) => {
                        // Result::Err - test failed via error return
                        let error_msg = format!("{:?}", e);
                        if let Some(mut ctx) = take_context() {
                            ctx.finish(Status::Failed, Some(error_msg), None);
                        }
                        Err(e)
                    }
                    Err(panic) => {
                        // Panic - test failed via panic
                        let panic_msg = if let Some(s) = panic.downcast_ref::<&str>() {
                            Some(s.to_string())
                        } else if let Some(s) = panic.downcast_ref::<String>() {
                            Some(s.clone())
                        } else {
                            Some("Test panicked".to_string())
                        };
                        if let Some(mut ctx) = take_context() {
                            ctx.finish(Status::Failed, panic_msg, None);
                        }
                        std::panic::resume_unwind(panic);
                    }
                }
            }
        })
    } else {
        // For regular sync tests - run body once and handle panics properly
        Ok(quote! {
            #(#attrs)*
            #test_attr
            #visibility fn #fn_name #generics () #output {
                use ::allure_core::runtime::{set_context, take_context, TestContext};
                use ::allure_core::enums::Status;

                // Build full name at runtime using module_path!()
                let full_name = concat!(module_path!(), "::", #fn_name_str);
                let ctx = TestContext::new(#test_name, full_name);
                set_context(ctx);

                #setup_metadata

                // Run the test body once and capture result
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| #block));

                // Extract panic message before taking context
                let panic_msg = match &result {
                    Ok(_) => None,
                    Err(e) => {
                        if let Some(s) = e.downcast_ref::<&str>() {
                            Some(s.to_string())
                        } else if let Some(s) = e.downcast_ref::<String>() {
                            Some(s.clone())
                        } else {
                            Some("Test panicked".to_string())
                        }
                    }
                };

                // Finish the test context with appropriate status
                if let Some(mut ctx) = take_context() {
                    match &result {
                        Ok(_) => ctx.finish(Status::Passed, None, None),
                        Err(_) => ctx.finish(Status::Failed, panic_msg, None),
                    }
                }

                // Re-panic if test failed to propagate to test framework
                if let Err(e) = result {
                    std::panic::resume_unwind(e);
                }
            }
        })
    }
}

/// Metadata extracted from attributes
#[derive(Default)]
#[allow(dead_code)]
struct TestMetadata {
    epic: Option<String>,
    feature: Option<String>,
    story: Option<String>,
    suite: Option<String>,
    parent_suite: Option<String>,
    sub_suite: Option<String>,
    severity: Option<String>,
    owner: Option<String>,
    tags: Vec<String>,
    id: Option<String>,
    description: Option<String>,
    description_html: Option<String>,
    issues: Vec<(String, Option<String>)>,
    tms_links: Vec<(String, Option<String>)>,
    links: Vec<(String, Option<String>)>,
    flaky: bool,
    muted: bool,
    known_issue: Option<String>,
}

fn extract_metadata_from_attrs(_attrs: &[Attribute]) -> TestMetadata {
    // For now, return default. The actual metadata will be set via runtime calls
    // or additional attribute macros
    TestMetadata::default()
}

fn generate_metadata_setup(metadata: &TestMetadata) -> proc_macro2::TokenStream {
    let mut setup = quote! {};

    if let Some(ref epic) = metadata.epic {
        setup = quote! { #setup ::allure_core::runtime::epic(#epic); };
    }
    if let Some(ref feature) = metadata.feature {
        setup = quote! { #setup ::allure_core::runtime::feature(#feature); };
    }
    if let Some(ref story) = metadata.story {
        setup = quote! { #setup ::allure_core::runtime::story(#story); };
    }
    if let Some(ref suite) = metadata.suite {
        setup = quote! { #setup ::allure_core::runtime::suite(#suite); };
    }
    if let Some(ref parent_suite) = metadata.parent_suite {
        setup = quote! { #setup ::allure_core::runtime::parent_suite(#parent_suite); };
    }
    if let Some(ref sub_suite) = metadata.sub_suite {
        setup = quote! { #setup ::allure_core::runtime::sub_suite(#sub_suite); };
    }
    if let Some(ref owner) = metadata.owner {
        setup = quote! { #setup ::allure_core::runtime::owner(#owner); };
    }
    if let Some(ref id) = metadata.id {
        setup = quote! { #setup ::allure_core::runtime::allure_id(#id); };
    }
    if let Some(ref desc) = metadata.description {
        setup = quote! { #setup ::allure_core::runtime::description(#desc); };
    }
    if metadata.flaky {
        setup = quote! { #setup ::allure_core::runtime::flaky(); };
    }
    if metadata.muted {
        setup = quote! { #setup ::allure_core::runtime::muted(); };
    }

    for tag in &metadata.tags {
        setup = quote! { #setup ::allure_core::runtime::tag(#tag); };
    }

    for (url, name) in &metadata.issues {
        let name_opt = name
            .as_ref()
            .map(|n| quote! { Some(#n.to_string()) })
            .unwrap_or(quote! { None });
        setup = quote! { #setup ::allure_core::runtime::issue(#url, #name_opt); };
    }

    for (url, name) in &metadata.tms_links {
        let name_opt = name
            .as_ref()
            .map(|n| quote! { Some(#n.to_string()) })
            .unwrap_or(quote! { None });
        setup = quote! { #setup ::allure_core::runtime::tms(#url, #name_opt); };
    }

    for (url, name) in &metadata.links {
        let name_opt = name
            .as_ref()
            .map(|n| quote! { Some(#n.to_string()) })
            .unwrap_or(quote! { None });
        setup = quote! { #setup ::allure_core::runtime::link(#url, #name_opt); };
    }

    setup
}

/// Attribute macro that marks a function as an Allure step.
///
/// # Examples
///
/// ```no_run
/// use allure_macros::allure_step_fn;
///
/// struct User { name: String }
/// impl User { fn new(name: &str) -> Self { Self { name: name.to_string() } } }
///
/// #[allure_step_fn]
/// fn setup_user() -> User {
///     User::new("test")
/// }
///
/// #[allure_step_fn("Create user with name {name}")]
/// fn create_user(name: &str) -> User {
///     User::new(name)
/// }
/// ```
#[proc_macro_attribute]
pub fn allure_step_fn(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let custom_name = if attr.is_empty() {
        None
    } else {
        let name = parse_macro_input!(attr as Lit);
        match name {
            Lit::Str(s) => Some(s.value()),
            _ => None,
        }
    };

    expand_step(input, custom_name)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn expand_step(
    input: ItemFn,
    custom_name: Option<String>,
) -> syn::Result<proc_macro2::TokenStream> {
    let fn_name = &input.sig.ident;
    let fn_name_str = fn_name.to_string();
    let step_name = custom_name.unwrap_or_else(|| fn_name_str.clone());
    let visibility = &input.vis;
    let attrs = &input.attrs;
    let block = &input.block;
    let sig = &input.sig;
    let is_async = sig.asyncness.is_some();
    let inputs = &sig.inputs;
    let output = &sig.output;
    let generics = &sig.generics;
    let where_clause = &sig.generics.where_clause;

    // Extract parameter names for interpolation
    let param_captures = generate_param_captures(inputs);

    // Interpolate step name with parameters
    let step_name_with_params = if step_name.contains('{') {
        quote! {
            {
                let mut name = #step_name.to_string();
                #param_captures
                name
            }
        }
    } else {
        quote! { #step_name.to_string() }
    };

    if is_async {
        Ok(quote! {
            #(#attrs)*
            #visibility async fn #fn_name #generics (#inputs) #output #where_clause {
                use ::allure_core::futures::FutureExt;

                let step_name = #step_name_with_params;
                ::allure_core::runtime::with_context(|ctx| ctx.start_step(&step_name));

                // Run the async body with panic catching
                let step_body = async #block;
                let panic_result = std::panic::AssertUnwindSafe(step_body).catch_unwind().await;

                match panic_result {
                    Ok(result) => {
                        ::allure_core::runtime::with_context(|ctx| {
                            ctx.finish_step(::allure_core::enums::Status::Passed, None, None)
                        });
                        result
                    }
                    Err(panic) => {
                        let panic_msg = if let Some(s) = panic.downcast_ref::<&str>() {
                            Some(s.to_string())
                        } else if let Some(s) = panic.downcast_ref::<String>() {
                            Some(s.clone())
                        } else {
                            Some("Step panicked".to_string())
                        };
                        ::allure_core::runtime::with_context(|ctx| {
                            ctx.finish_step(::allure_core::enums::Status::Failed, panic_msg, None)
                        });
                        std::panic::resume_unwind(panic);
                    }
                }
            }
        })
    } else {
        Ok(quote! {
            #(#attrs)*
            #visibility fn #fn_name #generics (#inputs) #output #where_clause {
                let step_name = #step_name_with_params;
                ::allure_core::runtime::step(step_name, || #block)
            }
        })
    }
}

fn generate_param_captures(inputs: &Punctuated<FnArg, Token![,]>) -> proc_macro2::TokenStream {
    let mut captures = quote! {};

    for arg in inputs {
        if let FnArg::Typed(pat_type) = arg {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                let param_name = &pat_ident.ident;
                let param_str = param_name.to_string();
                let placeholder = format!("{{{}}}", param_str);
                captures = quote! {
                    #captures
                    name = name.replace(#placeholder, &format!("{:?}", #param_name));
                };
            }
        }
    }

    captures
}

/// Attribute macro that groups tests in a module under a suite.
///
/// # Examples
///
/// ```no_run
/// use allure_macros::allure_suite;
///
/// #[allure_suite("Authentication Tests")]
/// mod auth_tests {
///     #[test]
///     fn test_login() { }
/// }
/// ```
#[proc_macro_attribute]
pub fn allure_suite(attr: TokenStream, item: TokenStream) -> TokenStream {
    let suite_name = parse_macro_input!(attr as Lit);
    let suite_name_str = match suite_name {
        Lit::Str(s) => s.value(),
        _ => {
            return syn::Error::new(suite_name.span(), "Expected string literal")
                .to_compile_error()
                .into()
        }
    };

    let input = parse_macro_input!(item as ItemMod);
    expand_allure_suite(input, suite_name_str)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn expand_allure_suite(
    input: ItemMod,
    _suite_name: String,
) -> syn::Result<proc_macro2::TokenStream> {
    // For now, just pass through the module unchanged
    // The suite metadata will be handled at runtime
    Ok(input.to_token_stream())
}

// === Metadata attribute macros ===

/// Adds an epic label to a test.
#[proc_macro_attribute]
pub fn allure_epic(attr: TokenStream, item: TokenStream) -> TokenStream {
    metadata_attr("epic", attr, item)
}

/// Adds a feature label to a test.
#[proc_macro_attribute]
pub fn allure_feature(attr: TokenStream, item: TokenStream) -> TokenStream {
    metadata_attr("feature", attr, item)
}

/// Adds a story label to a test.
#[proc_macro_attribute]
pub fn allure_story(attr: TokenStream, item: TokenStream) -> TokenStream {
    metadata_attr("story", attr, item)
}

/// Adds a suite label to a test.
#[proc_macro_attribute]
pub fn allure_suite_label(attr: TokenStream, item: TokenStream) -> TokenStream {
    metadata_attr("suite", attr, item)
}

/// Adds a parent suite label to a test.
#[proc_macro_attribute]
pub fn allure_parent_suite(attr: TokenStream, item: TokenStream) -> TokenStream {
    metadata_attr("parent_suite", attr, item)
}

/// Adds a sub-suite label to a test.
#[proc_macro_attribute]
pub fn allure_sub_suite(attr: TokenStream, item: TokenStream) -> TokenStream {
    metadata_attr("sub_suite", attr, item)
}

/// Adds a severity label to a test.
#[proc_macro_attribute]
pub fn allure_severity(attr: TokenStream, item: TokenStream) -> TokenStream {
    metadata_attr("severity", attr, item)
}

/// Adds an owner label to a test.
#[proc_macro_attribute]
pub fn allure_owner(attr: TokenStream, item: TokenStream) -> TokenStream {
    metadata_attr("owner", attr, item)
}

/// Adds tag labels to a test.
#[proc_macro_attribute]
pub fn allure_tag(attr: TokenStream, item: TokenStream) -> TokenStream {
    metadata_attr("tag", attr, item)
}

/// Adds an ID label to a test.
#[proc_macro_attribute]
pub fn allure_id(attr: TokenStream, item: TokenStream) -> TokenStream {
    metadata_attr("id", attr, item)
}

/// Adds a description to a test (markdown format).
#[proc_macro_attribute]
pub fn allure_description(attr: TokenStream, item: TokenStream) -> TokenStream {
    metadata_attr("description", attr, item)
}

/// Adds an HTML description to a test.
///
/// # Example
///
/// ```no_run
/// use allure_macros::{allure_test, allure_description_html};
///
/// #[allure_description_html("<h1>Test Header</h1><p>Test description</p>")]
/// #[allure_test]
/// fn test_with_html_description() { }
/// ```
#[proc_macro_attribute]
pub fn allure_description_html(attr: TokenStream, item: TokenStream) -> TokenStream {
    metadata_attr("description_html", attr, item)
}

/// Sets a custom title for a test.
///
/// This overrides the test name displayed in Allure reports.
///
/// # Example
///
/// ```no_run
/// use allure_macros::{allure_test, allure_title};
///
/// #[allure_title("User can login with valid credentials")]
/// #[allure_test]
/// fn test_login() { }
/// ```
#[proc_macro_attribute]
pub fn allure_title(attr: TokenStream, item: TokenStream) -> TokenStream {
    metadata_attr("title", attr, item)
}

/// Adds multiple epic labels to a test.
///
/// # Example
///
/// ```no_run
/// use allure_macros::{allure_test, allure_epics};
///
/// #[allure_epics("User Management", "Authentication")]
/// #[allure_test]
/// fn test_login() { }
/// ```
#[proc_macro_attribute]
pub fn allure_epics(attr: TokenStream, item: TokenStream) -> TokenStream {
    plural_metadata_attr("epic", attr, item)
}

/// Adds multiple feature labels to a test.
///
/// # Example
///
/// ```no_run
/// use allure_macros::{allure_test, allure_features};
///
/// #[allure_features("Login", "Registration")]
/// #[allure_test]
/// fn test_auth() { }
/// ```
#[proc_macro_attribute]
pub fn allure_features(attr: TokenStream, item: TokenStream) -> TokenStream {
    plural_metadata_attr("feature", attr, item)
}

/// Adds multiple story labels to a test.
///
/// # Example
///
/// ```no_run
/// use allure_macros::{allure_test, allure_stories};
///
/// #[allure_stories("User can login", "User can logout")]
/// #[allure_test]
/// fn test_user_session() { }
/// ```
#[proc_macro_attribute]
pub fn allure_stories(attr: TokenStream, item: TokenStream) -> TokenStream {
    plural_metadata_attr("story", attr, item)
}

/// Adds multiple tag labels to a test.
///
/// # Example
///
/// ```no_run
/// use allure_macros::{allure_test, allure_tags};
///
/// #[allure_tags("smoke", "regression", "api")]
/// #[allure_test]
/// fn test_api() { }
/// ```
#[proc_macro_attribute]
pub fn allure_tags(attr: TokenStream, item: TokenStream) -> TokenStream {
    plural_metadata_attr("tag", attr, item)
}

/// Helper to generate plural metadata attribute macros
fn plural_metadata_attr(meta_type: &str, attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let values: PluralArgs = match syn::parse(attr) {
        Ok(args) => args,
        Err(e) => return e.to_compile_error().into(),
    };

    let visibility = &input.vis;
    let attrs = &input.attrs;
    let block = &input.block;
    let sig = &input.sig;

    let mut runtime_calls = quote! {};
    for value in &values.values {
        let call = match meta_type {
            "epic" => quote! { ::allure_core::runtime::epic(#value); },
            "feature" => quote! { ::allure_core::runtime::feature(#value); },
            "story" => quote! { ::allure_core::runtime::story(#value); },
            "tag" => quote! { ::allure_core::runtime::tag(#value); },
            _ => quote! {},
        };
        runtime_calls = quote! { #runtime_calls #call };
    }

    let expanded = quote! {
        #(#attrs)*
        #visibility #sig {
            #runtime_calls
            #block
        }
    };

    expanded.into()
}

/// Arguments for plural metadata attributes: ("value1", "value2", ...)
struct PluralArgs {
    values: Vec<String>,
}

impl Parse for PluralArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut values = Vec::new();

        while !input.is_empty() {
            let lit: Lit = input.parse()?;
            match lit {
                Lit::Str(s) => values.push(s.value()),
                _ => return Err(syn::Error::new(lit.span(), "Expected string literal")),
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }

        Ok(PluralArgs { values })
    }
}

/// Marks a test as flaky.
#[proc_macro_attribute]
pub fn allure_flaky(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let visibility = &input.vis;
    let attrs = &input.attrs;
    let block = &input.block;
    let sig = &input.sig;

    let expanded = quote! {
        #(#attrs)*
        #visibility #sig {
            ::allure_core::runtime::flaky();
            #block
        }
    };

    expanded.into()
}

/// Adds an issue link to a test.
#[proc_macro_attribute]
pub fn allure_issue(attr: TokenStream, item: TokenStream) -> TokenStream {
    link_attr("issue", attr, item)
}

/// Adds a TMS link to a test.
#[proc_macro_attribute]
pub fn allure_tms(attr: TokenStream, item: TokenStream) -> TokenStream {
    link_attr("tms", attr, item)
}

/// Adds a generic link to a test.
#[proc_macro_attribute]
pub fn allure_link(attr: TokenStream, item: TokenStream) -> TokenStream {
    link_attr("link", attr, item)
}

/// Helper to generate metadata attribute macros
fn metadata_attr(meta_type: &str, attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let value = parse_macro_input!(attr as Lit);

    let value_str = match value {
        Lit::Str(s) => s.value(),
        _ => {
            return syn::Error::new(value.span(), "Expected string literal")
                .to_compile_error()
                .into()
        }
    };

    let visibility = &input.vis;
    let attrs = &input.attrs;
    let block = &input.block;
    let sig = &input.sig;

    let runtime_call = match meta_type {
        "epic" => quote! { ::allure_core::runtime::epic(#value_str); },
        "feature" => quote! { ::allure_core::runtime::feature(#value_str); },
        "story" => quote! { ::allure_core::runtime::story(#value_str); },
        "suite" => quote! { ::allure_core::runtime::suite(#value_str); },
        "parent_suite" => quote! { ::allure_core::runtime::parent_suite(#value_str); },
        "sub_suite" => quote! { ::allure_core::runtime::sub_suite(#value_str); },
        "severity" => {
            // Parse severity string to enum variant
            let severity = match value_str.to_lowercase().as_str() {
                "blocker" => quote! { ::allure_core::Severity::Blocker },
                "critical" => quote! { ::allure_core::Severity::Critical },
                "normal" => quote! { ::allure_core::Severity::Normal },
                "minor" => quote! { ::allure_core::Severity::Minor },
                "trivial" => quote! { ::allure_core::Severity::Trivial },
                _ => quote! { ::allure_core::Severity::Normal },
            };
            quote! { ::allure_core::runtime::severity(#severity); }
        }
        "owner" => quote! { ::allure_core::runtime::owner(#value_str); },
        "tag" => quote! { ::allure_core::runtime::tag(#value_str); },
        "id" => quote! { ::allure_core::runtime::allure_id(#value_str); },
        "description" => quote! { ::allure_core::runtime::description(#value_str); },
        "description_html" => quote! { ::allure_core::runtime::description_html(#value_str); },
        "title" => quote! { ::allure_core::runtime::title(#value_str); },
        _ => quote! {},
    };

    let expanded = quote! {
        #(#attrs)*
        #visibility #sig {
            #runtime_call
            #block
        }
    };

    expanded.into()
}

/// Helper to generate link attribute macros
fn link_attr(link_type: &str, attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    // Parse either a single string or a tuple (url, name)
    let (url, name) = if let Ok(lit) = syn::parse::<Lit>(attr.clone()) {
        match lit {
            Lit::Str(s) => (s.value(), None),
            _ => {
                return syn::Error::new(lit.span(), "Expected string literal")
                    .to_compile_error()
                    .into()
            }
        }
    } else {
        // Try parsing as tuple: ("url", "name")
        let args: LinkArgs = match syn::parse(attr) {
            Ok(args) => args,
            Err(e) => return e.to_compile_error().into(),
        };
        (args.url, args.name)
    };

    let visibility = &input.vis;
    let attrs = &input.attrs;
    let block = &input.block;
    let sig = &input.sig;

    let name_opt = name
        .map(|n| quote! { Some(#n.to_string()) })
        .unwrap_or(quote! { None });

    let runtime_call = match link_type {
        "issue" => quote! { ::allure_core::runtime::issue(#url, #name_opt); },
        "tms" => quote! { ::allure_core::runtime::tms(#url, #name_opt); },
        "link" => quote! { ::allure_core::runtime::link(#url, #name_opt); },
        _ => quote! {},
    };

    let expanded = quote! {
        #(#attrs)*
        #visibility #sig {
            #runtime_call
            #block
        }
    };

    expanded.into()
}

/// Arguments for link attributes: (url) or (url, name)
struct LinkArgs {
    url: String,
    name: Option<String>,
}

impl Parse for LinkArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let url: Lit = input.parse()?;
        let url_str = match url {
            Lit::Str(s) => s.value(),
            _ => {
                return Err(syn::Error::new(
                    url.span(),
                    "Expected string literal for URL",
                ))
            }
        };

        let name = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            let name_lit: Lit = input.parse()?;
            match name_lit {
                Lit::Str(s) => Some(s.value()),
                _ => {
                    return Err(syn::Error::new(
                        name_lit.span(),
                        "Expected string literal for name",
                    ))
                }
            }
        } else {
            None
        };

        Ok(LinkArgs { url: url_str, name })
    }
}

/// Macro for inline step definition.
///
/// # Examples
///
/// ```no_run
/// use allure_macros::allure_step;
///
/// fn setup() {}
/// fn configure() {}
///
/// fn main() {
///     allure_step!("Initialize system", {
///         setup();
///         configure();
///     });
/// }
/// ```
#[proc_macro]
pub fn allure_step(input: TokenStream) -> TokenStream {
    let step_input = parse_macro_input!(input as StepInput);
    let name = step_input.name;
    let body = step_input.body;

    let expanded = quote! {
        ::allure_core::runtime::step(#name, || { #body })
    };

    expanded.into()
}

struct StepInput {
    name: String,
    body: proc_macro2::TokenStream,
}

impl Parse for StepInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name_lit: Lit = input.parse()?;
        let name = match name_lit {
            Lit::Str(s) => s.value(),
            _ => return Err(syn::Error::new(name_lit.span(), "Expected string literal")),
        };

        input.parse::<Token![,]>()?;

        let body: proc_macro2::TokenStream = input.parse()?;

        Ok(StepInput { name, body })
    }
}
