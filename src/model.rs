// Represents a class that shall be mocked
#[derive(Debug)]
pub struct ClassToMock<'a> {
    pub class: clang::Entity<'a>,
    pub namespaces: Vec<clang::Entity<'a>>,
}

impl<'a> ClassToMock<'a> {
    pub fn methods(&self) -> Vec<clang::Entity<'a>> {
        self.class
            .get_children()
            .iter()
            .filter(|child| {
                child.get_kind() == clang::EntityKind::Method
                    && child.get_accessibility() == Some(clang::Accessibility::Public)
                    && !child.is_static_method()
            })
            .copied()
            .collect()
    }
}

// Finds classes to mock in the main file of a translation unit
pub fn classes_in_translation_unit<'a>(
    root: &'a clang::TranslationUnit<'a>,
) -> Vec<ClassToMock<'a>> {
    AstTraverser::new(root).traverse()
}

struct AstTraverser<'a> {
    root: clang::Entity<'a>,
    classes: Vec<ClassToMock<'a>>,
    namespace_stack: Vec<clang::Entity<'a>>,
}

impl<'a> AstTraverser<'a> {
    pub fn new(root: &'a clang::TranslationUnit<'a>) -> Self {
        Self {
            root: root.get_entity(),
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
                if entity.is_definition() {
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
}
