use anyhow::Result;
use clap::{Parser, ValueEnum};
use clap_repl::reedline::{DefaultPrompt, DefaultPromptSegment, FileBackedHistory};
use clap_repl::{ClapEditor, ReadCommandOutput};
use console::style;
use tracing::info;
use usb_io::{I2cMode, USBIODriver};
use wire_weaver_usb_host::wire_weaver_client_server::OnError;
use wire_weaver_usb_host::{UsbDeviceFilter, UsbError};

#[derive(Parser)]
#[command(name = "")] // This name will show up in clap's error messages, so it is important to set it to "".
enum Command {
    Connect,
    Disconnect,

    I2cInit,

    Exit,
    Quit,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let mut driver = None;

    let prompt = DefaultPrompt {
        left_prompt: DefaultPromptSegment::Basic("vhrd-io".to_owned()),
        ..DefaultPrompt::default()
    };
    let mut rl = ClapEditor::<Command>::builder()
        .with_prompt(Box::new(prompt))
        .with_editor_hook(|reed| {
            // Do custom things with `Reedline` instance here
            reed.with_history(Box::new(
                FileBackedHistory::with_file(10000, "repl_history.txt".into()).unwrap(),
            ))
        })
        .build();
    loop {
        match rl.read_command() {
            ReadCommandOutput::Command(c) => match c {
                Command::Connect => {
                    let filter = UsbDeviceFilter::AnyVhrdTechIo;
                    let b193 = USBIODriver::connect(filter, OnError::ExitImmediately, 512).await?;
                    info!("Connected!");
                    driver = Some(b193);
                }
                Command::Disconnect => {
                    if let Some(mut d) = driver.take() {
                        d.disconnect_and_exit().await?;
                        info!("Disconnected!");
                    } else {
                        info!("Already disconnected");
                    }
                }
                Command::Exit | Command::Quit => break,
                c => {
                    let Some(d) = driver.as_mut() else {
                        println!("{}", style("No connection, connect first").yellow());
                        continue;
                    };
                    match handle_command_inner(c, d).await {
                        Ok(_) => println!("{}", style("ok").green()),
                        Err(e) => println!("{}", style(e).red()),
                    }
                }
            },
            ReadCommandOutput::EmptyLine => (),
            ReadCommandOutput::ClapError(e) => {
                e.print()?;
            }
            ReadCommandOutput::ShlexError => {
                println!(
                    "{} input was not valid and could not be processed",
                    style("Error:").red().bold()
                );
            }
            ReadCommandOutput::ReedlineError(e) => {
                panic!("{e}");
            }
            ReadCommandOutput::CtrlC => continue,
            ReadCommandOutput::CtrlD => break,
        }
    }

    if let Some(d) = driver.as_mut() {
        d.disconnect_and_exit().await?;
    }
    Ok(())
}

async fn handle_command_inner(
    c: Command,
    d: &mut USBIODriver,
) -> Result<(), wire_weaver_usb_host::wire_weaver_client_server::Error<UsbError>> {
    match c {
        Command::I2cInit => {
            let r = d
                .i2c_configure(I2cMode::Master { speed_hz: 400_000 })
                .await?;
            println!("i2c_configure: {r:?}");
            Ok(())
        }
        _ => unreachable!(),
    }
}
