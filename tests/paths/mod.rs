use std::path::PathBuf;
use std::process::Stdio;
use std::sync::LazyLock;

static CARGO_ARTIFACTS: LazyLock<Vec<cargo_metadata::Artifact>> =
    LazyLock::<Vec<cargo_metadata::Artifact>>::new(|| {
        let mut cargo = std::process::Command::new("cargo")
            .args(vec!["test", "--no-run", "--message-format", "json"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("Cargo should provide info about tests");
        let reader = std::io::BufReader::new(cargo.stdout.take().unwrap());

        let mut artifacts = vec![];
        for message in cargo_metadata::Message::parse_stream(reader) {
            if let cargo_metadata::Message::CompilerArtifact(artifact) = message.unwrap() {
                artifacts.push(artifact);
            }
        }

        if cargo.wait().is_err() {
            panic!("Cargo failed!");
        }
        artifacts
    });

pub static MOCKSMITH_PATH: LazyLock<PathBuf> = LazyLock::<PathBuf>::new(|| {
    let artifact = CARGO_ARTIFACTS
        .iter()
        .find(|artifact| {
            artifact.target.name == "mocksmith"
                && artifact.target.is_bin()
                && !artifact.profile.test
        })
        .expect("No mocksmith artifact provided by cargo");
    let path = artifact
        .executable
        .as_ref()
        .expect("mocksmith artifact should have an executable");
    path.into()
});
