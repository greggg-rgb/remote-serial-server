use anyhow::Result;
use clap::{Parser, ValueEnum};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_serial::{SerialPortBuilderExt, DataBits, FlowControl, Parity, StopBits};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(long)]
    serial_port: String,

    #[arg(long, default_value_t = 115200)]
    baud_rate: u32,

    #[arg(long, default_value_t = 8)]
    data_bits: u8,

    #[arg(long, value_enum, default_value_t = ParityArg::None)]
    parity: ParityArg,

    #[arg(long, value_enum, default_value_t = StopBitsArg::One)]
    stop_bits: StopBitsArg,

    #[arg(long, default_value_t = 11223)]
    tcp_port: u16,
}

#[derive(Copy, Clone, ValueEnum, Debug, Default)]
enum ParityArg {
    Even,
    Odd,
    #[default]
    None,
}
impl From<ParityArg> for Parity {
    fn from(val: ParityArg) -> Self {
        match val {
            ParityArg::Even => Parity::Even,
            ParityArg::Odd => Parity::Odd,
            ParityArg::None => Parity::None,
        }
    }
}

#[derive(Copy, Clone, ValueEnum, Debug, Default)]
enum StopBitsArg {
    #[default]
    One,
    Two,
}
impl From<StopBitsArg> for StopBits {
    fn from(val: StopBitsArg) -> Self {
        match val {
            StopBitsArg::One => StopBits::One,
            StopBitsArg::Two => StopBits::Two,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let data_bits = match args.data_bits {
    5 => DataBits::Five,
    6 => DataBits::Six,
    7 => DataBits::Seven,
    8 => DataBits::Eight,
    _ => {
        eprintln!("Unsupported data bits: {}. Using 8 as default.", args.data_bits);
        DataBits::Eight
        }
    };

    let mut serial = tokio_serial::new(&args.serial_port, args.baud_rate)
        .data_bits(data_bits)
        .parity(args.parity.into())
        .stop_bits(args.stop_bits.into())
        .flow_control(FlowControl::None)
        .open_native_async()?;

    let listener = TcpListener::bind(("0.0.0.0", args.tcp_port)).await?;
    println!("Listening on port {}", args.tcp_port);

    let (mut socket, addr) = listener.accept().await?;
    println!("Client connected: {}", addr);

    let mut serial_buf = [0u8; 1024];
    let mut socket_buf = [0u8; 1024];

    loop {
        tokio::select! {
            read_serial = serial.read(&mut serial_buf) => {
                match read_serial {
                    Ok(n) if n > 0 => {
                        if socket.write_all(&serial_buf[..n]).await.is_err() {
                            println!("Client write failed");
                            continue;
                        }
                    },
                    Ok(_) => {},
                    Err(e) => {
                        eprintln!("Serial read error: {}", e);
                        continue;
                    }
                }
            },
            read_socket = socket.read(&mut socket_buf) => {
                match read_socket {
                    Ok(n) if n > 0 => {
                        if serial.write_all(&socket_buf[..n]).await.is_err() {
                            println!("Serial write failed");
                            continue;
                        }
                    },
                    Ok(_) => {},
                    Err(e) => {
                        eprintln!("Socket read error: {}", e);
                        continue;
                    }
                }
            }
        }
    }
}