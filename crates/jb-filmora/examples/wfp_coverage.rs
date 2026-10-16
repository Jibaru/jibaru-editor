//! Prints the union coverage of one or more `.wfp` files.
//!
//! ```sh
//! cargo run -p jb-filmora --example wfp_coverage -- [--json] [--warnings] a.wfp b.wfp ...
//! ```
//!
//! Names are resolved against the local Filmora resource folders when they
//! exist (only their `resinfo.json` metadata is read).

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let as_json = args.iter().any(|a| a == "--json");
    let warnings = args.iter().any(|a| a == "--warnings");
    let index = jb_filmora::ResourceIndex::scan_default();
    let mut total = jb_filmora::Coverage::default();
    for path in args.iter().filter(|a| !a.starts_with("--")) {
        let wfp = match jb_filmora::read(path) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("{path}: {e}");
                std::process::exit(1);
            }
        };
        if warnings {
            for w in &wfp.warnings {
                eprintln!("{path}: {w}");
            }
        }
        total.merge(&jb_filmora::coverage(&wfp));
    }
    total.resolve(&index);
    if as_json {
        println!("{}", serde_json::to_string_pretty(&total).unwrap());
    } else {
        print!("{}", total.to_markdown());
    }
}
