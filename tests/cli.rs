use stackd::{handle, load_count, probe, response, route, save_count, serve};
use std::io::{Cursor, Read, Write};
use std::net::{TcpListener, TcpStream};

#[test]
fn health_is_ok_and_does_not_count() {
    let mut n = 5;
    assert_eq!(
        route("GET /health HTTP/1.0\r\n", &mut n),
        (200, "ok".to_string())
    );
    assert_eq!(n, 5);
}

#[test]
fn hit_increments() {
    let mut n = 0;
    assert_eq!(route("GET /hit HTTP/1.0", &mut n), (200, "1".to_string()));
    assert_eq!(route("GET /hit HTTP/1.0", &mut n), (200, "2".to_string()));
}

#[test]
fn anything_else_is_404() {
    let mut n = 0;
    assert_eq!(route("POST /x HTTP/1.0", &mut n).0, 404);
}

#[test]
fn response_has_length_and_reason() {
    assert_eq!(
        response(200, "ok"),
        "HTTP/1.0 200 OK\r\nContent-Length: 2\r\n\r\nok"
    );
    assert!(response(404, "no").starts_with("HTTP/1.0 404 Not Found"));
}

#[test]
fn counter_file_round_trips_and_defaults_to_zero() {
    let dir = std::env::temp_dir().join("stackd-roundtrip");
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("count");
    let _ = std::fs::remove_file(&f);
    assert_eq!(load_count(&f), 0);
    save_count(&f, 41).unwrap();
    assert_eq!(load_count(&f), 41);
    std::fs::write(&f, "junk").unwrap();
    assert_eq!(load_count(&f), 0);
}

struct Duplex {
    input: Cursor<Vec<u8>>,
    output: Vec<u8>,
}
impl Read for Duplex {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        self.input.read(b)
    }
}
impl Write for Duplex {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.output.write(b)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn handle_answers_a_request_on_any_stream() {
    let mut n = 9;
    let s = Duplex {
        input: Cursor::new(b"GET /hit HTTP/1.0\r\n\r\n".to_vec()),
        output: vec![],
    };
    let mut s2 = s;
    let mut reader_side = Duplex {
        input: Cursor::new(s2.input.get_ref().clone()),
        output: vec![],
    };
    handle(&mut reader_side, &mut n).unwrap();
    assert_eq!(n, 10);
    assert!(String::from_utf8(reader_side.output)
        .unwrap()
        .ends_with("\r\n\r\n10"));
    s2.flush().unwrap();
}

#[test]
fn serve_counts_over_tcp_and_persists() {
    let dir = std::env::temp_dir().join("stackd-serve");
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("count");
    save_count(&f, 7).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let f2 = f.clone();
    let t = std::thread::spawn(move || serve(&listener, &f2, Some(2)).unwrap());
    for want in ["8", "9"] {
        let mut s = TcpStream::connect(&addr).unwrap();
        s.write_all(b"GET /hit HTTP/1.0\r\n\r\n").unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        assert!(out.ends_with(want), "{out}");
    }
    t.join().unwrap();
    assert_eq!(load_count(&f), 9);
}

#[test]
fn probe_true_when_healthy_false_otherwise() {
    let dir = std::env::temp_dir().join("stackd-probe");
    std::fs::create_dir_all(&dir).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let f = dir.join("count");
    let t = std::thread::spawn(move || serve(&listener, &f, Some(1)).unwrap());
    assert!(probe(&addr));
    t.join().unwrap();
    assert!(!probe(&addr));
}

#[test]
fn probe_false_when_the_answer_is_not_200() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let t = std::thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        let mut b = [0u8; 64];
        let _ = s.read(&mut b);
        s.write_all(b"HTTP/1.0 500 Oops\r\n\r\n").unwrap();
    });
    assert!(!probe(&addr));
    t.join().unwrap();
}

#[test]
#[ignore]
fn calls_nothing() {}
