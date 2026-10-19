//! Parses every `.oft` under a directory and reports failures (scratch investigation).
use opts_omsi::formats::oft::OftFile;
fn walk(d: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("oft")) {
            out.push(p);
        }
    }
}
fn main() {
    let dir = std::env::args().nth(1).expect("dir");
    let mut files = Vec::new();
    walk(std::path::Path::new(&dir), &mut files);
    let (mut ok, mut fonts, mut chars) = (0, 0, 0);
    for f in &files {
        match OftFile::parse(&std::fs::read(f).expect("read")) {
            Ok(o) => {
                ok += 1;
                fonts += o.fonts.len();
                chars += o.fonts.iter().map(|x| x.chars.len()).sum::<usize>();
                if f.file_name().is_some_and(|n| n == "StreetBerlin.oft") {
                    let x = &o.fonts[0];
                    println!(
                        "{} {} {} lh={} gap={} chars={} A={:?}",
                        x.name,
                        x.bitmap,
                        x.alpha,
                        x.line_height,
                        x.gap,
                        x.chars.len(),
                        x.chars.get(&'A')
                    );
                }
            }
            Err(e) => println!("FAIL {}: {e}", f.display()),
        }
    }
    println!(
        "{} files, {ok} ok, {fonts} fonts, {chars} chars",
        files.len()
    );
}
