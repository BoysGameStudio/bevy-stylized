//! Read-only command-line validator for CEL glTF assets.

use std::{
    env,
    ffi::OsString,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

use bevy::reflect::TypeRegistry;
use bevy_gltf_metadata::{
    MetadataDiagnostic, ValidationCode, ValidationReport, ValidationSeverity, validate_assets,
};

const HELP: &str = "cel_gltf_validate - validate CEL glTF assets without modifying them\n\nUsage: cel_gltf_validate [--format human|json] <ASSET>...\n\nOptions:\n  --format <FORMAT>  human (default) or json\n  -h, --help         Show this help\n";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum OutputFormat {
    #[default]
    Human,
    Json,
}

#[derive(Debug)]
struct Cli {
    format: OutputFormat,
    paths: Vec<PathBuf>,
}

#[derive(Debug)]
enum Action {
    Help,
    Validate(Cli),
}

fn main() -> ExitCode {
    match run(env::args_os().skip(1).collect()) {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "cel_gltf_validate: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: Vec<OsString>) -> io::Result<ExitCode> {
    let format_hint = requested_format(&arguments);
    let action = match parse(arguments) {
        Ok(action) => action,
        Err(code) => {
            write_usage_error(code, format_hint)?;
            return Ok(ExitCode::from(2));
        }
    };
    match action {
        Action::Help => {
            io::stdout().lock().write_all(HELP.as_bytes())?;
            Ok(ExitCode::SUCCESS)
        }
        Action::Validate(cli) => {
            let mut registry = TypeRegistry::default();
            registry.register::<MetadataDiagnostic>();
            let report = validate_assets(&cli.paths, &registry);
            write_report(&report, cli.format)?;
            Ok(if report.valid {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            })
        }
    }
}

fn requested_format(arguments: &[OsString]) -> OutputFormat {
    arguments
        .windows(2)
        .find(|pair| pair.first().and_then(|value| value.to_str()) == Some("--format"))
        .and_then(|pair| pair.get(1))
        .and_then(|value| value.to_str())
        .and_then(parse_format)
        .unwrap_or_default()
}

fn parse(arguments: Vec<OsString>) -> Result<Action, ValidationCode> {
    if arguments
        .iter()
        .any(|argument| matches!(argument.to_str(), Some("-h" | "--help")))
    {
        return Ok(Action::Help);
    }
    let mut format = OutputFormat::Human;
    let mut paths = Vec::new();
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        match argument.to_str() {
            Some("--format") => {
                let value = arguments
                    .next()
                    .ok_or(ValidationCode::CliFormatUnsupported)?;
                format = value
                    .to_str()
                    .and_then(parse_format)
                    .ok_or(ValidationCode::CliFormatUnsupported)?;
            }
            Some(value) if value.starts_with('-') => {
                return Err(ValidationCode::CliArgumentUnknown);
            }
            Some(_) | None => paths.push(PathBuf::from(argument)),
        }
    }
    if paths.is_empty() {
        Err(ValidationCode::CliInputMissing)
    } else {
        Ok(Action::Validate(Cli { format, paths }))
    }
}

const fn parse_format(value: &str) -> Option<OutputFormat> {
    match value.as_bytes() {
        b"human" => Some(OutputFormat::Human),
        b"json" => Some(OutputFormat::Json),
        _ => None,
    }
}

fn write_usage_error(code: ValidationCode, format: OutputFormat) -> io::Result<()> {
    match format {
        OutputFormat::Human => {
            let code = serde_json::to_string(&code).map_err(io::Error::other)?;
            writeln!(
                io::stderr().lock(),
                "ERROR {} <cli> - invalid command line; run --help",
                code.trim_matches('"')
            )
        }
        OutputFormat::Json => write_json(&ValidationReport::cli_error(code)),
    }
}

fn write_report(report: &ValidationReport, format: OutputFormat) -> io::Result<()> {
    match format {
        OutputFormat::Json => write_json(report),
        OutputFormat::Human => write_human(report),
    }
}

fn write_json(report: &ValidationReport) -> io::Result<()> {
    let mut output = io::BufWriter::new(io::stdout().lock());
    serde_json::to_writer(&mut output, report).map_err(io::Error::other)?;
    output.write_all(b"\n")
}

fn write_human(report: &ValidationReport) -> io::Result<()> {
    let mut output = io::BufWriter::new(io::stdout().lock());
    for diagnostic in &report.diagnostics {
        let code = serde_json::to_string(&diagnostic.code).map_err(io::Error::other)?;
        let severity = match diagnostic.severity {
            ValidationSeverity::Error => "ERROR",
            ValidationSeverity::Warning => "WARNING",
        };
        writeln!(
            output,
            "{severity} {} {}:{} - {}; fix: {}",
            code.trim_matches('"'),
            diagnostic.file_path,
            diagnostic.asset_path,
            diagnostic.message,
            diagnostic.remediation
        )?;
    }
    if report.valid {
        for file in &report.files {
            writeln!(output, "VALID {file}")?;
        }
    } else {
        let errors = report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == ValidationSeverity::Error)
            .count();
        writeln!(output, "INVALID {errors} error(s)")?;
    }
    Ok(())
}
