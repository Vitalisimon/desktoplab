use desktoplab_smoke_cli::{InProcessSmokeApi, SmokeCli, SmokeCommand};

fn main() {
    let mut cli = SmokeCli::new(InProcessSmokeApi::default());
    println!("{}", cli.run(SmokeCommand::DiagnosticsExport).body());
}
