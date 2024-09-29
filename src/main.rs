#![recursion_limit = "128"]

use anyhow::{bail, Context, Result};
use chiptool::{generate, svd2ir};
use clap::Parser;
use log::*;
use proc_macro2::TokenStream;
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::{fs::File, io::stdout};
use svd_parser::ValidateLevel;

use chiptool::ir::IR;

#[derive(Parser)]
#[clap(version = "1.0", author = "Dirbaio <dirbaio@dirbaio.net>")]
struct Opts {
    #[clap(subcommand)]
    subcommand: Subcommand,
}

#[derive(Parser)]
enum Subcommand {
    Generate(Generate),
    ExtractAll(ExtractAll),
    ExtractPeripheral(ExtractPeripheral),
    Transform(Transform),
    Fmt(Fmt),
    Check(Check),
    GenBlock(GenBlock),
}

/// Extract peripheral from SVD to YAML
#[derive(Parser)]
struct ExtractPeripheral {
    /// SVD file path
    #[clap(long)]
    svd: String,
    /// Peripheral from the SVD
    #[clap(long)]
    peripheral: String,
    /// Transforms file path
    #[clap(long)]
    transform: Vec<String>,
}

/// Extract all peripherals from SVD to YAML
#[derive(Parser)]
struct ExtractAll {
    /// SVD file path
    #[clap(long)]
    svd: String,
    /// Output directory. Each peripheral will be created as a YAML file here.
    #[clap(short, long)]
    output: String,
}

/// Apply transform to YAML
#[derive(Parser)]
struct Transform {
    /// Input YAML path
    #[clap(short, long)]
    input: String,
    /// Output YAML path
    #[clap(short, long)]
    output: String,
    /// Transforms file path
    #[clap(short, long)]
    transform: String,
}

/// Generate a PAC directly from a SVD
#[derive(Parser)]
struct Generate {
    /// SVD file path
    #[clap(long)]
    svd: String,
    /// Transforms file path
    #[clap(long)]
    transform: Vec<String>,
}

/// Reformat a YAML
#[derive(Parser)]
struct Fmt {
    /// Peripheral file path
    files: Vec<String>,
    /// Error if incorrectly formatted, instead of fixing.
    #[clap(long)]
    check: bool,
    /// Remove unused enums
    #[clap(long)]
    remove_unused: bool,
}

/// Check a YAML for errors.
#[derive(Parser)]
struct Check {
    /// Peripheral file path
    files: Vec<String>,

    #[clap(long)]
    allow_register_overlap: bool,
    #[clap(long)]
    allow_field_overlap: bool,
    #[clap(long)]
    allow_enum_dup_value: bool,
    #[clap(long)]
    allow_unused_enums: bool,
    #[clap(long)]
    allow_unused_fieldsets: bool,
}

/// Generate Rust code from a YAML register block
#[derive(Parser)]
struct GenBlock {
    /// Input YAML path
    #[clap(short, long)]
    input: String,
    /// Output YAML path
    #[clap(short, long)]
    output: String,
}

fn main() -> Result<()> {
    env_logger::init();

    let opts: Opts = Opts::parse();

    match opts.subcommand {
        Subcommand::ExtractPeripheral(x) => extract_peripheral(x),
        Subcommand::ExtractAll(x) => extract_all(x),
        Subcommand::Generate(x) => gen(x),
        Subcommand::Transform(x) => transform(x),
        Subcommand::Fmt(x) => fmt(x),
        Subcommand::Check(x) => check(x),
        Subcommand::GenBlock(x) => gen_block(x),
    }
}

fn load_svd(path: &str) -> Result<svd_parser::svd::Device> {
    let xml = &mut String::new();
    File::open(path)
        .context("Cannot open the SVD file")?
        .read_to_string(xml)
        .context("Cannot read the SVD file")?;

    let config = svd_parser::Config::default()
        .expand_properties(true)
        .validate_level(ValidateLevel::Disabled);
    let device = svd_parser::parse_with_config(xml, &config)?;
    Ok(device)
}

fn load_config(path: &str) -> Result<Config> {
    let config = fs::read(path).context("Cannot read the config file")?;
    serde_yaml::from_slice(&config).context("cannot deserialize config")
}

