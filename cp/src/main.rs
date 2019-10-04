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
    let is_no_dereference = args.is_present("no-dereference");

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
        if !is_dereference || is_no_dereference {
            let target = source.read_link()?;
            let filename = source.file_name().unwrap();
            let dest = destination.join(Path::new(filename));
            symlink_dir(target, dest);
        } else {
            let deref_source = source.read_link()?;
            copy_directory_content(deref_source.as_path(), destination, args);
        }
    } else {
        copy_directory_content(source, destination, args);
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
            // TODO make sure that the new dir has the same mode as source dir
            cp(path, new_dest_path, args);
        } else {
            cp(path, dest, args);
        }
    }

    Ok(())
}

/// Copies a file with name `filename` to the `destination`.
/// While `destination` can either be a file or a directory.
fn copy_file(source: &Path, destination: &Path, args: &ArgMatches) -> io::Result<()> {
    let is_recursive = args.is_present("recursive");
    let is_dereference = args.is_present("dereference");
    let is_no_dereference = args.is_present("no-dereference");

    // normal behaviour is symlinks are followed

    let mut dest = PathBuf::from(destination);

    if destination.is_dir() {
        let filename = source.file_name().unwrap();
        dest = destination.join(Path::new(filename));
    }

    if is_symlink(source) {
        if is_recursive || is_no_dereference {
            let target = source.read_link()?;
            symlink_file(target, dest);
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
    // TODO make sure file filename and dest_filename are the same, copy fails
    // TODO respect order of -n -f and -i
    // TODO check if copy respects its metadata
    let is_no_clobber = args.is_present("no-clobber");
    let is_forced = args.is_present("force");
    let is_interactive = args.is_present("interactive");

    let mut result = Ok(0);

    if dest_filename.exists() {
        if is_forced {
            fs::remove_file(dest_filename)?;
            result = fs::copy(filename, dest_filename);
        }
        if is_no_clobber {
            // DO NOT OVERWRITE
            return Ok(0);
        }
        if is_interactive && interactive(dest_filename) {
            result = fs::copy(filename, dest_filename);
        }
    } else {
        result = fs::copy(filename, dest_filename);
    }

    result
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