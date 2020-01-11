use crate::spec::{LinkArgs, LinkerFlavor, TargetOptions};
use std::env;
use std::io;
use std::path::Path;
use std::process::Command;

use Arch::*;

#[allow(non_camel_case_types)]
#[derive(Copy, Clone)]
pub enum Arch {
    Arm64,
    X86_64,
}

impl Arch {
    pub fn to_string(self) -> &'static str {
        match self {
            Arm64 => "arm64",
            X86_64 => "x86_64",
        }
    }
}

pub fn get_sdk_root(sdk_name: &str) -> Result<String, String> {
    // Following what clang does
    // (https://github.com/llvm/llvm-project/blob/
    // 296a80102a9b72c3eda80558fb78a3ed8849b341/clang/lib/Driver/ToolChains/Darwin.cpp#L1661-L1678)
    // to allow the SDK path to be set. (For clang, xcrun sets
    // SDKROOT; for rustc, the user or build system can set it, or we
    // can fall back to checking for xcrun on PATH.)
    if let Some(sdkroot) = env::var("SDKROOT").ok() {
        let p = Path::new(&sdkroot);
        match sdk_name {
            // Ignore `SDKROOT` if it's clearly set for the wrong platform.
            "appletvos"
                if sdkroot.contains("TVSimulator.platform")
                    || sdkroot.contains("MacOSX.platform") =>
            {
                ()
            }
            "appletvsimulator"
                if sdkroot.contains("TVOS.platform") || sdkroot.contains("MacOSX.platform") =>
            {
                ()
            }
            // Ignore `SDKROOT` if it's not a valid path.
            _ if !p.is_absolute() || p == Path::new("/") || !p.exists() => (),
            _ => return Ok(sdkroot),
        }
    }
    let res =
        Command::new("xcrun").arg("--show-sdk-path").arg("-sdk").arg(sdk_name).output().and_then(
            |output| {
                if output.status.success() {
                    Ok(String::from_utf8(output.stdout).unwrap())
                } else {
                    let error = String::from_utf8(output.stderr);
                    let error = format!("process exit with error: {}", error.unwrap());
                    Err(io::Error::new(io::ErrorKind::Other, &error[..]))
                }
            },
        );

    match res {
        Ok(output) => Ok(output.trim().to_string()),
        Err(e) => Err(format!("failed to get {} SDK path: {}", sdk_name, e)),
    }
}

fn build_pre_link_args(arch: Arch) -> Result<LinkArgs, String> {
    let sdk_name = match arch {
        Arm64 => "appletvos",
        X86_64 => "appletvsimulator",
    };

    let arch_name = arch.to_string();

    let sdk_root = get_sdk_root(sdk_name)?;

    let mut args = LinkArgs::new();
    args.insert(
        LinkerFlavor::Gcc,
        vec![
            "-arch".to_string(),
            arch_name.to_string(),
            "-isysroot".to_string(),
            sdk_root.clone(),
            "-Wl,-syslibroot".to_string(),
            sdk_root,
        ],
    );

    Ok(args)
}

fn target_cpu(arch: Arch) -> String {
    match arch {
        Arm64 => "cyclone",
        X86_64 => "core2",
    }
    .to_string()
}

fn link_env_remove(arch: Arch) -> Vec<String> {
    match arch {
        Arm64 | X86_64 => vec!["MACOSX_DEPLOYMENT_TARGET".to_string()],
    }
}

pub fn opts(arch: Arch) -> Result<TargetOptions, String> {
    let pre_link_args = build_pre_link_args(arch)?;
    Ok(TargetOptions {
        cpu: target_cpu(arch),
        dynamic_linking: false,
        executables: true,
        pre_link_args,
        link_env_remove: link_env_remove(arch),
        has_elf_tls: false,
        eliminate_frame_pointer: false,
        ..super::apple_base::opts()
    })
}