fn extract_peripheral(args: ExtractPeripheral) -> Result<()> {
    let config = if args.transform.is_empty() {
        Config::default()
    } else {
        args.transform
            .into_iter()
            .map(|s| load_config(&s))
            .collect::<Result<Config>>()?
    };

    let svd = load_svd(&args.svd)?;
    let mut ir = IR::new();

    let peri = args.peripheral;
    let mut p = svd
        .peripherals
        .iter()
        .find(|p| p.name == peri)
        .expect("peripheral not found");

    if let Some(f) = &p.derived_from {
        p = svd
            .peripherals
            .iter()
            .find(|p| p.name == *f)
            .expect("derivedFrom peripheral not found");
    }

    chiptool::svd2ir::convert_peripheral(&mut ir, p)?;

    // Descriptions in SVD's contain a lot of noise and weird formatting. Clean them up.
    let description_cleanups = [
        // Fix weird newline spam in descriptions.
        (Regex::new("[ \n]+").unwrap(), " "),
        // Fix weird tab and cr spam in descriptions.
        (Regex::new("[\r\t]+").unwrap(), " "),
        // Replace double-space (end of sentence) with period.
        (
            Regex::new(r"(?<first_sentence>.*?)[\s]{2}(?<next_sentence>.*)").unwrap(),
            "$first_sentence. $next_sentence",
        ),
        // Make sure every description ends with a period.
        (
            Regex::new(r"(?<full_description>.*)(?<last_character>[\s'[^\.\s']])$").unwrap(),
            "$full_description$last_character.",
        ),
        // Eliminate space characters between end of description and the closing period.
        (
            Regex::new(r"(?<full_description>.*)\s\.$").unwrap(),
            "$full_description.",
        ),
    ];
    for (re, rep) in description_cleanups.iter() {
        chiptool::transform::map_descriptions(&mut ir, |d| re.replace_all(d, *rep).into_owned())?;
    }

    for t in &config.transforms {
        info!("running: {:?}", t);
        t.run(&mut ir)?;
    }

    // Ensure consistent sort order in the YAML.
    chiptool::transform::sort::Sort {}.run(&mut ir).unwrap();

    serde_yaml::to_writer(stdout(), &ir).unwrap();
    Ok(())
}

fn extract_all(args: ExtractAll) -> Result<()> {
    std::fs::create_dir_all(&args.output)?;

    let svd = load_svd(&args.svd)?;

    for p in &svd.peripherals {
        if p.derived_from.is_some() {
            continue;
        }

        let mut ir = IR::new();
        chiptool::svd2ir::convert_peripheral(&mut ir, p)?;

        // Fix weird newline spam in descriptions.
        let re = Regex::new("[ \n]+").unwrap();
        chiptool::transform::map_descriptions(&mut ir, |d| re.replace_all(d, " ").into_owned())?;

        // Ensure consistent sort order in the YAML.
        chiptool::transform::sort::Sort {}.run(&mut ir).unwrap();

        let f = File::create(PathBuf::from(&args.output).join(format!("{}.yaml", p.name)))?;
        serde_yaml::to_writer(f, &ir).unwrap();
    }

    Ok(())
}

fn gen(args: Generate) -> Result<()> {
    let config = if args.transform.is_empty() {
        Config::default()
    } else {
        args.transform
            .into_iter()
            .map(|s| load_config(&s))
            .collect::<Result<Config>>()?
    };
    let svd: svd_parser::svd::Device = load_svd(&args.svd)?;
    let items = gen_impl(&svd, &config.transforms).unwrap();
    fs::write("lib.rs", items.to_string())?;

    Ok(())
}

fn gen_impl(
    svd: &svd_parser::svd::Device,
    transform_list: &Vec<chiptool::transform::Transform>,
) -> Result<TokenStream> {
    let mut ir = svd2ir::convert_svd(svd)?;

    // Fix weird newline spam in descriptions.
    let re = Regex::new("[ \n]+").unwrap();
    chiptool::transform::map_descriptions(&mut ir, |d| re.replace_all(d, " ").into_owned())?;

    for t in transform_list {
        info!("running: {:?}", t);
        t.run(&mut ir)?;
    }

    let generate_opts = generate::Options {
        common_module: generate::CommonModule::Builtin,
    };
    generate::render(&ir, &generate_opts)
}

fn transform(args: Transform) -> Result<()> {
    let data = fs::read(&args.input)?;
    let mut ir: IR = serde_yaml::from_slice(&data)?;
    let config = load_config(&args.transform)?;
    for t in &config.transforms {
        info!("running: {:?}", t);
        t.run(&mut ir)?;
    }
    let data = serde_yaml::to_string(&ir)?;
    fs::write(&args.output, data.as_bytes())?;

    Ok(())
}

