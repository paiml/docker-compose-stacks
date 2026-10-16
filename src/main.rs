use std::net::TcpListener;
use std::path::PathBuf;

fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    if std::env::args().nth(1).as_deref() == Some("--health") {
        std::process::exit(if stackd::probe(&format!("127.0.0.1:{port}")) {
            0
        } else {
            1
        });
    }
    let data =
        PathBuf::from(std::env::var("DATA_FILE").unwrap_or_else(|_| "/data/count".to_string()));
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).expect("bind");
    stackd::serve(&listener, &data, None).expect("serve");
}
