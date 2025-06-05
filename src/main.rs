use std::io::Read;

use mocksmith::Mocksmith;

fn main() {
    let mocksmith = Mocksmith::new()
        .unwrap_or_else(|message| panic!("Could not create Mocksmith instance: {message}"));

    if std::env::args().len() == 1 {
        let mut content = String::new();
        std::io::stdin()
            .read_to_string(&mut content)
            .unwrap_or_else(|_| {
                eprintln!("Failed to read from stdin");
                std::process::exit(1);
            });
        mocksmith
            .create_mocks_from_string(&content)
            .unwrap_or_else(|error| {
                eprintln!("Error creating mocks from string:\n{error}");
                std::process::exit(1);
            })
            .into_iter()
            .for_each(|mock| {
                println!("{}", mock);
            });
    } else {
        for arg in std::env::args().skip(1) {
            println!(
                "{}",
                mocksmith
                    .create_mock_header_for_file(std::path::Path::new(&arg))
                    .unwrap_or_else(|error| {
                        eprintln!("Error creating mocks from file:\n{error}");
                        std::process::exit(1);
                    })
            );
        }
    }
}
