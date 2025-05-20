use clang::Clang;

mod builder;
mod generate;
mod model;

fn main() {
    let clang = Clang::new().unwrap();
    let index = clang::Index::new(&clang, false, false);
    //println!("Clang version: {}", clang::get_version());

    for arg in std::env::args().skip(1) {
        let tu = index
            .parser(arg)
            .arguments(&["--language=c++"])
            .parse()
            .expect("Failed to parse translation unit");

        let classes = model::classes_in_translation_unit(&tu);
        for class in classes {
            let mock_name = format!("Mock{}", class.class.get_name().unwrap());
            println!(
                "{}",
                generate::generate_mock(builder::CodeBuilder::new(2), &class, mock_name.as_str(),)
            );
        }
    }
}
