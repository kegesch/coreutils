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
use filetime::FileTime;
use std::fmt::{Debug, Display};
use std::error::Error;
use core::fmt;

type Source = PathBuf;
type Dest = PathBuf;

#[derive(Default, Clone)]
struct CopyContext {
    source: Source,
    dest: Dest
}

impl CopyContext {
    fn new(source: &Source, dest: &Dest) -> Self {
        return CopyContext {source: PathBuf::from(source), dest: PathBuf::from(dest)}
    }
}

impl Display for CopyContext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", format!("cp: {} -> {}", self.source.display(), self.dest.display()))
    }
}

impl Debug for CopyContext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <CopyContext as std::fmt::Display>::fmt(self, f)
    }
}

#[derive(Default)]
struct CopyError {
    message: String
}

impl From<String> for CopyError {
    fn from(message: String) -> Self {
        CopyError { message }
    }
}

impl From<io::Error> for CopyError {
    fn from(err: io::Error) -> Self {
        return CopyError { message: format!("io error: {}", err.description())}
    }
}

impl Display for CopyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", format!("cp: {}", self.message))
    }
}

impl Debug for CopyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <CopyError as std::fmt::Display>::fmt(self, f)
    }
}

type CopyResult = Result<CopyContext, CopyError>;

fn main() {
    let yaml = load_yaml!("cp.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let source = matches.values_of("SOURCE").unwrap();
    let dest = matches.value_of("DEST").unwrap();

    let destination_path = PathBuf::from(dest);

    let is_single_source = source.clone().count() == 1; // cp has a weird behavior when it copies a single directory

    for val in source {
        let source_path = PathBuf::from(val);
        match cp(&source_path, &destination_path, &matches, true, is_single_source) {
            Ok(_) => (),
            Err(e) => {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
    }
}

/// Copies `source`to `destination`.
fn cp(source: &Source, dest: &Dest, args: &ArgMatches, from_cmd: bool, is_single_source: bool) -> CopyResult {
    let context = CopyContext::new(source, dest);

    if !source.exists() {
        return Err(CopyError::from(String::from("source does not exist")));
    }

    if dest.is_dir() && !dest.exists() {
        return Err(CopyError::from(String::from("destination is not a directory")));
    }

    if source.eq(dest) {
        println!("cp: {} and {} are identical (not copied).", source.display(), dest.display());
        return Ok(context);
    }

    let mut result: CopyResult = Ok(context);

    if source.is_file() {
        result = copy_file(source, dest, &args, from_cmd);
    } else if source.is_dir() {
        result = copy_directory(source, dest, &args, from_cmd, is_single_source);
    }

    result
}

/// Copies the content of the directory `source` to the `destination` directory.
fn copy_directory(source: &Source, dest: &Dest, args: &ArgMatches, from_cmd: bool, is_single_source: bool) -> CopyResult {
    let mut is_recursive = args.is_present("recursive");
    let mut is_no_dereference = args.is_present("no-dereference");
    let mut is_preserve = args.is_present("preserve");
    let is_dereference = args.is_present("dereference");
    let is_deref_cmd = args.is_present("dereference-cmd");
    let is_archive = args.is_present("archive");

    let context = CopyContext::new(source, dest);

    if is_archive {
        is_recursive = true;
        is_no_dereference = true;
        is_preserve = true;
    }

    if !dest.is_dir() {
        return Err(CopyError::from(format!("destination {} must be a directory", dest.display())));
    }

    if !is_recursive {
        println!("cp: {} is a directory (not copied).", source.display());
        return Ok(context);
    }

    // normal behaviour is copies symlinks same as -P
    // -L follows the Links

    if is_symlink(source) {
        let dereference = is_dereference || (is_deref_cmd && from_cmd);

        if !dereference || is_no_dereference {
            let target = source.read_link().unwrap();
            let filename = source.file_name().unwrap();
            let new_dest = dest.join(Path::new(filename));

            if new_dest.exists() {
                eprintln!("cp: cannot overwrite directory {} with non-directory {}", new_dest.display(), source.display());
            } else {
                if args.is_present("verbose") {
                    println!("cp: {}", context);
                }
                // TODO preserve
                symlink_dir(target, new_dest);
            }
        } else {
            let deref_source = source.read_link().unwrap();
            let dirname = source.file_name().unwrap();
            let new_dir = dest.join(dirname);
            fs::create_dir(&new_dir);

            if is_preserve {
                copy_attributes(&deref_source, &new_dir);
            }

            copy_directory_content(&deref_source, &new_dir, args);
        }
    } else {
        if is_single_source && from_cmd {
            copy_directory_content(source, dest, args);
        } else {
            let dirname = source.file_name().unwrap();
            let new_dir = dest.join(dirname);
            fs::create_dir(&new_dir);

            if is_preserve {
                copy_attributes(source, &new_dir);
            }

            copy_directory_content(source, &new_dir, args);
        }
    }

    Ok(context)
}

/// Copies files from `dir` to `dest`.
fn copy_directory_content(dir: &Source, dest: &Dest, args: &ArgMatches) -> CopyResult {
    let context = CopyContext::new(dir, dest);
    if args.is_present("verbose") {
        println!("cp: {}", context);
    }
    for entry in dir.read_dir().unwrap() {
        let entry = entry.unwrap();
        let path_buf = entry.path();

        //if path.is_dir() && !is_symlink(path) {
            //let dirname = path.file_name().unwrap();
            //let new_destination = dest.join(dirname);
            //let new_dest_path = new_destination.as_path();
            //fs::create_dir(new_dest_path);

            //if args.is_present("preserve") {
            //    copy_attributes(path, new_dest_path);
            //}
            //println!("copy dir and not symlink");
            //cp(path, new_dest_path, args, false, false);
        //} else {
            //println!("copy file or symlink");
            cp(&path_buf, dest, args, false, false);
        //}
    }

    Ok(context)
}

/// Copies the attributes (permission, created, modified) from `source` to `dest`.
fn copy_attributes(source: &Source, dest: &Dest) -> CopyResult {
    let metadata = fs::metadata(source)?;
    let permissions = metadata.permissions();
    println!("set permission {:?} at {}", permissions, dest.display());
    fs::set_permissions(dest, permissions);
    println!("set times {} {} at {}", FileTime::from_last_access_time(&metadata), FileTime::from_last_modification_time(&metadata), dest.display());
    filetime::set_file_times(
        Path::new(dest),
        FileTime::from_last_access_time(&metadata),
        FileTime::from_last_modification_time(&metadata),
    )?;

    Ok(CopyContext::new(source, dest))
}

/// Copies a file with name `filename` to the `destination`.
/// While `destination` can either be a file or a directory.
fn copy_file(source: &Source, dest: &Dest, args: &ArgMatches, _from_cmd: bool) -> CopyResult {
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

    let mut new_dest = PathBuf::from(dest);

    if new_dest.is_dir() {
        let filename = source.file_name().unwrap();
        new_dest = new_dest.join(Path::new(filename));
    }

    let context = CopyContext::new(source, &new_dest);

    if is_symlink(&source) {
        if is_recursive || is_no_dereference {
            if !is_no_clobber {
                fs::remove_file(&new_dest);
            }
            symlink_copy(source, &new_dest, is_preserve);
        } else {
            let mut source_deref = source.read_link()?;

            if source_deref.is_relative() {
                if let Some(parent) = source.parent() {
                    source_deref = parent.join(source_deref);
                }
            }

            copy_file_to_file(&source_deref, &new_dest, args);
        }
    } else {
        copy_file_to_file(source, &new_dest, args);
    }

    Ok(context)
}

/// Copies file with `filename` to file location with name `dest_filename` respecting `args`
fn copy_file_to_file(filename: &Source, dest_filename: &Dest, args: &ArgMatches) -> CopyResult {
    let is_no_clobber = args.is_present("no-clobber");
    let is_forced = args.is_present("force");
    let is_interactive = args.is_present("interactive");
    let _is_preserve = args.is_present("preserve");

    let context = CopyContext::new(filename, dest_filename);

    let mut result = Ok(context);

    if dest_filename.exists() {
        if is_forced {
            fs::remove_file(dest_filename).unwrap();
            result = file_copy(filename, dest_filename, args);
        } else if is_no_clobber {
            println!("not overwriting because of option --no-clobber");
        } else if is_interactive {
            if interactive(dest_filename) {
                result = file_copy(filename, dest_filename, args);
            }
        } else {
            result = file_copy(filename, dest_filename, args);
        }
    } else {
        result = file_copy(filename, dest_filename, args);
    }

    result
}

/// copies a file from `from` to `to`
fn file_copy(from: &Source, to: &Dest, args: &ArgMatches) -> CopyResult {
    let context = CopyContext::new(from, to);

    if args.is_present("verbose") {
        println!("cp: {}", context);
    }

    if args.is_present("preserve") {
        match fs::copy(from.as_path(), to.as_path()) {
            Ok(_) => Ok(context),
            Err(_) => {
                eprintln!("cp: could not copy {} to {}", from.display(), to.display());
                Ok(context)
            },
        }
    } else {
        match fs::File::open(from.as_path()) {
            Ok(mut source_file) => {
                match fs::File::create(to.as_path()) {
                    Ok(mut new_file) => {
                        if let Ok(_) = io::copy(&mut source_file, &mut new_file) {
                            Ok(context)
                        } else {
                            eprintln!("cp: could not copy file content from {} to {}", from.display(), to.display());
                            Ok(context)
                        }
                    },
                    Err(_) => {
                        println!("cp: could not create file at {}", to.display());
                        Ok(context)
                    }
                }
            },
            Err(_) => {
                println!("cp: could not open file at {}", to.display());
                Ok(context)
            }
        }

    }
}
/// copies a symlink from `from` to `to`
fn symlink_copy(from: &Source, to: &Dest, preserve: bool) -> CopyResult {
    let target = from.read_link()?;
    let context = CopyContext::new(from, to);
    if preserve {
        println!("hardlinkin");//TODO check if this works
        match fs::hard_link(from, to) {
            Ok(_) => Ok(context),
            Err(e) => Err(CopyError::from(String::from(e.description())))
        }
    } else {
        match symlink_file(target, to) {
            Ok(_) => Ok(context),
            Err(e) => Err(CopyError::from(String::from(e.description())))
        }
    }
}

/// Requests the user if the `file` should be overwritten.
/// Returns `true` if the user answers with yes, else `false`
fn interactive(file: &Source) -> bool {
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
fn is_symlink(file: &Source) -> bool {
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