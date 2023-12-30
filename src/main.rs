use hecs_game::{AssetManager, FileProtocol};


fn main() {
    let assets = AssetManager::builder()
        .default_protocol(FileProtocol)
        .build();
}