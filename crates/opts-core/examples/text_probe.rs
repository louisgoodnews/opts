//! Lists tile objects that carry string variables (scratch investigation of `[texttexture]`).
use opts_core::formats::map::MapTile;
fn main() {
    let dir = std::env::args().nth(1).expect("map dir");
    let mut n = 0;
    for e in std::fs::read_dir(&dir).expect("dir").flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !name.starts_with("tile_") || !name.ends_with(".map") {
            continue;
        }
        let Ok(b) = std::fs::read(e.path()) else {
            continue;
        };
        let Ok(t) = MapTile::parse(&b) else { continue };
        for o in &t.objects {
            if !o.string_vars.is_empty()
                && n < 400
                && (o.sco_file.contains("hst_wartehaus_alt01")
                    || o.sco_file.contains("Name_Timetable"))
            {
                println!("{name} {} {:?} {:?}", o.sco_file, o.string_vars, o.position);
                n += 1;
            }
        }
    }
}
