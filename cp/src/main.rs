use clap::{
    App,
    load_yaml,
    ArgMatches
};
use std::{
    fs,
    io,
    path::Path,
    process
};
use std::fs::{copy, Metadata};
use std::io::Read;

// TODO -L
// TODO -P
// TODO -p
fn main() {
    let yaml = load_yaml!("cp.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let source = matches.values_of("SOURCE").unwrap();
    let dest = matches.value_of("DEST").unwrap();

    for val in source {
        match cp(val, dest, &matches) {
            Ok(_) => (),
            Err(e) => {
                eprintln!("cp: could not copy\n{}", e);
                process::exit(1);
            }
        }
    }
}

/// Copies `source`to `destination`.
fn cp(source: &str, dest: &str, args: &ArgMatches) -> io::Result<()> {
    let source_path = Path::new(source);
    let destination_path = Path::new(dest);

    if !source_path.exists() {
        eprintln!("cp: source does not exist");
        process::exit(1);
    }

    if destination_path.is_dir() && !destination_path.exists() {
        eprintln!("cp: destination is not a directory");
        process::exit(1);
    }

    let result = match (should_dereference(&args) && is_symlink(source_path)) || source_path.is_file() {
        true => copy_file(source_path, destination_path, &args),
        false => copy_directory(source_path, destination_path, &args)
    };

    return result
}

/// Copies the content of the directory `source` to the `destination` directory.
fn copy_directory(source: &Path, destination: &Path, args: &ArgMatches) -> io::Result<()> {
    // destination must be directoryname!
    println!("Copy directory");

    let is_recursive = args.is_present("recursive");

    if !destination.is_dir() {
        eprintln!("cp: destination must be a directory");
        process::exit(1);
    }

    for entry in source.read_dir()? {
        let entry = entry?;
        let path_buf = entry.path();
        let mut path = path_buf.as_path();
        path = get_path(path, args);

        if path.is_dir() {
            if is_recursive {
                let dirname = path.file_name().unwrap();
                let new_destination = destination.join(dirname);
                let new_dest_path = new_destination.as_path();
                fs::create_dir(new_dest_path);
                copy_directory(path, new_dest_path, args);
            }
        } else {
            copy_file(path, destination, args);
        }
    }

    Ok(())
}

/// Copies a file with name `filename` to the `destination`.
/// While `destination` can either be a file or a directory.
fn copy_file(filename: &Path, destination: &Path, args: &ArgMatches) -> io::Result<()> {
    // destination could be a directory or a filename
    println!("Copy file");
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

/// Copies file with `filename` to file location with name `dest_filename` respecting `args`
fn copy_file_to_file(filename: &Path, dest_filename: &Path, args: &ArgMatches) -> io::Result<()> {
    println!("Copy {} to {}", filename.display(), dest_filename.display());

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
                        fs::copy(real_path, dest_filename);
                    }
                }
            }
        } else {
            if !is_no_clobber && (is_interactive && interactive(dest_filename)) {
                fs::copy(real_path, dest_filename);
            }
        }
    } else {
        fs::copy(real_path, dest_filename);
    }

    Ok(())
}

/// Requests the user if the `file` should be overwritten.
/// Returns `true` if the user answers with yes, else `false`
fn interactive(file: &Path) -> bool {
    let name = file.file_name().unwrap();
    let name_str = name.to_str().unwrap();

    println!("overwrite {}? (y/n [n])", name_str);

    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer);
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
fn get_path<'a>(file: &'a Path, args: &ArgMatches) -> &'a Path {
    if should_dereference(args) && is_symlink(file) {
        if let Ok(path_buf) = file.read_link() {
            let path = path_buf.as_path();
            return path;
        } else {
            eprintln!("cp: could not read link {}", file.to_str().unwrap());
            return file;
        }
    } else {
        return file;
    }
}

fn is_dir(file: &Path, args: &ArgMatches) -> bool {
    if should_dereference(args) && is_symlink(file) {
        return get_path(file, args).is_dir();
    } else {
        return file.is_dir();
    }
}

fn is_file(file: &Path, args: &ArgMatches) -> bool {
    if should_dereference(args) && is_symlink(file) {
        return get_path(file, args).is_file();
    } else {
        return file.is_file();
    }
}