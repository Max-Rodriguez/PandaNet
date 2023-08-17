// DONET SOFTWARE
// Copyright (c) 2023, DoNet Authors.

// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License version 3.
// You should have received a copy of this license along
// with this source code in a file named "LICENSE."
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program; if not, write to the Free Software Foundation,
// Inc., 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.

#[path = "config.rs"]
mod config;
#[path = "logger.rs"]
mod logger;
#[path = "service_factory.rs"]
mod service_factory;

fn main() -> std::io::Result<()> {
    use self::logger::logger;
    use config::config::DonetConfig;
    use log::SetLoggerError;
    use std::fs::File;
    use std::io::Read;

    const VERSION_STRING: &str = "0.1.0";
    static GIT_SHA1: &str = env!("GIT_SHA1");
    let mut config_file: &str = "daemon.toml"; // default
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        let mut index: usize = 0;
        for argument in &args {
            if index == 0 {
                index += 1; // skip binary name
                continue;
            }
            if argument.starts_with("-") {
                if argument == "-h" || argument == "--help" {
                    print_help_page(config_file);
                    return Ok(());
                } else if argument == "-v" || argument == "--version" {
                    print_version(VERSION_STRING, GIT_SHA1);
                    return Ok(());
                } else {
                    println!("donet: {}: Invalid argument.\n", argument);
                    print_help_page(config_file);
                    return Ok(());
                }
            } else if index == (args.len() - 1) {
                // last argument given & doesn't match any of the above,
                // so it must be the configuration file path given.
                config_file = argument.as_str();
                break;
            }
        }
    }
    // Initialize the logger utility
    let res: Result<(), SetLoggerError> = logger::initialize_logger();
    if res.is_err() {
        panic!("Failed to initialize the logger utility!");
    }

    // Read the daemon configuration file
    let mut conf_file = File::open(config_file)?;
    let mut contents: String = String::new();
    conf_file.read_to_string(&mut contents)?;

    let daemon_config: DonetConfig = toml::from_str(contents.as_str()).unwrap();
    return Ok(());
}

fn print_help_page(config_path: &str) -> () {
    println!(
        "Usage:    donet [options] ... [CONFIG_FILE]\n\
        \n\
        DoNet - Distributed Object Network Engine.\n\
        This binary will look for a configuration file (.toml)\n\
        in the current working directory as \"{}\".\n\
        \n\
        -h, --help      Print the help page.\n\
        -v, --version   Print DoNet binary version & build info.\n",
        config_path
    );
}

fn print_version(version_string: &str, git_sha1: &str) -> () {
    let bin_arch: &str = if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "unknown" // aka not supported
    };
    let bin_platform: &str = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "freebsd") {
        "freebsd"
    } else {
        "unknown" // aka not supported
    };
    let bin_env: &str = if cfg!(target_env = "gnu") {
        "gnu"
    } else if cfg!(target_env = "msvc") {
        "msvc"
    } else {
        "other"
    };
    println!(
        "DoNet, version {} ({} {}-{})\n\
        Revision (Git SHA1): {}\n\n\
        Released under the AGPL-3.0 license. <https://www.gnu.org/licenses/agpl-3.0.html>\n\
        View the source code on GitHub: https://github.com/donet-server/DoNet\n",
        version_string, bin_arch, bin_platform, bin_env, git_sha1
    );
}
