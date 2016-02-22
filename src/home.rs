use std::fs::{self, File};
use std::io::{self, Read, BufRead, BufReader, Write, BufWriter};

pub fn gen_home() -> Result<(), io::Error> {
    // slurp the entire template file into memory
    // yum
    let mut template_file = try!(File::open("data/home/TEMPLATE.html"));
    let mut template = String::new();
    try!(template_file.read_to_string(&mut template));

    let files = try!(fs::read_dir("data/home"));
    for file in files {
        let path = file.unwrap().path();
        if !path.ends_with("TEMPLATE.html") {
            let br = BufReader::new(try!(File::open(path.clone())));
            let fname = path.file_name().unwrap().to_str().unwrap();
            let mut bw = BufWriter::new(try!(File::create(
                if fname == "index.html" { "../index.html".to_string() }
                else { format!("../{}/index.html", &fname[..fname.len()-5]) }
            )));

            let mut template_lines = template.lines();
            let mut indent_level = 0;
            while let Some(template_line) = template_lines.next() {
                if template_line.ends_with("<!--<>-->") {
                    indent_level = template_line.find(|c| c != ' ').unwrap();
                    break;
                } else {
                    match template_line.find("<!--<A>-->") {
                        Some(idx) => {
                            let target_path = &template_line[
                                template_line[..idx-1].to_string().rfind('/')
                                    .unwrap()+1..idx-2];
                            try!(writeln!(bw, "{}{}>{}", &template_line[..idx-1],
                                    if &fname[..fname.len()-5] == target_path
                                        { " id='active'" } else { "" },
                                    &template_line[idx+10..]));
                        },
                        None => {
                            try!(writeln!(bw, "{}", template_line));
                        }
                    }
                }
            }
            for line in br.lines() {
                // this range-map-collect thing probably isn't too idiomatic
                // but who cares
                try!(writeln!(bw, "{}{}", (0..indent_level).map(|_| ' ')
                        .collect::<String>(), line.unwrap()));
            }
            for template_line in template_lines {
                try!(writeln!(bw, "{}", template_line));
            }
        }
    }

    Ok(())
}
