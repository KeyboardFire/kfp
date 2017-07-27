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
    tags: Vec<String>
}

pub fn gen_blog() -> Result<(), io::Error> {
    // we need to keep a thing of all the posts with each tag, in chronological
    //   order, so that we can generate a.) /blog.html and
    //   b.) /blog/sometag/index.html
    // this is that thing
    let mut posts = Vec::<Post>::new();

    // slurp the entire template file into memory
    // yum
    let mut template_file = File::open("_data/blog/TEMPLATE.html")?;
    let mut template = String::new();
    template_file.read_to_string(&mut template)?;

    // build the RSS feed while we're at it
    let mut rss_file = File::create("blog.xml")?;
    writeln!(rss_file,
             "<?xml version='1.0' encoding='utf-8'?>\
            \n<rss version='2.0'>\
            \n  <channel>\
            \n    <title>KeyboardFire—blog</title>\
            \n    <link>http://keyboardfire.com/blog/</link>\
            \n    <description>ramblings on various topics</description>\
            \n    <language>en</language>")?;

    // let's generate /blog/somepost/index.html first
    let files = fs::read_dir("_data/blog")?;
    for file in files {
        let path = file.unwrap().path();
        if !path.ends_with("TEMPLATE.html") {
            let mut md_file = File::open(path.clone())?;
            let mut md = String::new();
            md_file.read_to_string(&mut md)?;

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
            let (title, date, tags, summary) = (
                *metadata.get("title").expect("metadata missing title"),
                *metadata.get("date").expect("metadata missing date"),
                (*metadata.get("tags").unwrap_or(&"untagged")).split(' ')
                    .map(String::from).collect::<Vec<String>>(),
                *metadata.get("summary").expect("metadata missing summary"));

            let fname = path.file_name().unwrap().to_str().unwrap();
            let postname = &fname[..fname.len()-3];
            let _ = fs::create_dir(format!("blog/{}", postname));
            let mut bw = BufWriter::new(File::create(format!(
                        "blog/{}/index.html", postname))?);

            let summary_doc = Markdown::new(summary);
            let mut summary_html = Html::new(html::Flags::empty(), 0);
            posts.push(Post {
                title: title.to_string(),
                date: NaiveDate::parse_from_str(date, "%Y-%m-%d")
                    .expect("metadata date parse error"),
                summary: summary_html.render(&summary_doc).to_str().unwrap()
                    .to_string(),
                slug: (&fname[..fname.len()-3]).to_string(),
                tags: tags.clone()
            });

            for template_line in template.lines() {
                if template_line.ends_with("<!--<>-->") {
                    // this range-map-collect thing probably isn't too
                    // idiomatic, but who cares
                    let indent: String = (0..template_line.find(|c| c != ' ')
                        .unwrap()).map(|_| ' ').collect();
                    writeln!(bw, "{}<h2>{}<div class='subheader'>posted \
                                       on {} with tags {}</div></h2>",
                                       indent, title, date, tags_html(&tags))?;
                    let doc = Markdown::new(&md[..]);
                    let mut html = Html::new(html::Flags::empty(), 0);
                    for line in html.render(&doc).to_str().unwrap().lines() {
                        writeln!(bw, "{}{}", indent, line)?;
                    }
                } else {
                    writeln!(bw, "{}", template_line)?;
                }
            }
        }
    }

    // sort the list of posts by date real quick
    posts.sort_by(|a, b| b.date.cmp(&a.date));

    // also get a list of tags sorted by frequency
    let mut full_tags: Vec<String> = posts.iter()
        .flat_map(|p| p.tags.clone()).collect();
    full_tags.sort();
    let mut tags = full_tags.clone();
    tags.dedup();
    tags.sort_by(|a, b|
        full_tags.iter().filter(|&x| x == b).count().cmp(
            &full_tags.iter().filter(|&x| x == a).count()));

    // now let's generate /blog/sometag/index.html next
    for tag in &tags {
        let _ = fs::create_dir(format!("blog/{}", tag));
        let mut bw = BufWriter::new(File::create(format!(
                    "blog/{}/index.html", tag))?);

        for template_line in template.lines() {
            if template_line.ends_with("<!--<>-->") {
                let indent: String = (0..template_line.find(|c| c != ' ')
                    .unwrap()).map(|_| ' ').collect();

                writeln!(bw, "{}<h2>Posts tagged [{}]</h2>", indent, tag)?;
                for post in posts.iter().filter(|p| p.tags.contains(tag)) {
                    writeln!(bw, "{}", post_html(post, &indent))?;
                }
            } else {
                writeln!(bw, "{}", template_line)?;
            }
        }
    }

    // finally, patch /blog.html
    // there's no better way to do this than by copying down to a temp file and
    //   then replacing the original
    fs::copy("blog/index.html", "_blog.html")?;
    let (br, mut bw) = (
        BufReader::new(File::open("_blog.html")?),
        BufWriter::new(File::create("blog/index.html")?));
    for br_line in br.lines() {
        let line = br_line.unwrap();
        if line.ends_with("<!--<C>-->") {
            let indent: String = (0..line.find(|c| c != ' ').unwrap())
                .map(|_| ' ').collect();

            // write tags
            writeln!(bw, "{}<p>Tags:", indent)?;
            for tag in &tags {
                writeln!(bw, "{}    [<a href='/blog/{}'>{1}</a>]",
                         indent, tag)?;
            }
            writeln!(bw, "{}</p>", indent)?;

            // write posts
            for post in &posts {
                writeln!(bw, "{}", post_html(post, &indent))?;
                writeln!(rss_file, "{}", post_rss(post))?;
            }
        } else {
            writeln!(bw, "{}", line)?;
        }
    }
    fs::remove_file("_blog.html")?;

    // close RSS tags
    writeln!(rss_file,
             "  </channel>\
            \n</rss>")?;

    Ok(())
}

fn post_html(post: &Post, indent: &String) -> String {
    format!(
     "{0}<section class='post'>\
    \n{0}   <h3>\
    \n{0}       <a href='/blog/{2}'>{3}</a>\
    \n{0}       {1}\
    \n{0}       <div class='subheader'>{4}</div>\
    \n{0}   </h3>\
    \n{0}   {5}\
    \n{0}</section>",
    indent, tags_html(&post.tags), post.slug, post.title, post.date, post.summary)
}

fn post_rss(post: &Post) -> String {
    format!(
     "    <item>\
    \n      <title>{}</title>\
    \n      <link>http://keyboardfire.com/blog/{}</link>\
    \n      <description>{}</description>\
    \n      <pubDate>{}</pubDate>\
    \n      <guid>http://keyboardfire.com/blog/{1}</guid>\
    \n    </item>", post.title, post.slug, unhtml(&post.summary), post.date.format("%a, %d %b %Y 17:00:00 GMT"))
}

fn tags_html(tags: &Vec<String>) -> String {
    tags.iter().map(|tag| format!("[<a href='/blog/{0}'>{0}</a>]", tag))
        .collect::<Vec<String>>().join(" ")
}

fn unhtml(s: &String) -> String {
    let mut s = s.clone();
    while let Some(openpos) = s.find('<') {
        let closepos = s.find('>').unwrap();
        s.drain(openpos..closepos+1);
    }
    s.trim_right().to_string()
}
