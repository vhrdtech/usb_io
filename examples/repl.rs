use anyhow::Result;
use clap::{Parser, Subcommand};
use clap_repl::reedline::{DefaultPrompt, DefaultPromptSegment, FileBackedHistory};
use clap_repl::{ClapEditor, ReadCommandOutput};
use console::style;
use std::time::Duration;
use tracing::info;
use usb_io::{I2cMode, I2cReadKind, USBIODriver};
use wire_weaver_usb_host::UsbDeviceFilter;
use wire_weaver_usb_host::wire_weaver_client_server::OnError;

#[derive(Parser)]
#[command(name = "")] // This name will show up in clap's error messages, so it is important to set it to "".
enum Command {
    Connect,
    Disconnect,

    #[clap(subcommand)]
    I2c(I2cCommand),

    Exit,
    Quit,
}

#[derive(Debug, Clone, Subcommand)]
enum I2cCommand {
    Capabilities,
    Configure,
    Scan,
    Read {
        addr: String,
        len: u16,
    },
    Write {
        addr: String,
        data: String,
    },
    WriteRead {
        addr: String,
        write_data: String,
        read_len: u16,
    },
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
                    let b193 =
                        match USBIODriver::connect(filter, OnError::ExitImmediately, 512).await {
                            Ok(driver) => driver,
                            Err(e) => {
                                println!("{:?}", style(e).red());
                                continue;
                            }
                        };
                    info!("Connected!");
                    driver = Some(b193);
                }
                Command::Disconnect => {
                    if let Some(mut d) = driver.take() {
                        let r = d.disconnect_and_exit().await;
                        info!("Disconnect: {r:?}");
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
                        // Ok(_) => println!("{}", style("ok").green()),
                        Ok(_) => {}
                        Err(e) => println!("{}", style(e).red()),
                    }
                }
            },
            ReadCommandOutput::EmptyLine => {
                // TODO: check if connection is broken and drop device
            }
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
        if let Err(e) = d.disconnect_and_exit().await {
            println!("{:?}", style(e).red());
        }
    }
    Ok(())
}

async fn handle_command_inner(c: Command, d: &mut USBIODriver) -> Result<()> {
    match c {
        Command::I2c(command) => match command {
            I2cCommand::Configure => {
                let r = d
                    .i2c_configure(
                        None,
                        I2cMode::Master {
                            speed_hz: 400_000,
                            monitor: false,
                        },
                        3300,
                    )
                    .await?;
                println!("i2c configure: {r:?}");
            }
            I2cCommand::Scan => {
                let r = d.i2c_scan(Some(Duration::from_millis(3000))).await?;
                println!("i2c scan: {r:02x?}");
            }
            I2cCommand::Capabilities => {
                let capabilities = d.i2c_capabilities(None).await?;
                println!("{capabilities:?}");
            }
            I2cCommand::Read { addr, len } => {
                let addr = u8::from_str_radix(addr.as_str(), 16)?;
                let r = d.i2c_read(None, addr, I2cReadKind::Plain, len).await?;
                match r {
                    Ok(envelope) => {
                        println!("i2c read 0x{:02x}: {:02x?}", addr, envelope.data.as_slice())
                    }
                    Err(e) => println!("i2c read 0x{:02x}: {:?}", addr, style(e).red()),
                }
            }
            I2cCommand::Write { addr, data } => {
                let addr = u8::from_str_radix(addr.as_str(), 16)?;
                let data = hex::decode(data)?;
                let r = d.i2c_write(None, addr, data.clone()).await?;
                match r {
                    Ok(_) => println!("i2c write 0x{:02x}: {:02x?} ok", addr, data),
                    Err(e) => println!("i2c write 0x{:02x}: {:?}", addr, style(e).red()),
                }
            }
            I2cCommand::WriteRead {
                addr,
                write_data,
                read_len,
            } => {
                let addr = u8::from_str_radix(addr.as_str(), 16)?;
                let write_data = hex::decode(write_data)?;
                let r = d
                    .i2c_read(
                        None,
                        addr,
                        I2cReadKind::RepeatedStart {
                            write: write_data.clone(),
                        },
                        read_len,
                    )
                    .await?;
                match r {
                    Ok(envelope) => {
                        println!(
                            "i2c write-read 0x{:02x} {:02x?}: {:02x?} ok",
                            addr,
                            write_data,
                            envelope.data.as_slice()
                        )
                    }
                    Err(e) => println!("i2c read 0x{:02x}: {:?}", addr, style(e).red()),
                }
            }
        },
        _ => unreachable!(),
    }
    Ok(())
}
