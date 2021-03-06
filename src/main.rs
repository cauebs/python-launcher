// https://docs.python.org/3.8/using/windows.html#python-launcher-for-windows
// https://github.com/python/cpython/blob/master/PC/launcher.c

extern crate python_launcher;

use std::{collections::HashMap, env, fs::File, path::PathBuf, process::Command};

use python_launcher as py;

fn main() {
    let mut args = env::args().collect::<Vec<String>>();
    args.remove(0); // Strip the path to this executable.
    let mut requested_version = py::RequestedVersion::Any;

    if !args.is_empty() {
        if args[0].starts_with('-') {
            if let Some(version) = py::version_from_flag(&args[0]) {
                requested_version = version;
                args.remove(0);
            }
        } else if let Ok(open_file) = File::open(&args[0]) {
            if let Some(shebang) = py::find_shebang(open_file) {
                if let Some((shebang_version, mut extra_args)) = py::split_shebang(&shebang) {
                    requested_version = shebang_version;
                    extra_args.append(&mut args);
                    args = extra_args;
                }
            }
        }
    }

    if requested_version == py::RequestedVersion::Any {
        if let Some(venv_root) = env::var_os("VIRTUAL_ENV") {
            let mut path = PathBuf::new();
            path.push(venv_root);
            path.push("bin");
            path.push("python");
            // TODO: is_file() check?
            if let Err(e) = Command::new(path).args(args).status() {
                println!("{:?}", e);
            }
            return;
        }
    }

    use py::RequestedVersion::*;

    requested_version = match requested_version {
        Any => py::check_default_env_var().unwrap_or(requested_version),
        Loose(major) => py::check_major_env_var(major).unwrap_or(requested_version),
        Exact(_, _) => requested_version,
    };

    let mut found_versions = HashMap::new();
    for path in py::path_entries() {
        let all_contents = py::directory_contents(&path);

        for (version, path) in py::filter_python_executables(all_contents) {
            match version.matches(&requested_version) {
                py::VersionMatch::NotAtAll => continue,
                py::VersionMatch::Loosely => {
                    if path.is_file() {
                        found_versions.entry(version).or_insert(path);
                    }
                }
                py::VersionMatch::Exactly => {
                    if path.is_file() {
                        found_versions.insert(version, path);
                        break;
                    }
                }
            };
        }
    }

    let chosen_path = py::choose_executable(&found_versions).unwrap();
    if let Err(e) = Command::new(chosen_path).args(args).status() {
        println!("{:?}", e);
    }
}
