use clap::Parser;
use roxmltree::Document;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "latex2web")]
#[command(about = "Convert LaTeX documents to HTML", long_about = None)]
struct Cli {
    input: PathBuf,

    #[arg(short, long)]
    output: Option<PathBuf>,

    #[arg(short, long, default_value = "clean-serif")]
    theme: String,
}

fn main() {
    let cli = Cli::parse();

    // check if latexml is installed
    if Command::new("latexml").arg("--version").output().is_err() {
        eprintln!("error: latexml not found");
        eprintln!("install with: brew install latexml (mac) or apt install latexml (linux)");
        std::process::exit(1);
    }

    println!("converting {} with latexml...", cli.input.display());

    // run latexml to get XML
    let xml_output = Command::new("latexml")
        .arg(&cli.input)
        .output()
        .expect("failed to run latexml");

    if !xml_output.status.success() {
        eprintln!("latexml failed: {}", String::from_utf8_lossy(&xml_output.stderr));
        std::process::exit(1);
    }

    let xml_str = String::from_utf8_lossy(&xml_output.stdout);
    
    // parse XML and convert to HTML
    let html = xml_to_html(&xml_str, &cli.theme);

    let output_path = cli.output.unwrap_or_else(|| {
        cli.input.with_extension("html")
    });

    fs::write(&output_path, html)
        .expect("couldn't write output file");

    println!("wrote {}", output_path.display());
}

fn xml_to_html(xml_str: &str, theme: &str) -> String {
    let doc = Document::parse(xml_str).expect("failed to parse XML");
    
    let root = doc.root_element();
    
    // extract metadata
    let title = extract_text_by_tag(&root, "title").unwrap_or("Untitled".to_string());
    let author = extract_text_by_tag(&root, "creator");
    
    // find the document body
    let body_html = if let Some(body) = find_element(root, "document") {
        process_node(&body)
    } else {
        process_node(&root)
    };

    let author_html = if let Some(a) = author {
        if !a.is_empty() {
            format!("<p class=\"author\">{}</p>", a)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <style>
{}
    </style>
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/themes/prism-tomorrow.min.css">
    <script>
        MathJax = {{
            tex: {{
                inlineMath: [['\\(', '\\)']],
                displayMath: [['\\[', '\\]']]
            }}
        }};
    </script>
    <script src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js"></script>
</head>
<body>
    <article>
        <header>
            <h1 class="title">{}</h1>
            {}
        </header>
        <main>
{}
        </main>
    </article>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/prism.min.js"></script>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-python.min.js"></script>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-java.min.js"></script>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-c.min.js"></script>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-cpp.min.js"></script>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-javascript.min.js"></script>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-bash.min.js"></script>
</body>
</html>"#,
        title,
        get_theme_css(theme),
        title,
        author_html,
        body_html
    )
}

fn find_element<'a, 'input: 'a>(node: roxmltree::Node<'a, 'input>, tag: &str) -> Option<roxmltree::Node<'a, 'input>> {
    if node.has_tag_name(tag) {
        return Some(node);
    }
    
    for child in node.children() {
        if let Some(found) = find_element(child, tag) {
            return Some(found);
        }
    }
    None
}

fn extract_text_by_tag(node: &roxmltree::Node, tag: &str) -> Option<String> {
    if let Some(element) = find_element(*node, tag) {
        Some(element.text().unwrap_or("").to_string())
    } else {
        None
    }
}

