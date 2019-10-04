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

// TODO -L
// TODO -P
// TODO -p
// TODO -a (= -RpP)
// TODO -H
fn main() {
    let yaml = load_yaml!("cp.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let source = matches.values_of("SOURCE").unwrap();
    let dest = matches.value_of("DEST").unwrap();

    let destination_path = Path::new(dest);

    for val in source {
        let source_path = Path::new(val);
        match cp(source_path, destination_path, &matches) {
            Ok(_) => (),
            Err(e) => {
                eprintln!("cp: could not copy\n{}", e);
                process::exit(1);
            }
        }
    }
}

/// Copies `source`to `destination`.
fn cp(source: &Path, dest: &Path, args: &ArgMatches) -> io::Result<()> {

    if !source.exists() {
        eprintln!("cp: source does not exist");
        process::exit(1);
    }

    if dest.is_dir() && !dest.exists() {
        eprintln!("cp: destination is not a directory");
        process::exit(1);
    }

    if source.is_file() {
        copy_file(source, dest, &args);
    } else if source.is_dir() {
        copy_directory(source, dest, &args);
    }

    Ok(())
}

/// Copies the content of the directory `source` to the `destination` directory.
fn copy_directory(source: &Path, destination: &Path, args: &ArgMatches) -> io::Result<()> {
    let is_recursive = args.is_present("recursive");
    let is_dereference = args.is_present("dereference");

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
        if !is_dereference {
            let target = source.read_link()?;
            let dest = destination.join(Path::new(source.file_name()?));
            symlink_dir(target, dest);
        } else {
            let deref_source = source.read_link()?;
            copy_directory_content(deref_source.as_path(), destination, args);
        }
    } else {
        copy_directory_content(deref_source.as_path(), destination, args);
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
            if is_recursive {
                let dirname = path.file_name().unwrap();
                let new_destination = dest.join(dirname);
                let new_dest_path = new_destination.as_path();
                fs::create_dir(new_dest_path);
                cp(path, new_dest_path, args);
            }
        } else {
            cp(path, dest, args);
        }
    }

    Ok(())
}

/// Copies a file with name `filename` to the `destination`.
/// While `destination` can either be a file or a directory.
fn copy_file(filename: &Path, destination: &Path, args: &ArgMatches) -> io::Result<()> {
    // destination could be a directory or a filename
    println!("Copy file");

    // normal behaviour is symlinks are followed

    if destination.is_dir() {
        let source_name = filename.file_name().unwrap();
        let path_source_name = Path::new(source_name);
        let destination_filename_buffered = destination.join(path_source_name);
        copy_file_to_file(filename, destination_filename_buffered.as_path(), args);
    } else {
        copy_file_to_file(filename, destination, args);
    }
    Ok(())
}

/// Copies a symlink `filename` to `dest_filename`
fn copy_symlink(filename: &Path, dest_filename: &Path, args: &ArgMatches) -> io::Result<()> {

    let mut dest_path_buf = PathBuf::from(filename);

    if dest_filename.is_dir() {
        let source_name = filename.file_name().unwrap();
        let path_source_name = Path::new(source_name);
        dest_path_buf = dest_filename.join(path_source_name);
    }

    let dest_path = dest_path_buf.as_path();

    println!("Copy symlink {} to {}", filename.display(), dest_filename.display());
    let target_path_buf = filename.read_link()?;
    let target_path_rel = target_path_buf.canonicalize()?;
    let target_path = target_path_rel.as_path();

    println!("creating symlink at {} pointing to {}", dest_path.display(), target_path.display());

    let res = match filename.is_file() {
        true => symlink_file(target_path, dest_path),
        false => symlink_dir(target_path, dest_path)
    };
    println!("was successful: {:?}", res);

    res
}

/// Copies file with `filename` to file location with name `dest_filename` respecting `args`
fn copy_file_to_file(filename: &Path, dest_filename: &Path, args: &ArgMatches) -> io::Result<()> {
    println!("Copy {} to {}", filename.display(), dest_filename.display());

    // TODO make sure file filename and dest_filename are the same, copy fails
    // TODO respect order of -n -f and -i
    // TODO check if copy respects its metadata
    let mut is_no_clobber = args.is_present("no-clobber");
    let is_forced = args.is_present("force");
    let is_interactive = args.is_present("interactive");

    if is_forced || is_interactive {
        is_no_clobber = false;
    }

    let real_path = get_path(filename, args);

    if dest_filename.exists() {
        if is_forced {
            match fs::OpenOptions::new().write(true)
            .open(dest_filename) {
                Ok(_) => (),
                Err(_) => {
                    if is_interactive && interactive(dest_filename) {
                        fs::remove_file(dest_filename)?;
                        copy_or_link(real_path, dest_filename, args);
                    }
                }
            }
        } else {
            println!("Target alread exists: no-clobber {}, is-interactive {}", is_no_clobber, is_interactive);
            if is_interactive && interactive(dest_filename) || !is_no_clobber {
                copy_or_link(real_path, dest_filename, args);
            }
        }
    } else {
        copy_or_link(real_path, dest_filename, args);
    }

    Ok(())
}

/// Copies a file or creates a symlink
fn copy_or_link<A: AsRef<Path>, B: AsRef<Path>>(from: A, to: B, args: &ArgMatches)-> io::Result<u64> {
    let from_path: &Path = from.as_ref();
    let to_path: &Path = to.as_ref();

    if args.is_present("verbose") {
        println!("cp: {} -> {}", from_path.display(), to_path.display());
    }

    if is_symlink(from_path) && !should_dereference(args) {
        let target_path_buf = from_path.read_link()?;
        let target_path_rel = target_path_buf.canonicalize()?;
        let target_path = target_path_rel.as_path();

        println!("target is relative {} or absolute {}", target_path.is_relative(), target_path.is_absolute());
        println!("creating symlink at {} pointing to {}", to_path.display(), target_path.display());
        let res = symlink::symlink_auto(target_path, from_path);
        println!("was successful: {:?}", res);
    } {
        fs::copy(from_path, to_path)
    }
}

/// Requests the user if the `file` should be overwritten.
/// Returns `true` if the user answers with yes, else `false`
fn interactive(file: &Path) -> bool {
    let name = file.file_name().unwrap();
    let name_str = name.to_str().unwrap();

    println!("overwrite {}? (y/n [n])", name_str);

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

/// Parses `args` and tells whether cp should dereference or not.
fn should_dereference(args: &ArgMatches) -> bool {
    let is_no_dereference = args.is_present("no-dereference");
    let is_dereference = args.is_present("dereference");

    return is_dereference && !is_no_dereference;
}

/// Returns the Path to `file`. If `file` is a symlink and dereferences is set, it returns the dereferenced Path.
/// If not it returns `file`.
fn get_path(file:  &Path, args: &ArgMatches) -> PathBuf{
    if should_dereference(args) && is_symlink(file) {
        if let Ok(path_buf) = file.read_link() {
            return path_buf;
        } else {
            eprintln!("cp: could not read link {}", file.to_str().unwrap());
            return PathBuf::from(file);
        }
    } else {
        return PathBuf::from(file);
    }
}