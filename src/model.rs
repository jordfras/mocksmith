// Represents a class that shall be mocked
#[derive(Debug)]
pub(crate) struct ClassToMock<'a> {
    pub(crate) class: clang::Entity<'a>,
    pub(crate) namespaces: Vec<clang::Entity<'a>>,
}

impl<'a> ClassToMock<'a> {
    pub(crate) fn methods(&self) -> Vec<clang::Entity<'a>> {
        self.class
            .get_children()
            .iter()
            .filter(|child| child.get_kind() == clang::EntityKind::Method)
            .copied()
            .collect()
    }

    pub(crate) fn name(&self) -> String {
        self.class.get_name().expect("Class should have a name")
    }
}

// Finds classes to mock in the main file of a translation unit
pub(crate) fn classes_in_translation_unit<'a>(
    root: &'a clang::TranslationUnit<'a>,
    methods_to_mock: crate::MethodsToMockStrategy,
) -> Vec<ClassToMock<'a>> {
    AstTraverser::new(root, methods_to_mock).traverse()
}

struct AstTraverser<'a> {
    root: clang::Entity<'a>,
    methods_to_mock: crate::MethodsToMockStrategy,

    classes: Vec<ClassToMock<'a>>,
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

    fn traverse(mut self) -> Vec<ClassToMock<'a>> {
        self.traverse_recursive(self.root);
        self.classes
    }

    fn traverse_recursive(&mut self, entity: clang::Entity<'a>) {
        match entity.get_kind() {
            clang::EntityKind::ClassDecl => {
                if entity.is_definition() && self.should_mock_class(&entity) {
                    let class = ClassToMock {
                        class: entity,
                        namespaces: self.namespace_stack.clone(),
                    };
                    self.classes.push(class);
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
