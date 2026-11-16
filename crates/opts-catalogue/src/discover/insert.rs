//! Inserting directories and files into the scan database.

use super::*;

pub(super) fn insert_directories(
    tx: &rusqlite::Transaction<'_>,
    dirs: &[scan::DirRec],
) -> Result<HashMap<String, i64>> {
    let mut dir_ids = HashMap::with_capacity(dirs.len());
    let mut st = tx.prepare(
        "INSERT INTO directories(parent_dir_id, rel_path, dir_name, depth, mtime_sec, child_dir_count, child_file_count)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )?;
    for d in dirs {
        let parent = d.parent.as_ref().and_then(|p| dir_ids.get(p)).copied();
        st.execute(params![
            parent,
            d.rel_path,
            d.name,
            d.depth,
            d.mtime,
            d.child_dirs,
            d.child_files
        ])?;
        dir_ids.insert(d.rel_path.clone(), tx.last_insert_rowid());
    }
    Ok(dir_ids)
}

pub(super) fn insert_files(
    tx: &rusqlite::Transaction<'_>,
    files: &[scan::FileRec],
    dir_ids: &HashMap<String, i64>,
) -> Result<HashMap<String, i64>> {
    let mut file_ids = HashMap::with_capacity(files.len());
    let mut st = tx.prepare(
        "INSERT INTO files(dir_id, rel_path, file_name, extension, size_bytes, mtime_sec, is_symlink, symlink_target, is_backup)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
    )?;
    for f in files {
        let Some(dir_id) = dir_ids.get(&f.dir) else {
            continue;
        };
        st.execute(params![
            dir_id,
            f.rel_path,
            f.name,
            f.ext,
            f.size,
            f.mtime,
            f.is_symlink,
            f.symlink_target,
            f.is_backup
        ])?;
        file_ids.insert(f.rel_path.to_lowercase(), tx.last_insert_rowid());
    }
    Ok(file_ids)
}

pub(super) fn build_name_index(files: &[scan::FileRec]) -> HashMap<String, Vec<String>> {
    let mut by_name: HashMap<String, Vec<String>> = HashMap::new();
    for f in files {
        by_name
            .entry(f.name.to_lowercase())
            .or_default()
            .push(f.rel_path.clone());
    }
    by_name
}
