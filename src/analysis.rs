use std::path::PathBuf;

pub fn native_engine_path() -> Option<PathBuf> {
    let maps = std::fs::read_to_string("/proc/self/maps").ok()?;
    engine_path_from_maps(&maps)
}

fn engine_path_from_maps(maps: &str) -> Option<PathBuf> {
    let libmain = maps.lines().find_map(|line| {
        let path = line.split_whitespace().last()?;
        path.ends_with("/libmain.so").then_some(path)
    })?;

    Some(PathBuf::from(libmain).parent()?.join("libkatago.so"))
}

#[cfg(test)]
mod tests {
    use super::engine_path_from_maps;
    use std::path::PathBuf;

    #[test]
    fn finds_engine_next_to_libmain() {
        let maps = "7f00-7f01 r-xp 00000000 00:00 0 /data/app/pkg/lib/x86_64/libmain.so\n";
        assert_eq!(
            engine_path_from_maps(maps),
            Some(PathBuf::from("/data/app/pkg/lib/x86_64/libkatago.so"))
        );
    }

    #[test]
    fn ignores_maps_without_libmain() {
        assert_eq!(engine_path_from_maps("no lib here"), None);
    }
}
