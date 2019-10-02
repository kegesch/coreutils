use clap::{App, load_yaml, ArgMatches};
use std::{
    io,
    path::Path,
    process
};

fn main() {
    let yaml = load_yaml!("cp.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let source = matches.value_of("SOURCE").unwrap();
    let dest = matches.value_of("DEST").unwrap();

    let source_path = Path::new(source);
    let destination_path = Path::new(dest);

    if !source_path.exists() {
        eprintln!("Cp: Source does not exist");
        process::exit(1);
    }

    if !destination_path.exists() {
        eprintln!("Cp: Destination does not exist");
        process::exit(1);
    }

    let result = match source_path.is_file() {
        true => copy_file(source_path, destination_path, &matches),
        false => copy_directory(source_path, destination_path, &matches)
    };

    match result {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Cp: Could not copy\n{}", e);
            process::exit(1);
        }
    }
}

fn copy_directory(directoryname: &Path, destination: &Path, args: &ArgMatches) -> io::Result<()> {
    // destination must be directoryname!
    println!("Copy directory");
    Ok(())
}

fn copy_file(filename: &Path, destination: &Path, args: &ArgMatches) -> io::Result<()> {
    // destination could be a directory or a filename
    println!("Copy file");
    Ok(())
}

