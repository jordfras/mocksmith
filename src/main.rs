use std::io::Read;

use mocksmith::MockSmith;

fn main() {
    let mocksmith = MockSmith::new()
        .unwrap_or_else(|message| panic!("Could not create MockSmith instance: {message}"));

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
            .into_iter()
            .for_each(|mock| {
                println!("{}", mock);
            });
    } else {
        for arg in std::env::args().skip(1) {
            mocksmith
                .create_mocks_for_file(std::path::Path::new(&arg))
                .into_iter()
                .for_each(|mock| {
                    println!("{}", mock);
                });
        }
    }
}
