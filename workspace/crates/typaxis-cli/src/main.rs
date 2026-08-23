#![forbid(unsafe_code)]

const EXIT_USAGE: i32 = 2;
const EXIT_INTERNAL_INVARIANT: i32 = 4;
const KNOWN_SKELETON_COMMANDS: &[&str] = &[
    "build",
    "check",
    "dump-ast",
    "dump-layout",
    "inspect-font",
    "list-fonts",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Dispatch<'a> {
    Help,
    Version,
    KnownSkeleton(&'a str),
    Unknown(&'a str),
}

fn classify(command: Option<&str>) -> Dispatch<'_> {
    match command {
        Some("--version") | Some("version") => Dispatch::Version,
        Some("--help") | Some("help") | None => Dispatch::Help,
        Some(command) if KNOWN_SKELETON_COMMANDS.contains(&command) => {
            Dispatch::KnownSkeleton(command)
        }
        Some(command) => Dispatch::Unknown(command),
    }
}

fn dispatch_exit_code(dispatch: Dispatch<'_>) -> i32 {
    match dispatch {
        Dispatch::Help | Dispatch::Version => 0,
        Dispatch::KnownSkeleton(_) => EXIT_INTERNAL_INVARIANT,
        Dispatch::Unknown(_) => EXIT_USAGE,
    }
}

fn main() {
    let mut args = std::env::args();
    let program = args.next().unwrap_or_else(|| "typaxis".to_owned());
    let command = args.next();
    let dispatch = classify(command.as_deref());
    match dispatch {
        Dispatch::Version => println!("typaxis {}", env!("CARGO_PKG_VERSION")),
        Dispatch::Help => print_help(&program),
        Dispatch::KnownSkeleton(command) => {
            eprintln!("command `{command}` is part of the Typaxis contract but is not implemented in the reference skeleton");
        }
        Dispatch::Unknown(command) => {
            eprintln!("unknown command `{command}`");
            eprintln!("run `{program} --help` for usage");
        }
    }
    let exit_code = dispatch_exit_code(dispatch);
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}

fn print_help(program: &str) {
    println!("{program} <build|check|dump-ast|dump-layout|inspect-font|list-fonts> [options]");
    println!("Typaxis reference contract skeleton; target commands are not implemented.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_command_is_usage_error() {
        let dispatch = classify(Some("unknown"));
        assert_eq!(dispatch, Dispatch::Unknown("unknown"));
        assert_eq!(dispatch_exit_code(dispatch), EXIT_USAGE);
    }

    #[test]
    fn known_contract_command_is_skeleton_internal_error() {
        for command in KNOWN_SKELETON_COMMANDS {
            let dispatch = classify(Some(*command));
            assert_eq!(dispatch, Dispatch::KnownSkeleton(command));
            assert_eq!(dispatch_exit_code(dispatch), EXIT_INTERNAL_INVARIANT);
        }
    }

    #[test]
    fn help_and_version_succeed() {
        assert_eq!(dispatch_exit_code(classify(None)), 0);
        assert_eq!(dispatch_exit_code(classify(Some("--help"))), 0);
        assert_eq!(dispatch_exit_code(classify(Some("--version"))), 0);
    }
}
