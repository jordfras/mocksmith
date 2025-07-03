// Represents a class that shall be mocked
#[derive(Debug)]
pub(crate) struct ClassToMock {
    pub(crate) name: String,
    pub(crate) namespaces: Vec<String>,
    pub(crate) methods: Vec<MethodToMock>,
}

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

#[derive(Debug)]
pub(crate) struct Argument {
    pub(crate) type_name: String,
    pub(crate) name: Option<String>,
}

// Finds classes to mock in the main file of a translation unit
pub(crate) fn classes_in_translation_unit(
    root: &clang::TranslationUnit,
    methods_to_mock: crate::MethodsToMockStrategy,
) -> Vec<ClassToMock> {
    AstTraverser::new(root, methods_to_mock).traverse()
}

impl ClassToMock {
    fn from_entity(
        class: &clang::Entity,
        namespaces: &Vec<clang::Entity>,
        methods_to_mock: crate::MethodsToMockStrategy,
    ) -> Self {
        Self {
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
                .map(|method| MethodToMock::from_entity(method))
                .collect(),
        }
    }
}

impl MethodToMock {
    fn from_entity(method: &clang::Entity) -> Self {
        Self {
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
                    type_name: arg
                        .get_type()
                        .expect("Argument should have a type")
                        .get_display_name(),
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
}

struct AstTraverser<'a> {
    root: clang::Entity<'a>,
    methods_to_mock: crate::MethodsToMockStrategy,

    classes: Vec<ClassToMock>,
    namespace_stack: Vec<clang::Entity<'a>>,
}

impl<'a> AstTraverser<'a> {
    pub fn new(
        root: &'a clang::TranslationUnit<'a>,
        methods_to_mock: crate::MethodsToMockStrategy,
    ) -> Self {
        Self {
            root: root.get_entity(),
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
                    self.classes.push(ClassToMock::from_entity(
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
