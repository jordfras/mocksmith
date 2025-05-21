use mocksmith::MockSmith;

fn main() {
    let mocksmith = MockSmith::new();
    for arg in std::env::args().skip(1) {
        mocksmith
            .create_mocks_for_file(std::path::Path::new(&arg))
            .into_iter()
            .for_each(|mock| {
                println!("{}", mock);
            });
    }
}
