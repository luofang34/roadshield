//! Argument definitions (clap builder API).

use std::path::PathBuf;

use clap::{Arg, ArgAction, Command, value_parser};

fn pack_arg() -> Arg {
    Arg::new("pack")
        .long("pack")
        .value_name("DIR")
        .default_value("packs/americana")
        .value_parser(value_parser!(PathBuf))
        .help("Resource pack directory")
}

fn path_arg(name: &'static str, help: &'static str) -> Arg {
    Arg::new(name)
        .long(name)
        .value_name("PATH")
        .value_parser(value_parser!(PathBuf))
        .help(help)
}

fn context_args(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("scale")
            .long("scale")
            .value_parser(value_parser!(f64))
            .default_value("1")
            .help("Logical scale relative to Americana 1x pixels"),
    )
    .arg(
        Arg::new("lang")
            .long("lang")
            .help("BCP 47 language tag for shaping"),
    )
    .arg(path_arg(
        "context",
        "JSON file with a full DisplayContext (overrides --scale/--lang)",
    ))
    .arg(
        Arg::new("strict-network")
            .long("strict-network")
            .action(ArgAction::SetTrue)
            .help("Fail on unknown networks instead of using the generic fallback"),
    )
}

fn raster_args(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("format")
            .long("format")
            .value_parser(["svg", "png", "both"])
            .default_value("svg")
            .help("Output format"),
    )
    .arg(
        Arg::new("dpr")
            .long("dpr")
            .value_parser(value_parser!(f32))
            .default_value("2")
            .help("Device pixel ratio for PNG output"),
    )
}

fn render_cmd() -> Command {
    raster_args(context_args(
        Command::new("render")
            .about("Render one shield")
            .arg(pack_arg())
            .arg(Arg::new("network").long("network").required(true))
            .arg(Arg::new("ref").long("ref"))
            .arg(Arg::new("name").long("name"))
            .arg(path_arg("out", "Output file (stdout for SVG when omitted)"))
            .arg(
                Arg::new("json")
                    .long("json")
                    .action(ArgAction::SetTrue)
                    .help("Print metadata JSON"),
            ),
    ))
}

fn bound(
    name: &'static str,
    parser: clap::builder::ValueParser,
    default: Option<&'static str>,
    help: &'static str,
) -> Arg {
    let arg = Arg::new(name).long(name).value_parser(parser).help(help);
    match default {
        Some(d) => arg.default_value(d),
        None => arg,
    }
}

fn batch_cmd() -> Command {
    raster_args(context_args(
        Command::new("batch")
            .about("Render every route descriptor in a JSONL file")
            .arg(pack_arg())
            .arg(path_arg("input", "JSONL file of RouteDescriptor objects").required(true))
            .arg(path_arg("out-dir", "Output directory").required(true))
            .arg(bound(
                "max-items",
                value_parser!(usize).into(),
                Some("1000000"),
                "Stop after this many items",
            ))
            .arg(bound(
                "max-input-bytes",
                value_parser!(u64).into(),
                Some("268435456"),
                "Refuse larger input files",
            ))
            .arg(bound(
                "max-output-bytes",
                value_parser!(u64).into(),
                Some("4294967296"),
                "Stop after writing this much",
            ))
            .arg(bound(
                "time-limit",
                value_parser!(u64).into(),
                None,
                "Stop after this many seconds",
            ))
            .arg(bound(
                "jobs",
                value_parser!(usize).into(),
                None,
                "Worker threads",
            )),
    ))
}

fn import_cmd() -> Command {
    Command::new("import")
        .about("Build a pack from pinned upstream inputs")
        .arg(path_arg("config", "Import config JSON").default_value("packs/americana.import.json"))
        .arg(
            path_arg("checkout", "Upstream checkout")
                .default_value(".upstream/openstreetmap-americana"),
        )
        .arg(path_arg("inputs", "Pinned input files").default_value("packs/americana.inputs"))
        .arg(path_arg("out", "Output pack directory").default_value("packs/americana"))
}

fn subset_cmd() -> Command {
    Command::new("subset")
        .about("Cut a verifiable network subset from a full pack")
        .arg(pack_arg())
        .arg(
            Arg::new("networks")
                .long("networks")
                .required(true)
                .value_delimiter(',')
                .help("Network keys or prefixes, comma separated"),
        )
        .arg(path_arg("out", "Output pack directory").required(true))
}

fn diff_cmd() -> Command {
    Command::new("diff")
        .about("Compare two packs: rules, assets, engine sources, visuals")
        .arg(
            Arg::new("old")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("new")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(path_arg("html", "Write a visual comparison page"))
        .arg(path_arg("json", "Write the diff as JSON"))
}

fn inspect_cmd() -> Command {
    Command::new("inspect")
        .about("Summarise a pack and list rule issues")
        .arg(pack_arg())
        .arg(
            Arg::new("networks")
                .long("networks")
                .action(ArgAction::SetTrue)
                .help("List network keys"),
        )
}

fn bench_cmd() -> Command {
    Command::new("bench")
        .about("Measure cold/hot render latency and batch throughput")
        .arg(pack_arg())
        .arg(bound(
            "iterations",
            value_parser!(usize).into(),
            Some("2000"),
            "Renders to time",
        ))
}

/// The full command tree.
pub fn command() -> Command {
    Command::new("roadshield")
        .about("Road shield generator compatible with OSM Americana ShieldJSON")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(true)
        .subcommands([
            render_cmd(),
            batch_cmd(),
            import_cmd(),
            subset_cmd(),
            diff_cmd(),
            inspect_cmd(),
            bench_cmd(),
        ])
}

#[cfg(test)]
mod tests;
