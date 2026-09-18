//! showcase CLI：`cargo run -p showcase -- <name|all> [--capture]`。

use showcase::{ShowcaseCase, all_cases, capture_case};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "inspect") {
        #[cfg(feature = "inspector")]
        if let Err(error) = showcase::inspector::run(&args[1..]) {
            eprintln!("inspector: {error}");
            std::process::exit(1);
        }
        #[cfg(not(feature = "inspector"))]
        {
            eprintln!("inspect requires --features inspector");
            std::process::exit(2);
        }
        #[cfg(feature = "inspector")]
        return;
    }
    let capture_mode = args.iter().any(|a| a == "--capture");
    let name = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(String::as_str)
        .unwrap_or("all");

    let cases = all_cases();
    let selected: Vec<&ShowcaseCase> = if name == "all" {
        cases.iter().collect()
    } else {
        match cases.iter().find(|c| c.name == name) {
            Some(case) => vec![case],
            None => {
                eprintln!("unknown showcase case: {name}");
                eprintln!(
                    "available: {}",
                    cases.iter().map(|c| c.name).collect::<Vec<_>>().join(", ")
                );
                std::process::exit(2);
            }
        }
    };

    if !capture_mode {
        for case in selected {
            showcase::run_interactive(case);
        }
        return;
    }

    // --capture：headless 离屏，PNG 写入 target/stylized/showcase/。
    let Some(resources) = golden_harness::probe_render_resources() else {
        eprintln!("showcase: no GPU adapter available; nothing captured");
        std::process::exit(0);
    };
    let out_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/stylized/showcase");
    std::fs::create_dir_all(&out_dir).expect("create showcase output dir");
    for case in selected {
        let image = capture_case(case, resources.clone());
        let png = golden_harness::encode_png(&image).expect("encode png");
        let path = out_dir.join(format!("{}.png", case.name));
        std::fs::write(&path, png).expect("write png");
        println!("[captured] {}", path.display());
    }
}