fn fmt(args: Fmt) -> Result<()> {
    for file in args.files {
        let got_data = fs::read(&file)?;
        let mut ir: IR = serde_yaml::from_slice(&got_data)?;

        if args.remove_unused {
            let mut used_enums = HashSet::new();
            for fs in ir.fieldsets.values_mut() {
                for f in fs.fields.iter_mut().filter(|f| f.enumm.is_some()) {
                    used_enums.insert(f.enumm.as_ref().unwrap().clone());
                }
            }

            ir.enums.retain(|name, _| used_enums.contains(name));
        }

        // Ensure consistent sort order in the YAML.
        chiptool::transform::sort::Sort {}.run(&mut ir).unwrap();

        // Trim all descriptions

        let cleanup = |s: &mut Option<String>| {
            if let Some(s) = s.as_mut() {
                *s = s.trim().to_string()
            }
        };

        for b in ir.blocks.values_mut() {
            cleanup(&mut b.description);
            for i in b.items.iter_mut() {
                cleanup(&mut i.description);
            }
        }

        for b in ir.fieldsets.values_mut() {
            cleanup(&mut b.description);
            for i in b.fields.iter_mut() {
                cleanup(&mut i.description);
            }
        }

        for b in ir.enums.values_mut() {
            cleanup(&mut b.description);
            for i in b.variants.iter_mut() {
                cleanup(&mut i.description);
            }
        }

        let want_data = serde_yaml::to_string(&ir)?;

        if got_data != want_data.as_bytes() {
            if args.check {
                bail!("File {} is not correctly formatted", &file);
            } else {
                fs::write(&file, want_data)?;
            }
        }
    }
    Ok(())
}

fn check(args: Check) -> Result<()> {
    let opts = chiptool::validate::Options {
        allow_enum_dup_value: args.allow_enum_dup_value,
        allow_field_overlap: args.allow_field_overlap,
        allow_register_overlap: args.allow_register_overlap,
        allow_unused_enums: args.allow_unused_enums,
        allow_unused_fieldsets: args.allow_unused_fieldsets,
    };

    let mut fails = 0;

    for file in args.files {
        let got_data = fs::read(&file)?;
        let ir: IR = serde_yaml::from_slice(&got_data)?;
        let errs = chiptool::validate::validate(&ir, opts.clone());
        fails += errs.len();
        for e in errs {
            println!("{}: {}", &file, e);
        }
    }

    if fails != 0 {
        bail!("{} failures", fails)
    }

    Ok(())
}

fn gen_block(args: GenBlock) -> Result<()> {
    let data = fs::read(&args.input)?;
    let mut ir: IR = serde_yaml::from_slice(&data)?;

    chiptool::transform::Sanitize {}.run(&mut ir).unwrap();

    // Ensure consistent sort order in the YAML.
    chiptool::transform::sort::Sort {}.run(&mut ir).unwrap();

    let generate_opts = generate::Options {
        common_module: generate::CommonModule::Builtin,
    };
    let items = generate::render(&ir, &generate_opts).unwrap();
    fs::write(&args.output, items.to_string())?;

    Ok(())
}
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Config {
    transforms: Vec<chiptool::transform::Transform>,
}

impl FromIterator<Config> for Config {
    fn from_iter<I: IntoIterator<Item = Config>>(iter: I) -> Self {
        let transforms: Vec<_> = iter.into_iter().flat_map(|c| c.transforms).collect();
        Self { transforms }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_generation_should_generate_stable_results() {
        let mut fails = 0;
        let expected_code = fs::read_to_string("src/test/gen_lib.rs")
            .expect("Should be able to read the expected result");

        let config: Config =
            load_config("src/test/test.yaml").expect("Should be able to load transform");

        let svd: svd_parser::svd::Device =
            load_svd("src/test/test.svd").expect("Should be able to load svd");

        for _ in 0..20 {
            let result = gen_impl(&svd, &config.transforms);
            let generated_code = result.expect("Unable to parse generate code").to_string();

            if expected_code != generated_code {
                fails += 1;
                println!("!!!!!!!!!!!!!!!!!!!!!!!! FAILED");
            }
            println!("");
            println!("");
            println!("");
        }
        println!("Failures {fails}");
        assert_eq!(0, fails);
    }
}
