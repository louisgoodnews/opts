//! Prints a short report on degenerate triangles of the given `.o3d` files (P-1014 triage).
use opts_omsi::formats::o3d::parse;
fn main() {
    for a in std::env::args().skip(1) {
        let o = parse(&std::fs::read(&a).unwrap()).unwrap();
        let (mut rep, mut zero, mut tiny) = (0, 0, 0);
        let mut ex = Vec::new();
        for (i, t) in o.indices.iter().enumerate() {
            if t[0] == t[1] || t[1] == t[2] || t[0] == t[2] {
                rep += 1;
                continue;
            }
            let p = t.map(|k| o.vertices[k as usize].position);
            let ar = (p[1] - p[0]).cross(p[2] - p[0]).length();
            if ar == 0.0 {
                zero += 1;
            } else if ar < 1e-9 {
                tiny += 1;
            } else {
                continue;
            }
            if ex.len() < 3 {
                ex.push((i, *t, p));
            }
        }
        let maxidx = o.indices.iter().flatten().max();
        println!(
            "{a}: nv={} nt={} repeated={rep} zero_area={zero} tiny={tiny} max_index={maxidx:?}",
            o.vertices.len(),
            o.indices.len()
        );
        for e in ex {
            println!("   {e:?}");
        }
    }
}
