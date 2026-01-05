use crate::log;
use std::rc::Rc;

// Represents a class that shall be mocked
#[derive(Debug)]
pub(crate) struct ClassToMock {
    pub(crate) name: String,
    pub(crate) namespaces: Vec<String>,
    pub(crate) methods: Vec<MethodToMock>,
}

// Represents a class method that shall be mocked
#[derive(Debug)]
pub(crate) struct MethodToMock {
    pub(crate) name: String,
    pub(crate) result_type: String,
    pub(crate) arguments: Vec<Argument>,
    pub(crate) is_const: bool,
    pub(crate) is_virtual: bool,
    pub(crate) is_noexcept: bool,
    pub(crate) ref_qualifier: Option<String>,
}

// Represents a method argument
#[derive(Debug, PartialEq)]
pub(crate) struct Argument {
    pub(crate) type_name: String,
    pub(crate) name: Option<String>,
}

// Finds classes to mock in the main file of a translation unit
pub(crate) fn classes_in_translation_unit(
    log: Rc<Option<log::Logger>>,
    root: &clang::TranslationUnit,
    methods_to_mock: crate::MethodsToMockStrategy,
) -> Vec<ClassToMock> {
    AstTraverser::new(log, root, methods_to_mock).traverse()
}

// Factory for creating model objects from clang entities
struct ModelFactory {
    log: Rc<Option<log::Logger>>,
    file_contents: Option<String>,
}

impl ModelFactory {
    fn new(log: Rc<Option<log::Logger>>) -> Self {
        Self {
            log,
            file_contents: None,
        }
    }

    fn class_from_entity(
        &mut self,
        class: &clang::Entity,
        namespaces: &Vec<clang::Entity>,
        methods_to_mock: crate::MethodsToMockStrategy,
    ) -> ClassToMock {
        ClassToMock {
            name: class.get_name().expect("Class should have a name"),
            namespaces: namespaces
                .iter()
                .map(|ns| ns.get_name().expect("Namespace should have a name"))
                .collect::<Vec<_>>(),
            methods: class
                .get_children()
                .iter()
                .filter(|child| child.get_kind() == clang::EntityKind::Method)
                .filter(|method| methods_to_mock.should_mock(method))
                .map(|method| self.method_from_entity(method))
                .collect(),
        }
    }

    fn method_from_entity(&mut self, method: &clang::Entity) -> MethodToMock {
        MethodToMock {
            name: method.get_name().expect("Method should have a name"),
            result_type: method
                .get_result_type()
                .expect("Method should have a return type")
                .get_display_name(),
            arguments: method
                .get_arguments()
                .expect("Method should have arguments")
                .iter()
                .map(|arg| Argument {
                    type_name: self.get_type(arg),
                    name: arg.get_name(),
                })
                .collect(),
            is_const: method.is_const_method(),
            is_virtual: method.is_virtual_method(),
            is_noexcept: (method.get_exception_specification()
                == Some(clang::ExceptionSpecification::BasicNoexcept)),
            ref_qualifier: method.get_type().and_then(|t| t.get_ref_qualifier()).map(
                |rq| match rq {
                    clang::RefQualifier::LValue => "&".to_string(),
                    clang::RefQualifier::RValue => "&&".to_string(),
                },
            ),
        }
    }

    fn get_type(&mut self, entity: &clang::Entity) -> String {
        self.extract_type_from_source(entity).unwrap_or_else(|| {
            entity
                .get_type()
                .expect("Entity should have a type")
                .get_display_name()
        })
    }

    fn extract_type_from_source(&mut self, entity: &clang::Entity) -> Option<String> {
        if let Some(range) = entity.get_range() {
            self.cache_file_contents(entity);

            if let Some(file_contents) = &self.file_contents {
                let start = range.get_start().get_file_location().offset as usize;
                let mut end = range.get_end().get_file_location().offset as usize;
                if let Some(name) = entity.get_name() {
                    end -= name.len();
                }

                if start >= end || end > file_contents.len() {
                    log!(
                        self.log,
                        "Falling back to clang type extraction for entity {:?} \
                         due to illegal file position",
                        entity
                    );
                    return None;
                }
                return Some(file_contents[start..end].trim().to_string());
            }
        }
        log!(
            self.log,
            "Falling back to clang type extraction for entity {:?} \
             due to missing range or file contents",
            entity
        );
        None
    }

    fn cache_file_contents(&mut self, entity: &clang::Entity) {
        if self.file_contents.is_none()
            && let Some(location) = entity.get_location()
            && let Some(file) = location.get_file_location().file
        {
            self.file_contents = file.get_contents();
        }
    }
}

// Traverses the AST to find classes to mock
struct AstTraverser<'a> {
    root: clang::Entity<'a>,
    factory: ModelFactory,
    methods_to_mock: crate::MethodsToMockStrategy,

    classes: Vec<ClassToMock>,
    namespace_stack: Vec<clang::Entity<'a>>,
}

impl<'a> AstTraverser<'a> {
    pub fn new(
        log: Rc<Option<log::Logger>>,
        root: &'a clang::TranslationUnit<'a>,
        methods_to_mock: crate::MethodsToMockStrategy,
    ) -> Self {
        Self {
            root: root.get_entity(),
            factory: ModelFactory::new(log),
            methods_to_mock,
            classes: Vec::new(),
            namespace_stack: Vec::new(),
        }
    }

