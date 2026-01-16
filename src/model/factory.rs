use super::{Argument, ClassToMock, MethodToMock};
use crate::log;
use std::rc::Rc;

// Factory for creating model objects from clang entities
pub(crate) struct ModelFactory {
    log: Rc<Option<log::Logger>>,
    file_contents: Option<String>,
}

impl ModelFactory {
    pub(crate) fn new(log: Rc<Option<log::Logger>>) -> Self {
        Self {
            log,
            file_contents: None,
        }
    }

    pub(crate) fn class_from_entity(
        &mut self,
        class: &clang::Entity,
        namespaces: &Vec<clang::Entity>,
        methods_to_mock: crate::MethodsToMockStrategy,
    ) -> ClassToMock {
        self.cache_file_contents(class);
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
                    type_name: self.get_argument_type(arg),
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

    fn get_argument_type(&mut self, arg_entity: &clang::Entity) -> String {
        self.extract_argument_type_from_source(arg_entity)
            .unwrap_or_else(|| {
                arg_entity
                    .get_type()
                    .expect("Entity should have a type")
                    .get_display_name()
            })
    }

    fn get_arg_range(&self, arg_entity: &clang::Entity) -> Option<(usize, usize)> {
        // entity.get_range() only seems to work when argument has a name, but
        // get_location() seems to work. We use it to find the start and then scan the
        // source to find the end
        if arg_entity.get_name().is_some() {
            arg_entity.get_range().map(|r| {
                let start = r.get_start().get_file_location().offset as usize;
                let end = r.get_end().get_file_location().offset as usize;
                (start, end)
            })
        } else if let Some(file_contents) = &self.file_contents
            && let Some(location) = arg_entity.get_location()
        {
            // Location is now _after_ the unknown argument type, so we need to scan
            // backwards to find the start
            let end = location.get_file_location().offset as usize;
            let bytes = file_contents.as_bytes();
            let mut start = 0;

            for i in (0..end).rev() {
                let c = bytes[i] as char;
                if c == ',' || c == '(' {
                    start = i + 1;
                    break;
                }
            }
            Some((start, end))
        } else {
            None
        }
    }

    fn extract_argument_type_from_source(&mut self, arg_entity: &clang::Entity) -> Option<String> {
        if let Some((start, mut end)) = self.get_arg_range(arg_entity)
            && let Some(file_contents) = &self.file_contents
        {
            if let Some(name) = arg_entity.get_name() {
                end -= name.len();
            }

            if start >= end || end > file_contents.len() {
                log!(
                    self.log,
                    "Falling back to clang type extraction for entity {:?} \
                         due to illegal file position",
                    arg_entity
                );
                return None;
            }
            return Some(file_contents[start..end].trim().to_string());
        }
        log!(
            self.log,
            "Falling back to clang type extraction for entity {:?} \
             due to missing range or file contents",
            arg_entity
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
