mod image_pairs;
// Diagnostic capture and an optional strict parity gate.
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--image-pairs") {
        let result = (|| {
            let path = args.get(i + 1).ok_or("--image-pairs requires a manifest")?;
            let out = args
                .iter()
                .position(|a| a == "--out")
                .and_then(|i| args.get(i + 1))
                .ok_or("--out required")?;
            image_pairs::run(std::path::Path::new(path), std::path::Path::new(out))
        })();
        match result {
            Ok(passed) => {
                if !passed && args.iter().any(|a| a == "--required") {
                    std::process::exit(1);
                }
            }
            Err(error) => {
                eprintln!("BLOCKED: {error}");
                std::process::exit(2);
            }
        }
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--character-case") {
        let Some(path) = args.get(i + 1) else {
            eprintln!("case path required");
            std::process::exit(2)
        };
        let Some(out) = args
            .iter()
            .position(|a| a == "--out")
            .and_then(|i| args.get(i + 1))
        else {
            eprintln!("--out required");
            std::process::exit(2)
        };
        match reference_comparison::character::run(path.into(), out.into()) {
            Ok(report) => {
                println!("{report}");
                if args.iter().any(|a| a == "--required") && report["parity_status"] != "PASS" {
                    std::process::exit(1)
                }
            }
            Err(e) => {
                eprintln!("BLOCKED: {e}");
                std::process::exit(2)
            }
        }
        return;
    }
    eprintln!("use --character-case CASE --out DIR or --image-pairs MANIFEST --out DIR");
    std::process::exit(2);
}
