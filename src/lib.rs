mod builder;
mod generate;
mod model;

use std::path::Path;

/// MockSmith is a struct for generating Google Mock mocks for C++ classes.
pub struct MockSmith {
    clang: clang::Clang,
}

impl MockSmith {
    pub fn new() -> Self {
        Self {
            clang: clang::Clang::new()
                .unwrap_or_else(|message| panic!("Could not create Clang: {}", message)),
        }
    }

    pub fn create_mocks_for_file(&self, file: &Path) -> Vec<String> {
        let index = clang::Index::new(&self.clang, true, false);
        let tu = tu_from_file(&index, file);
        let classes = model::classes_in_translation_unit(&tu);
        classes
            .iter()
            .map(|class| {
                generate::generate_mock(builder::CodeBuilder::new(2), &class, &mock_name(class))
            })
            .collect()
    }
}

pub fn tu_from_file<'a>(index: &'a clang::Index<'_>, file: &Path) -> clang::TranslationUnit<'a> {
    index
        .parser(file)
        .arguments(&["--language=c++"])
        .parse()
        .expect("Failed to parse translation unit")
}

fn mock_name(class: &model::ClassToMock<'_>) -> String {
    format!(
        "Mock{}",
        class.class.get_name().expect("Class should have a name")
    )
}
