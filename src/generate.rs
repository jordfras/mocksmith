use crate::builder;
use crate::model;

pub(crate) fn generate_mock(
    mut builder: builder::CodeBuilder,
    class: &model::ClassToMock,
    methods_to_mock: crate::MethodsToMock,
    mock_name: &str,
) -> String {
    if let Some(namespace_start) = namespace_start(&class.namespaces) {
        builder.add_line(namespace_start.as_str());
    }

    builder.add_line(&format!(
        "class {} : public {}",
        mock_name,
        class.class.get_name().unwrap()
    ));
    builder.add_line("{");
    builder.add_line("public:");
    builder.push_indent();
    class
        .methods()
        .iter()
        .filter(|method| methods_to_mock.should_mock(method))
        .for_each(|method| {
            builder.add_line(&format!(
                "MOCK_METHOD({}, {}, ({}), ({}));",
                method_return_type(method),
                method.get_name().expect("Method should have a name"),
                method_arguments(method).join(", "),
                method_qualifiers(method).join(", ")
            ));
        });
    builder.pop_indent();
    builder.add_line("};");

    if let Some(namespace_end) = namespace_end(&class.namespaces) {
        builder.add_line(namespace_end.as_str());
    }

    builder.build()
}

fn namespace_start(namespaces: &[clang::Entity]) -> Option<String> {
    if namespaces.is_empty() {
        None
    } else {
        Some(
            namespaces
                .iter()
                .map(|namespace| {
                    format!(
                        "namespace {} {{",
                        namespace.get_name().expect("Namespace should have a name")
                    )
                })
                .collect::<Vec<_>>()
                .join(" "),
        )
    }
}

fn namespace_end(namespaces: &[clang::Entity]) -> Option<String> {
    if namespaces.is_empty() {
        None
    } else {
        Some("}".repeat(namespaces.len()))
    }
}

fn wrap_with_parentheses_if_contains_comma(return_type_or_arg: String) -> String {
    if return_type_or_arg.contains(',') {
        format!("({return_type_or_arg})")
    } else {
        return_type_or_arg.to_string()
    }
}

fn method_return_type(method: &clang::Entity) -> String {
    wrap_with_parentheses_if_contains_comma(
        method
            .get_result_type()
            .expect("Method should have a return type")
            .get_display_name(),
    )
}

fn method_arguments(method: &clang::Entity) -> Vec<String> {
    method
        .get_arguments()
        .expect("Method should have arguments")
        .iter()
        .map(|arg| {
            let type_name = arg
                .get_type()
                .expect("Argument should have a type")
                .get_display_name();
            if let Some(arg_name) = arg.get_name() {
                format!("{} {}", type_name, arg_name)
            } else {
                type_name
            }
        })
        .map(wrap_with_parentheses_if_contains_comma)
        .collect()
}

fn method_qualifiers(method: &clang::Entity) -> Vec<String> {
    let mut qualifiers = Vec::new();
    if method.is_const_method() {
        qualifiers.push("const".to_string());
    }
    if let Some(exception_specification) = method.get_exception_specification() {
        if exception_specification == clang::ExceptionSpecification::BasicNoexcept {
            qualifiers.push("noexcept".to_string());
        }
    }
    if method.is_virtual_method() {
        qualifiers.push("override".to_string());
    }
    qualifiers
}
