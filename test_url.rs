use url::Url;
fn main() {
    let u = Url::parse("http://example.com/foo/../bar").unwrap();
    println!("Path: {}", u.path());
    if let Some(segs) = u.path_segments() {
        for seg in segs {
            println!("Seg: '{}'", seg);
        }
    }
    
    let u2 = Url::parse("http://example.com/%2e%2e/etc/passwd").unwrap();
    println!("Path 2: {}", u2.path());
    if let Some(segs) = u2.path_segments() {
        for seg in segs {
            println!("Seg 2: '{}'", seg);
        }
    }
}
