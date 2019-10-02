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
use std::fs::copy;

fn main() {
    let yaml = load_yaml!("cp.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let source = matches.value_of("SOURCE").unwrap();
    let dest = matches.value_of("DEST").unwrap();

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

    let result = match source_path.is_file() {
        true => copy_file(source_path, destination_path, &matches),
        false => copy_directory(source_path, destination_path, &matches)
    };

    match result {
        Ok(_) => (),
        Err(e) => {
            eprintln!("cp: could not copy\n{}", e);
            process::exit(1);
        }
    }
}

fn copy_directory(source: &Path, destination: &Path, args: &ArgMatches) -> io::Result<()> {
    // destination must be directoryname!
    println!("Copy directory");
    if !destination.is_dir() {
        eprintln!("cp: destination must be a directory");
        process::exit(1);
    }

    for entry in source.read_dir()? {
        let entry = entry?;
        let path_buf = entry.path();
        let path = path_buf.as_path();
        if path.is_dir() {
            let dirname = path.file_name().unwrap();
            let new_destination = destination.join(dirname);
            let new_dest_path = new_destination.as_path();
            fs::create_dir(new_dest_path);
            copy_directory(path, new_dest_path, args);
        } else {
            copy_file(path, destination, args);
        }
    }

    Ok(())
}

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

fn copy_file_to_file(filename: &Path, dest_filename: &Path, args: &ArgMatches) -> io::Result<()> {
    println!("Copy {} to {}", filename.display(), dest_filename.display());
    fs::copy(filename, dest_filename);
    Ok(())
}

