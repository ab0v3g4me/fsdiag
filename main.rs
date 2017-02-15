extern crate clap;
extern crate crypto;
extern crate time;

use crypto::digest::Digest;
use std::fs;
use std::path;
use std::io::{Read,Write};
use std::process;
use std::os::linux::fs::MetadataExt;
use std::str;
use std::io::BufReader;
use std::io::BufRead;

const OUTPUT_FILE: &'static str = "fsdiag.log"; // Default name of the output file
const GREEN: &'static str = "\x1b[32m";
const RED: &'static str = "\x1b[31m";
const YELLOW: &'static str = "\x1b[33m";
const DEFAULT: &'static str = "\x1b[39m";


fn main() {
    let fsdiag_app = clap::App::new("File system diag")
                            .version("1.0.0")
                            .author("Abovegame")
                            .arg(clap::Arg::with_name("path")
                                    .required(true)
                                    .help("Specify the path you want to search in.")
                                )
                            .arg(clap::Arg::with_name("output")
                                    .short("o")
                                    .long("output")
                                    .help("Specify the name of the output file. default: [fsdiag.log]")
                                    .value_name("FILE")
                                )
                            .arg(clap::Arg::with_name("extension")
                                    .short("ex")
                                    .long("extension")
                                    .help("Search only files with the specified extension.")
                                    .value_name("EXTENSION")
                                )
                            .arg(clap::Arg::with_name("compare")
                                    .short("c")
                                    .long("compare")
                                    .help("Compares files from output file to see if any changes were made.")
                                    .value_name("FILE")
                                    .conflicts_with_all(&["new","output","extension"])
                                )
                            .arg(clap::Arg::with_name("new")
                                    .short("n")
                                    .long("new")
                                    .help("Prints newly created files within N days.")
                                    .value_name("N")
                                    .conflicts_with_all(&["compare","extension","output"])
                                )
                            .get_matches();
                            /* Declaring variables */
    let mut files: Vec<std::path::PathBuf> = Vec::new(); // All corresponding files
    let mut file_hashes: Vec<String> = Vec::new(); // All corresponding hashes
    let of = fsdiag_app.value_of("output");
    let cf = fsdiag_app.value_of("compare"); // Compare file name
    let days = fsdiag_app.value_of("new"); // Number of days
    let ext = fsdiag_app.value_of("extension"); // Specified extension
    let path = path::Path::new( fsdiag_app.value_of("path").unwrap() ); // Specified path
                            /* Opt logic */
    if fsdiag_app.is_present("new") { search(path, &mut files,ext,days); }
    else if fsdiag_app.is_present("compare") { compare(cf.unwrap(),path); }
    else if fsdiag_app.is_present("output") && fsdiag_app.is_present("extension")
    || fsdiag_app.is_present("output") || fsdiag_app.is_present("extension")
    || fsdiag_app.is_present("path")
    { search(path, &mut files,ext,days);
      for mut f in files.iter_mut() { file_hashes.push(f2md5(&mut f)); }
      output_write(&mut files,&mut file_hashes,of); }
}

fn search(p: &std::path::Path,files: &mut Vec<std::path::PathBuf>,extension: Option<&str>,new_opt: Option<&str>){
    for entry in fs::read_dir(&p).unwrap(){
        let ent = match entry {
            Ok(e) => e,
            Err(e) => {let _ = e; continue },
        };
        if ent.file_type().unwrap().is_symlink() { continue }
        else if ent.path().is_dir() { search(&ent.path(),files,extension,new_opt); }
        else if let Some(new_date) = new_opt{
            let nd = match new_date.parse::<i64>(){
                Ok(n) => n,
                Err(_) => {println!("{}[-]{} Bad input argument for `--new` option.",RED, DEFAULT);process::exit(1);}
            };
            let dayspassed = ( time::now().to_timespec().sec - ent.path().metadata().unwrap().st_ctime() ) / 86400;
            if nd >= dayspassed { println!("{}[+]{} Found file - {}", YELLOW, DEFAULT, ent.path().to_str().unwrap() );}
            }
        else{
            match extension{
                Some(ext) => {if ent.path().to_str().unwrap().ends_with(ext){  files.push(ent.path() );   } },
                None => files.push( ent.path() ),
            }
        }
    };
 }
                /* Fn for hashing files */
fn f2md5(p: &mut std::path::PathBuf) -> String {
    let mut  f = fs::File::open(p).unwrap();
    let mut current_hash = crypto::md5::Md5::new();
    loop {
        let mut buffer:[u8;1024] = [0;1024];
        match f.read(&mut buffer){
            Ok(0) => break,
            Ok(e) => current_hash.input(&[e as u8]),
            Err(_) => break,
        }
    }
    current_hash.result_str()
}
            /* Fn for creating an output file */
fn output_write(files: &mut Vec<std::path::PathBuf>,hashes: &mut Vec<String>,fname: Option<&str>){
    if files.is_empty() { println!("{}[-]{} No files found matching specified criteria.",RED,DEFAULT); process::exit(1); }
    let file_name = match fname{
        Some(f) => f,
        None => OUTPUT_FILE,
    };
    let mut of = match fs::File::create(file_name){
        Ok(e) => e,
        Err(e) => panic!("{:?}",e),
    };
    for idx in 0..files.len(){
        let full_str = format!("{}{}{}{}",files[idx].to_str().unwrap(),"[=]",hashes[idx],"\n");
        let _ = of.write(full_str.as_bytes());
    }
    println!("{}[+]{} {} - succesfully created.Contains: {} items.",GREEN,DEFAULT ,file_name, files.len() );
}
        /* Fn for comparing previously generated output file */
fn compare(f: &str,p: &std::path::Path) {
    let mut files: Vec<[String;2]> = Vec::new();
    let mut f_changed: usize = 0;
    let cf = match fs::File::open(f){
        Ok(f) => f,
        Err(f) => {println!("{}[-]{} Failed to open - {}",RED,DEFAULT,f);process::exit(1);}
    };
    let buff = BufReader::new(cf).lines();
    for line in buff{
        let l = match line{
            Ok(l) => l,
            Err(l) => panic!("{:?}",l),};
        let mut parts = l.splitn(2,"[=]");
        let (fpath, fhash) = (parts.next().unwrap().to_owned(), parts.next().unwrap().to_owned());
        if fpath.contains( p.to_str().unwrap() ){files.push([fpath,fhash]);}
    }
    if files.is_empty() { println!("{}[-]{} No files found with the specified path.",RED,DEFAULT );process::exit(1); }
    for f in files {
        let ref nf = f[0];
        let mut pb = path::Path::new(nf.as_str()).to_path_buf();
        match fs::File::open(nf){
            Ok(_) => {
                if f2md5(&mut pb) != f[1] {  println!("{}[-]{} File {} was modified.",RED,DEFAULT,f[0] );
                f_changed += 1; }
            },
            Err(_) => { println!("{}[-]{} File {} - removed or renamed.",RED,DEFAULT,f[0]); f_changed += 1; },
        }
    }
    if f_changed == 0 { println!("{}[+]{} No files were modified.",GREEN, DEFAULT);}
    else { println!("{}[+]{} All other files were not changed.",GREEN, DEFAULT); }
}
