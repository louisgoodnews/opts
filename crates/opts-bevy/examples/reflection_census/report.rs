//! Report printing of the reflection census example.

use std::collections::BTreeSet;

use super::{Census, Counts};

impl Census {
    pub(super) fn print_report(&self, files_walked: usize) {
        println!(
            "files walked: {files_walked}  text files read: {}",
            self.text_files
        );
        println!(
            "vehicle folders: {}  object folders: {}  .sco files: {}",
            self.vehicles.len(),
            self.objects.len(),
            self.sco_files
        );
        println!(
            "materials ([matl]): {} in {} files",
            self.total_matl,
            self.total_matl_files.len()
        );
        println!("\n== keyword | uses | files | vehicle/object units");
        for (k, t) in &self.kw {
            let v = t.units.iter().filter(|u| u.starts_with("V:")).count();
            let o = t.units.iter().filter(|u| u.starts_with("O:")).count();
            println!(
                "[{k}] | {} | {} | {v} vehicle dirs, {o} object dirs, {} other",
                t.count,
                t.files.len(),
                t.units.len() - v - o
            );
        }
        println!("\n== [matl_alpha] values (all)");
        for (k, v) in &self.alpha_vals {
            println!("{k:?}: {v}");
        }
        self.print_materials();
        println!("\n== name hits");
        for (k, v) in &self.name_hits {
            let u: BTreeSet<&str> = v.iter().map(String::as_str).collect();
            println!("{k}: {} hits", v.len());
            for s in u.iter().take(12) {
                println!("   {s}");
            }
        }
    }

    fn print_materials(&self) {
        println!("\n== [matl_envmap] texture (top 15)");
        top(&self.envmap_tex, 15);
        println!("\n== [matl_envmap] factor (top 15)");
        top(&self.envmap_factor, 15);
        println!(
            "\n== glass-named materials: {} (with envmap {})",
            self.glass_mats, self.glass_env
        );
        println!("alpha mode of glass-named materials:");
        top(&self.glass_alpha, 10);
        println!("glass texture names (top 15):");
        top(&self.glass_tex, 15);
        println!(
            "\n== mirror-named materials: {} (with envmap {})",
            self.mirror_mats, self.mirror_env
        );
        top(&self.mirror_tex, 15);
        println!(
            "\n== env-mapped materials that are neither glass- nor mirror-named: {}",
            self.env_nonglass_mats
        );
        println!("\n== envmap/reflect-named files (top 10)");
        top(&self.env_files_bmp, 10);
    }
}

fn top(m: &Counts, n: usize) {
    let mut v: Vec<_> = m.iter().collect();
    v.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
    for (k, c) in v.into_iter().take(n) {
        println!("   {c:>6}  {k}");
    }
}