    fn traverse(mut self) -> Vec<ClassToMock> {
        self.traverse_recursive(self.root);
        self.classes
    }

    fn traverse_recursive(&mut self, entity: clang::Entity<'a>) {
        match entity.get_kind() {
            clang::EntityKind::ClassDecl => {
                if entity.is_definition() && self.should_mock_class(&entity) {
                    self.classes.push(self.factory.class_from_entity(
                        &entity,
                        &self.namespace_stack,
                        self.methods_to_mock,
                    ));
                }
            }

            clang::EntityKind::Namespace => {
                self.namespace_stack.push(entity);
            }

            _ => {}
        }

        for child in entity.get_children() {
            if child.is_in_main_file() {
                self.traverse_recursive(child);
            }
        }

        if entity.get_kind() == clang::EntityKind::Namespace {
            self.namespace_stack.pop();
        }
    }

    fn should_mock_class(&self, class: &clang::Entity) -> bool {
        class.get_children().iter().any(|child| {
            child.get_kind() == clang::EntityKind::Method && self.methods_to_mock.should_mock(child)
        })
    }
}

impl crate::MethodsToMockStrategy {
    fn should_mock(self, method: &clang::Entity) -> bool {
        match self {
            crate::MethodsToMockStrategy::All => !method.is_static_method(),
            crate::MethodsToMockStrategy::AllVirtual => method.is_virtual_method(),
            crate::MethodsToMockStrategy::OnlyPureVirtual => method.is_pure_virtual_method(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clangwrap::ClangWrap;

    #[test]
    fn class_with_methods_with_recognized_types() {
        let code = r#"
        class MyClass {
        public:
            virtual void foo() const noexcept;
            int bar(int x);
            virtual int baz() = 0;
            virtual auto bizz() const noexcept -> int = 0;
            static void staticMethod();
        };
        "#;

        let clang = ClangWrap::blocking_new().unwrap();
        let _ = clang.with_tu_from_string(&[], code, |tu| {
            let classes =
                classes_in_translation_unit(Rc::new(None), &tu, crate::MethodsToMockStrategy::All);

            assert_eq!(classes.len(), 1);
            let class = &classes[0];
            assert_eq!(class.name, "MyClass");
            // staticMethod should be excluded
            assert_eq!(class.methods.len(), 4);

            assert!(matches!(
                &class.methods[0],
                &MethodToMock {
                    name: ref n,
                    result_type: ref rt,
                    arguments: ref args,
                    is_const: true,
                    is_virtual: true,
                    is_noexcept: true,
                    ref_qualifier: None,
                }
                if n == "foo" && rt == "void" && args.is_empty()
            ));

            assert!(matches!(
                &class.methods[1],
                &MethodToMock {
                    name: ref n,
                    result_type: ref rt,
                    arguments: ref args,
                    is_const: false,
                    is_virtual: false,
                    is_noexcept: false,
                    ref_qualifier: None,
                } if n == "bar"
                     && rt == "int"
                     && args == &vec![Argument{ type_name: "int".to_string(), name: Some("x".to_string()) }]
            ));

            assert!(matches!(
                &class.methods[2],
                &MethodToMock {
                    name: ref n,
                    result_type: ref rt,
                    arguments: ref args,
                    is_const: false,
                    is_virtual: true,
                    is_noexcept: false,
                ref_qualifier: None,
                } if n == "baz" && rt == "int" && args.is_empty()
            ));

            assert!(matches!(
                &class.methods[3],
                &MethodToMock {
                    name: ref n,
                    result_type: ref rt,
                    arguments: ref args,
                    is_const: true,
                    is_virtual: true,
                    is_noexcept: true,
                    ref_qualifier: None,
                } if n == "bizz" && rt == "int" && args.is_empty()
            ));

            Ok(())
        });
    }

    #[test]
    fn unknown_arguments_types_can_be_handled() {
        let code = r#"
        class MyClass {
        public:
            virtual void foo(Unknown x) const noexcept;
            void bar(Unknown);
            static void staticMethods(Unknown);
        };
        "#;

        let clang = ClangWrap::blocking_new().unwrap();
        let _ = clang.with_tu_from_string(&[], code, |tu| {
            let classes =
                classes_in_translation_unit(Rc::new(None), &tu, crate::MethodsToMockStrategy::All);

            assert_eq!(classes.len(), 1);
            let class = &classes[0];
            assert_eq!(class.name, "MyClass");
             // staticMethod should be excluded
            assert_eq!(class.methods.len(), 2);

            assert!(matches!(
                &class.methods[0],
                &MethodToMock {
                    name: ref n,
                    arguments: ref args,
                    is_const: true,
                    is_virtual: true,
                    is_noexcept: true,
                    ..
                }
                if n == "foo" && args == &vec![Argument { type_name: "Unknown".to_string(), name: Some("x".to_string()) }]
            ));

            assert!(matches!(
                &class.methods[1],
                &MethodToMock {
                    name: ref n,
                    arguments: ref args,
                    is_const: false,
                    is_virtual: false,
                    is_noexcept: false,
                    ..
                }
                if n == "bar" && args == &vec![Argument { type_name: "Unknown".to_string(), name: None }]
            ));

            Ok(())
        });
    }
}