fn process_node(node: &roxmltree::Node) -> String {
    let mut html = String::new();
    
    if node.is_text() {
        if let Some(text) = node.text() {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                html.push_str(trimmed);
                html.push(' '); // add space after text
            }
        }
        return html;
    }

    if !node.is_element() {
        return html;
    }

    let tag = node.tag_name().name();
    
    // skip these LaTeXML metadata elements
    match tag {
        "tags" | "tag" | "ref" | "bibref" => return html,
        _ => {}
    }
    
    match tag {
        "section" => {
            html.push_str("<section>");
            for child in node.children() {
                html.push_str(&process_node(&child));
            }
            html.push_str("</section>");
        }
        "title" => {
            // skip if it's the document title (already in header)
            if let Some(parent) = node.parent() {
                if parent.has_tag_name("document") {
                    return html;
                }
            }
            
            let level = get_section_depth(node);
            let h_tag = format!("h{}", (level + 1).min(6));
            html.push_str(&format!("<{}>", h_tag));
            for child in node.children() {
                html.push_str(&process_node(&child));
            }
            html.push_str(&format!("</{}>", h_tag));
        }
        "para" | "p" => {
            html.push_str("<p>");
            for child in node.children() {
                html.push_str(&process_node(&child));
            }
            html.push_str("</p>");
        }
        "emph" | "em" => {
            html.push_str("<em>");
            for child in node.children() {
                html.push_str(&process_node(&child));
            }
            html.push_str("</em>");
        }
        "text" if node.attribute("font") == Some("bold") => {
            html.push_str("<strong>");
            for child in node.children() {
                html.push_str(&process_node(&child));
            }
            html.push_str("</strong>");
        }
        "text" if node.attribute("font") == Some("typewriter") => {
            html.push_str("<code>");
            for child in node.children() {
                html.push_str(&process_node(&child));
            }
            html.push_str("</code>");
        }
        "text" => {
            for child in node.children() {
                html.push_str(&process_node(&child));
            }
        }
        "itemize" => {
            html.push_str("<ul>");
            for child in node.children() {
                if child.has_tag_name("item") {
                    html.push_str("<li>");
                    for item_child in child.children() {
                        html.push_str(&process_node(&item_child));
                    }
                    html.push_str("</li>");
                }
            }
            html.push_str("</ul>");
        }
        "enumerate" => {
            html.push_str("<ol>");
            for child in node.children() {
                if child.has_tag_name("item") {
                    html.push_str("<li>");
                    for item_child in child.children() {
                        html.push_str(&process_node(&item_child));
                    }
                    html.push_str("</li>");
                }
            }
            html.push_str("</ol>");
        }
        "tabular" | "table" => {
            html.push_str("<div class=\"table-wrapper\"><table>");
            
            // process table rows
            for child in node.children() {
                if child.has_tag_name("tr") {
                    html.push_str("<tr>");
                    for cell in child.children() {
                        if cell.has_tag_name("td") {
                            html.push_str("<td>");
                            for cell_child in cell.children() {
                                html.push_str(&process_node(&cell_child));
                            }
                            html.push_str("</td>");
                        } else if cell.has_tag_name("th") {
                            html.push_str("<th>");
                            for cell_child in cell.children() {
                                html.push_str(&process_node(&cell_child));
                            }
                            html.push_str("</th>");
                        }
                    }
                    html.push_str("</tr>");
                }
            }
            
            html.push_str("</table></div>");
        }
        "graphics" | "figure" => {
            // handle images
            if let Some(src) = node.attribute("graphic") {
                let caption = node.children()
                    .find(|c| c.has_tag_name("caption"))
                    .map(|c| get_all_text(&c))
                    .unwrap_or_default();
                
                html.push_str("<figure>");
                html.push_str(&format!("<img src=\"{}\" alt=\"{}\">", src, caption));
                if !caption.is_empty() {
                    html.push_str(&format!("<figcaption>{}</figcaption>", caption));
                }
                html.push_str("</figure>");
            } else {
                // process children if no graphic attribute
                for child in node.children() {
                    html.push_str(&process_node(&child));
                }
            }
        }
        "verbatim" | "lstlisting" => {
            // code blocks
            let code_content = get_all_text(node);
            let language = node.attribute("language").unwrap_or("");
            
            html.push_str(&format!("<pre><code class=\"language-{}\">{}</code></pre>", 
                language, 
                html_escape(&code_content)
            ));
        }
        "Math" | "math" => {
            let math_content = get_all_text(node);
            if node.attribute("mode") == Some("display") {
                html.push_str("<div class=\"math-display\">");
                html.push_str(&format!("\\[{}\\]", math_content));
                html.push_str("</div>");
            } else {
                html.push_str(&format!("\\({}\\)", math_content));
            }
        }
        "creator" => {
            // skip, we handle this separately
        }
        _ => {
            // recursively process children for unknown tags
            for child in node.children() {
                html.push_str(&process_node(&child));
            }
        }
    }

    html
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn get_section_depth(node: &roxmltree::Node) -> usize {
    let mut depth = 0;
    let mut current = node.parent();
    
    while let Some(parent) = current {
        if parent.has_tag_name("section") {
            depth += 1;
        }
        current = parent.parent();
    }
    
    depth
}

fn get_all_text(node: &roxmltree::Node) -> String {
    let mut text = String::new();
    
    if node.is_text() {
        if let Some(t) = node.text() {
            text.push_str(t);
        }
    }
    
    for child in node.children() {
        text.push_str(&get_all_text(&child));
    }
    
    text
}

fn get_theme_css(theme: &str) -> &'static str {
    match theme {
        "dark" => include_str!("themes/dark.css"),
        _ => include_str!("themes/clean-serif.css"),
    }
}
