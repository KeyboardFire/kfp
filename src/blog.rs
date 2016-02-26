use std::collections::HashMap;

use std::fs::{self, File};
use std::io::{self, Read, BufRead, BufReader, Write, BufWriter};

extern crate hoedown;
use self::hoedown::*;
use self::hoedown::renderer::html;

extern crate chrono;
use self::chrono::*;

struct Post {
    title: String,
    date: NaiveDate,
    summary: String,
    slug: String,
    category: String
}

pub fn gen_blog() -> Result<(), io::Error> {
    // we need to keep a thing of all the posts in each category, in
    //   chronological order, so that we can generate a.) /blog.html and b.)
    //   /blog/somecategory/index.html
    // this is that thing
    let mut posts = Vec::<Post>::new();

    // slurp the entire template file into memory
    // yum
    let mut template_file = try!(File::open("data/blog/TEMPLATE.html"));
    let mut template = String::new();
    try!(template_file.read_to_string(&mut template));

    // let's generate /blog/somecategory/somepost.html first
    let files = try!(fs::read_dir("data/blog"));
    for file in files {
        let path = file.unwrap().path();
        if !path.ends_with("TEMPLATE.html") {
            let mut md_file = try!(File::open(path.clone()));
            let mut md = String::new();
            try!(md_file.read_to_string(&mut md));

            let mut metadata = HashMap::new();
            let mut looking_for_summary = false;
            for line in md.lines().skip_while(|x| *x != "<!--METADATA").skip(1) {
                if looking_for_summary {
                    metadata.insert("summary", line);
                    break;
                }
                if line == "-->" { looking_for_summary = true; }
                else {
                    let colon = line.find(':')
                        .expect("metadata line missing colon");
                    metadata.insert(&line[..colon], &line[colon+2..]);
                }
            }
            let (title, date, category, summary) = (
                *metadata.get("title").expect("metadata missing title"),
                *metadata.get("date").expect("metadata missing date"),
                *metadata.get("category").unwrap_or(&"uncategorized"),
                *metadata.get("summary").expect("metadata missing summary"));

            let fname = path.file_name().unwrap().to_str().unwrap();
            let mut bw = BufWriter::new(try!(File::create(format!(
                "../blog/{}/{}.html", category, &fname[..fname.len()-3]))));

            let summary_doc = Markdown::new(summary);
            let mut summary_html = Html::new(html::Flags::empty(), 0);
            posts.push(Post {
                title: title.to_string(),
                date: NaiveDate::parse_from_str(date, "%Y-%m-%d")
                    .expect("metadata date parse error"),
                summary: summary_html.render(&summary_doc).to_str().unwrap()
                    .to_string(),
                slug: (&fname[..fname.len()-3]).to_string(),
                category: category.to_string()
            });

            for template_line in template.lines() {
                if template_line.ends_with("<!--<>-->") {
                    // this range-map-collect thing probably isn't too
                    // idiomatic, but who cares
                    let indent: String = (0..template_line.find(|c| c != ' ')
                        .unwrap()).map(|_| ' ').collect();
                    try!(writeln!(bw, "{}<h2>{}<div class='subheader'>posted \
                                       on {} in category <a href='/blog/{}'>\
                                       {3}</a></div></h2>",
                            indent, title, date, category));
                    let doc = Markdown::new(&md[..]);
                    let mut html = Html::new(html::Flags::empty(), 0);
                    for line in html.render(&doc).to_str().unwrap().lines() {
                        try!(writeln!(bw, "{}{}", indent, line));
                    }
                } else {
                    try!(writeln!(bw, "{}", template_line));
                }
            }
        }
    }

    // sort the list of posts by date real quick
    posts.sort_by(|a, b| b.date.cmp(&a.date));

    // also get a list of categories sorted by frequency
    let mut full_categories: Vec<String> = posts.iter()
        .map(|p| p.category.clone()).collect();
    full_categories.sort();
    let mut categories = full_categories.clone();
    categories.dedup();
    categories.sort_by(|a, b|
        full_categories.iter().filter(|&x| x == b).count().cmp(
            &full_categories.iter().filter(|&x| x == a).count()));

    // now let's generate /blog/somecategory/index.html next
    for category in categories.iter() {
        let mut bw = BufWriter::new(try!(File::create(format!(
            "../blog/{}/index.html", category))));

        for template_line in template.lines() {
            if template_line.ends_with("<!--<>-->") {
                let indent: String = (0..template_line.find(|c| c != ' ')
                    .unwrap()).map(|_| ' ').collect();

                try!(writeln!(bw, "{}<h2>Posts in category [{}]</h2>",
                        indent, category));
                for post in posts.iter().filter(|p| p.category == *category) {
                    try!(writeln!(bw, "{}", post_html(post, &indent)));
                }
            } else {
                try!(writeln!(bw, "{}", template_line));
            }
        }
    }

    // finally, patch /blog.html
    // there's no better way to do this than by copying down to a temp file and
    //   then replacing the original
    try!(fs::copy("../blog/index.html", "../_blog.html"));
    let (br, mut bw) = (
        BufReader::new(try!(File::open("../_blog.html"))),
        BufWriter::new(try!(File::create("../blog/index.html"))));
    for br_line in br.lines() {
        let line = br_line.unwrap();
        if line.ends_with("<!--<C>-->") {
            let indent: String = (0..line.find(|c| c != ' ').unwrap())
                .map(|_| ' ').collect();

            // write categories
            try!(writeln!(bw, "{}<p>Categories:", indent));
            for category in categories.iter() {
                try!(writeln!(bw, "{}    [<a href='/blog/{}'>{1}</a>]",
                    indent, category));
            }
            try!(writeln!(bw, "{}</p>", indent));

            // write posts
            for post in posts.iter() {
                try!(writeln!(bw, "{}", post_html(post, &indent)));
            }
        } else {
            try!(writeln!(bw, "{}", line));
        }
    }
    try!(fs::remove_file("../_blog.html"));

    Ok(())
}

fn post_html(post: &Post, indent: &String) -> String {
    format!(
     "{0}<section class='post'>\
    \n{0}   <h3>\
    \n{0}       <a href='/blog/{1}/{2}.html'>{3}</a>\
    \n{0}       [<a href='/blog/{1}'>{1}</a>]
    \n{0}       <div class='subheader'>{4}</div>\
    \n{0}   </h3>\
    \n{0}   {5}\
    \n{0}</section>",
    indent, post.category, post.slug, post.title, post.date, post.summary)
}
