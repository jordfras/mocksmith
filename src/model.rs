use crate::log;
use std::rc::Rc;

mod factory;

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

// Traverses the AST to find classes to mock
struct AstTraverser<'a> {
    root: clang::Entity<'a>,
    factory: factory::ModelFactory,
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
            factory: factory::ModelFactory::new(log),
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
        }).unwrap();
    }

    #[test]
    fn unknown_argument_types_can_be_handled() {
        let code = r#"
        class MyClass {
        public:
            virtual void foo(Unknown x) const noexcept;
            void bar(Unknown);
            void bizz(Unknown1, Unknown2 x, Unknown3);
            static void staticMethods(Unknown);
        };
        "#;

        let mut clang = ClangWrap::blocking_new().unwrap();
        clang.set_ignore_errors(true);
        let _ = clang.with_tu_from_string(&[], code, |tu| {
            let classes =
                classes_in_translation_unit(Rc::new(None), &tu, crate::MethodsToMockStrategy::All);

            assert_eq!(classes.len(), 1);
            let class = &classes[0];
             // staticMethod should be excluded
            assert_eq!(class.methods.len(), 3);

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

            assert!(matches!(
                &class.methods[2],
                &MethodToMock {
                    name: ref n,
                    arguments: ref args,
                    ..
                }
                if n == "bizz" && args == &vec![
                    Argument { type_name: "Unknown1".to_string(), name: None },
                    Argument { type_name: "Unknown2".to_string(), name: Some("x".to_string()) },
                    Argument { type_name: "Unknown3".to_string(), name: None }
                ]
            ));

            Ok(())
        }).unwrap();
    }
}
