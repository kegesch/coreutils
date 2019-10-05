use clap::{
    App,
    load_yaml,
    ArgMatches
};
use std::{
    fs,
    io,
    path::{
        Path,
        PathBuf
    },
    process
};
use symlink::{
    symlink_file,
    symlink_dir
};

fn main() {
    let yaml = load_yaml!("cp.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let source = matches.values_of("SOURCE").unwrap();
    let dest = matches.value_of("DEST").unwrap();

    let destination_path = Path::new(dest);

    for val in source {
        let source_path = Path::new(val);
        match cp(source_path, destination_path, &matches, true) {
            Ok(_) => (),
            Err(e) => {
                eprintln!("cp: could not copy\n{}", e);
                process::exit(1);
            }
        }
    }
}

/// Copies `source`to `destination`.
fn cp(source: &Path, dest: &Path, args: &ArgMatches, from_cmd: bool) -> io::Result<()> {

    if !source.exists() {
        eprintln!("cp: source does not exist");
        process::exit(1);
    }

    if dest.is_dir() && !dest.exists() {
        eprintln!("cp: destination is not a directory");
        process::exit(1);
    }

    if source.eq(dest) {
        println!("cp: {} and {} are identical (not copied).", source.display(), dest.display());
        return Ok(());
    }

    if source.is_file() {
        copy_file(source, dest, &args, from_cmd);
    } else if source.is_dir() {
        copy_directory(source, dest, &args, from_cmd);
    }

    Ok(())
}

/// Copies the content of the directory `source` to the `destination` directory.
fn copy_directory(source: &Path, destination: &Path, args: &ArgMatches, from_cmd: bool) -> io::Result<()> {
    let mut is_recursive = args.is_present("recursive");
    let is_dereference = args.is_present("dereference");
    let mut is_no_dereference = args.is_present("no-dereference");
    let mut _is_preserve = args.is_present("preserve");
    let is_deref_cmd = args.is_present("dereference-cmd");
    let is_archive = args.is_present("archive");

    if is_archive {
        is_recursive = true;
        is_no_dereference = true;
        _is_preserve = true;
    }

    if !destination.is_dir() {
        eprintln!("cp: destination {} must be a directory", destination.display());
        process::exit(1);
    }

    if !is_recursive {
        println!("cp: {} is a directory (not copied).", source.display());
        return Ok(());
    }

    // normal behaviour is copies symlinks same as -P
    // -L follows the Links

    if is_symlink(source) {
        let dereference = is_dereference || (is_deref_cmd && from_cmd);

        if !dereference || is_no_dereference {
            let target = source.read_link()?;
            let filename = source.file_name().unwrap();
            let dest = destination.join(Path::new(filename));

            if dest.exists() {
                eprintln!("cp: cannot overwrite directory {} with non-directory {}", dest.display(), source.display());
            } else {
                symlink_dir(target, dest);
            }
        } else {
            let deref_source = source.read_link()?;
            let dirname = source.file_name().unwrap();
            let new_dir = destination.join(dirname);
            let new_dir_path = new_dir.as_path();
            fs::create_dir(new_dir_path);
            copy_directory_content(deref_source.as_path(), new_dir_path, args);
        }
    } else {
        let dirname = source.file_name().unwrap();
        let new_dir = destination.join(dirname);
        let new_dir_path = new_dir.as_path();
        fs::create_dir(new_dir_path);
        copy_directory_content(source, new_dir_path, args);
    }

    Ok(())
}

/// Copies files from `dir` to `dest`.
fn copy_directory_content(dir: &Path, dest: &Path, args: &ArgMatches) -> io::Result<()> {
    for entry in dir.read_dir()? {
        let entry = entry?;
        let path_buf = entry.path();
        let path = path_buf.as_path();

        if path.is_dir() {
            let dirname = path.file_name().unwrap();
            let new_destination = dest.join(dirname);
            let new_dest_path = new_destination.as_path();
            fs::create_dir(new_dest_path);
            // TODO copy metadata if preserved
            cp(path, new_dest_path, args, false);
        } else {
            cp(path, dest, args, false);
        }
    }

    Ok(())
}

/// Copies a file with name `filename` to the `destination`.
/// While `destination` can either be a file or a directory.
fn copy_file(source: &Path, destination: &Path, args: &ArgMatches, _from_cmd: bool) -> io::Result<()> {
    let mut is_recursive = args.is_present("recursive");
    // let is_dereference = args.is_present("dereference");
    let mut is_no_dereference = args.is_present("no-dereference");
    let is_no_clobber = args.is_present("no-clobber");
    let mut is_preserve = args.is_present("preserve");
    let is_archive = args.is_present("archive");

    if is_archive {
        is_recursive = true;
        is_no_dereference = true;
        is_preserve = true;
    }

    // normal behaviour is symlinks are followed

    let mut dest = PathBuf::from(destination);

    if destination.is_dir() {
        let filename = source.file_name().unwrap();
        dest = destination.join(Path::new(filename));
    }

    if is_symlink(source) {
        if is_recursive || is_no_dereference {
            let dest_ref = dest.as_path();
            if !is_no_clobber {
                fs::remove_file(dest_ref);
            }
            symlink_copy(source, dest_ref, is_preserve);
        } else {
            let source_deref = source.read_link()?;
            copy_file_to_file(source_deref.as_path(), dest.as_path(), args);
        }
    } else {
        copy_file_to_file(source, dest.as_path(), args);
    }

    Ok(())
}

/// Copies file with `filename` to file location with name `dest_filename` respecting `args`
fn copy_file_to_file(filename: &Path, dest_filename: &Path, args: &ArgMatches) -> io::Result<u64> {
    if args.is_present("verbose") {
        println!("cp: {} -> {}", filename.display(), dest_filename.display());
    }

    let is_no_clobber = args.is_present("no-clobber");
    let is_forced = args.is_present("force");
    let is_interactive = args.is_present("interactive");
    let is_preserve = args.is_present("preserve");

    let mut result = Ok(0);

    if dest_filename.exists() {
        if is_forced {
            fs::remove_file(dest_filename)?;
            result = file_copy(filename, dest_filename, is_preserve);
        } else if is_no_clobber {
            // DO NOT OVERWRITE
            return Ok(0);
        } else if is_interactive {
            if interactive(dest_filename) {
                result = file_copy(filename, dest_filename, is_preserve);
            }
        } else {
            result = file_copy(filename, dest_filename, is_preserve);
        }
    } else {
        result = file_copy(filename, dest_filename, is_preserve);
    }

    result
}

/// copies a file from `from` to `to`
fn file_copy<A: AsRef<Path>, B: AsRef<Path>>(from: A, to: B, preserve: bool) -> io::Result<u64> {
    if preserve {
        fs::copy(from, to)
    } else {
        let mut source_file = fs::File::open(from)?;
        let mut new_file = fs::File::create(to)?;

        io::copy(&mut source_file, &mut new_file)
    }
}
/// copies a symlink from `from` to `to`
fn symlink_copy<A: AsRef<Path>, B: AsRef<Path>>(from: A, to: B, preserve: bool) -> io::Result<()> {
    let target_path = from.as_ref();
    let target = target_path.read_link()?;

    if preserve {
        // TODO preserve metadata of symlink
    }

    symlink_file(target, to)
}

/// Requests the user if the `file` should be overwritten.
/// Returns `true` if the user answers with yes, else `false`
fn interactive(file: &Path) -> bool {
    print!("overwrite {}? (y/n [n])", file.display());

    let mut buffer = String::new();
    match io::stdin().read_line(&mut buffer) {
        Ok(_) => {},
        Err(_) => {
            return false;
        }
    }
    buffer.starts_with("y")
}

/// Return `true` if `file` is a symbolic link.
fn is_symlink(file: &Path) -> bool {
    match file.symlink_metadata() {
        Ok(metadata) => {
            let file_type = metadata.file_type();
            return file_type.is_symlink();
        },
        Err(_) => {
            eprintln!("cp: could not retrieve metadata for {}. Permission denied.", file.file_name().unwrap().to_str().unwrap());
            return false;
        }
    }
}